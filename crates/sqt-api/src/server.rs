//! HTTP/gRPC server wiring.

use std::net::SocketAddr;
use std::sync::Arc;

use axum::middleware;
use axum::Router;
use sqt_audit::AuditStorage;
use tonic::transport::Server;
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

/// Starts the HTTP and gRPC servers.
pub async fn serve<S: AuditStorage + 'static>(
    state: Arc<AppState<S>>,
    config: ServerConfig,
    http_port: u16,
    grpc_port: u16,
) -> anyhow::Result<()> {
    let http_state = state.clone();
    let http_task = tokio::spawn(async move {
        let mut app = Router::new()
            .merge(rest::router(http_state.clone()))
            .merge(a2a::router(http_state.clone()))
            .merge(mcp::router(http_state))
            .layer(TraceLayer::new_for_http());

        if config.auth_enabled {
            app = app.layer(middleware::from_fn_with_state(
                AuthConfig {
                    api_key: config.api_key.clone(),
                    enabled: config.auth_enabled,
                },
                api_key_middleware,
            ));
        }

        let addr: SocketAddr = ([0, 0, 0, 0], http_port).into();
        info!("HTTP server listening on {addr}");
        let listener = tokio::net::TcpListener::bind(addr).await?;
        axum::serve(listener, app).await?;
        Ok::<(), anyhow::Error>(())
    });

    let grpc_task = tokio::spawn(async move {
        let agent = AgentService::new(state.clone());
        let health = HealthService;

        let addr: SocketAddr = ([0, 0, 0, 0], grpc_port).into();
        info!("gRPC server listening on {addr}");
        Server::builder()
            .add_service(HealthServer::new(health))
            .add_service(AgentServer::new(agent))
            .serve(addr)
            .await?;
        Ok::<(), anyhow::Error>(())
    });

    tokio::select! {
        result = http_task => result??,
        result = grpc_task => result??,
    }

    Ok(())
}
