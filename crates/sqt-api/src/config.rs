//! Server configuration read from environment variables.
//!
//! All secrets and operational toggles are externalised so the same binary can
//! run in production, development, and tests without recompilation.

/// Configuration for the HTTP/gRPC server.
#[derive(Debug, Clone)]
pub struct ServerConfig {
    /// API key required by the `x-api-key` header when auth is enabled.
    pub api_key: Option<String>,
    /// Whether API-key authentication is enforced.
    pub auth_enabled: bool,
}

impl ServerConfig {
    /// Loads configuration from the process environment.
    ///
    /// Recognised variables:
    ///
    /// * `SQT_API_KEY` – the shared API key.
    /// * `SQT_AUTH_ENABLED` – `true`/`1`/`yes` to require the API key (default `false`).
    pub fn from_env() -> Self {
        Self {
            api_key: std::env::var("SQT_API_KEY").ok(),
            auth_enabled: parse_bool_env("SQT_AUTH_ENABLED").unwrap_or(false),
        }
    }
}

fn parse_bool_env(name: &str) -> Option<bool> {
    std::env::var(name)
        .ok()
        .map(|v| matches!(v.to_lowercase().as_str(), "1" | "true" | "yes" | "on"))
}
