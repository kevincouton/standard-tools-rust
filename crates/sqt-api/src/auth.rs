//! API-key authentication middleware.
//!
//! The middleware permits unauthenticated access to `/health` so that load
//! balancers and container health checks can operate without a key. All other
//! requests must carry the `x-api-key` header matching `SQT_API_KEY`.

use axum::{
    body::Body,
    extract::State,
    http::{Request, StatusCode},
    middleware::Next,
    response::{IntoResponse, Response},
    Json,
};
use serde_json::json;
use tracing::warn;

/// Configuration carried by the auth middleware.
#[derive(Debug, Clone)]
pub struct AuthConfig {
    /// API key required by the `x-api-key` header when auth is enabled.
    pub api_key: Option<String>,
    /// Whether API-key authentication is enforced.
    pub enabled: bool,
}

/// Axum middleware function that validates the `x-api-key` header.
///
/// When `enabled` is `false` the middleware is a no-op. The `/health` endpoint
/// is always exempt so orchestrators can probe liveness.
pub async fn api_key_middleware(
    State(config): State<AuthConfig>,
    req: Request<Body>,
    next: Next,
) -> Response {
    if !config.enabled || req.uri().path() == "/health" {
        return next.run(req).await;
    }

    let provided = req
        .headers()
        .get("x-api-key")
        .and_then(|v| v.to_str().ok());

    match (config.api_key.as_ref(), provided) {
        (Some(expected), Some(got)) if constant_time_eq(expected, got) => next.run(req).await,
        _ => {
            warn!(path = %req.uri().path(), "request rejected: missing or invalid api key");
            (
                StatusCode::UNAUTHORIZED,
                Json(json!({ "error": "unauthorized" })),
            )
                .into_response()
        }
    }
}

fn constant_time_eq(a: &str, b: &str) -> bool {
    use subtle::ConstantTimeEq;
    a.as_bytes().ct_eq(b.as_bytes()).into()
}
