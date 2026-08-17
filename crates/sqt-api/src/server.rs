//! HTTP/gRPC server wiring.

use std::net::SocketAddr;
use std::sync::Arc;
use std::time::Duration;

use axum::middleware;
use axum::Router;
use sqt_audit::AuditStorage;
use tonic::transport::Server;
use tower_http::limit::RequestBodyLimitLayer;
use tower_http::timeout::TimeoutLayer;
use tower_http::trace::TraceLayer;
use tracing::info;

use crate::a2a;
use crate::auth::{api_key_middleware, AuthConfig};
use crate::config::ServerConfig;
use crate::grpc::proto::agent::agent_service_server::AgentServiceServer as AgentServer;
use crate::grpc::proto::health::health_service_server::HealthServiceServer as HealthServer;
use crate::grpc::{AgentService, HealthService};
use crate::mcp;
use crate::rest;
use crate::state::AppState;

const REQUEST_BODY_LIMIT: usize = 16 * 1024 * 1024; // 16 MiB
const REQUEST_TIMEOUT: Duration = Duration::from_secs(60);

/// Starts the HTTP and gRPC servers.
pub async fn serve<S: AuditStorage + 'static>(
    state: Arc<AppState<S>>,
    config: ServerConfig,
    http_port: u16,
    grpc_port: u16,
) -> anyhow::Result<()> {
    let http_state = state.clone();
    let http_config = config.clone();
    let http_task = tokio::spawn(async move {
        let mut app = Router::new()
            .merge(rest::router(http_state.clone()))
            .merge(a2a::router(http_state.clone()))
            .merge(mcp::router(http_state))
            .layer(TraceLayer::new_for_http())
            .layer(RequestBodyLimitLayer::new(REQUEST_BODY_LIMIT))
            .layer(TimeoutLayer::with_status_code(
                axum::http::StatusCode::REQUEST_TIMEOUT,
                REQUEST_TIMEOUT,
            ));

        if http_config.auth_enabled {
            app = app.layer(middleware::from_fn_with_state(
                AuthConfig {
                    api_key: http_config.api_key.clone(),
                    enabled: http_config.auth_enabled,
                },
                api_key_middleware,
            ));
        }

        let addr: SocketAddr = ([0, 0, 0, 0], http_port).into();
        info!("HTTP server listening on {addr}");
        let listener = tokio::net::TcpListener::bind(addr).await?;
        axum::serve(listener, app)
            .with_graceful_shutdown(shutdown_signal())
            .await?;
        Ok::<(), anyhow::Error>(())
    });

    let grpc_task = tokio::spawn(async move {
        let agent = AgentService::new(state.clone());
        let health = HealthService;

        let addr: SocketAddr = ([0, 0, 0, 0], grpc_port).into();
        info!("gRPC server listening on {addr}");

        let mut builder = Server::builder().timeout(REQUEST_TIMEOUT);
        if config.auth_enabled {
            let interceptor = crate::auth::grpc_api_key_interceptor(AuthConfig {
                api_key: config.api_key.clone(),
                enabled: config.auth_enabled,
            });
            builder
                .add_service(HealthServer::with_interceptor(health, interceptor.clone()))
                .add_service(AgentServer::with_interceptor(agent, interceptor))
                .serve_with_shutdown(addr, shutdown_signal())
                .await?;
        } else {
            builder
                .add_service(HealthServer::new(health))
                .add_service(AgentServer::new(agent))
                .serve_with_shutdown(addr, shutdown_signal())
                .await?;
        }
        Ok::<(), anyhow::Error>(())
    });

    tokio::select! {
        result = http_task => result??,
        result = grpc_task => result??,
    }

    Ok(())
}

/// Resolves when SIGINT or SIGTERM is received.
async fn shutdown_signal() {
    let ctrl_c = async {
        let _ = tokio::signal::ctrl_c().await;
    };

    #[cfg(unix)]
    let terminate = async {
        if let Ok(mut signal) =
            tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate())
        {
            signal.recv().await;
        } else {
            std::future::pending::<()>().await;
        }
    };

    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();

    tokio::select! {
        _ = ctrl_c => {},
        _ = terminate => {},
    }
    info!("shutdown signal received, draining in-flight requests");
}
