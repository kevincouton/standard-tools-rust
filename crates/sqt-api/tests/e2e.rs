//! End-to-end tests for the HTTP API surface.
//!
//! These tests spin up the full Axum router with an in-memory market-data
//! provider so that no external network calls are required. They exercise the
//! REST agent endpoints as well as the A2A and MCP JSON-RPC style endpoints,
//! and verify that the audit trail records tool executions.

use std::sync::Arc;

use axum::body::Body;
use axum::http::{Request, StatusCode};
use http_body_util::BodyExt;
use rust_decimal::prelude::FromPrimitive;
use serde_json::json;
use sqt_audit::{AuditWriter, InMemoryStorage};
use sqt_core::{BarInterval, DateRange, Ohlcv, Ticker};
use sqt_marketdata::{MarketDataService, MokaMarketDataCache};
use tower::ServiceExt;

use axum::Router;
use sqt_api::services::build_dispatcher;
use sqt_api::state::AppState;

/// Deterministic market-data provider that returns a synthetic upward-trending
/// series for any ticker.
struct SyntheticProvider;

#[async_trait::async_trait]
impl sqt_marketdata::MarketDataProvider for SyntheticProvider {
    fn name(&self) -> &'static str {
        "synthetic"
    }

    async fn fetch(
        &self,
        ticker: &Ticker,
        range: DateRange,
        _interval: BarInterval,
    ) -> sqt_core::Result<Vec<Ohlcv>> {
        let mut bars = Vec::new();
        let mut date = range.start;
        let mut price = rust_decimal::Decimal::from(100);
        while date <= range.end {
            let open = price;
            let close = price + rust_decimal::Decimal::from(1);
            let high = open.max(close) + rust_decimal::Decimal::from_f64(0.5).unwrap();
            let low = open.min(close) - rust_decimal::Decimal::from_f64(0.5).unwrap();
            bars.push(Ohlcv {
                ticker: ticker.clone(),
                date,
                open,
                high,
                low,
                close,
                volume: 1_000_000,
            });
            price = close;
            date = date.succ_opt().unwrap_or(date);
        }
        Ok(bars)
    }
}

fn test_state() -> Arc<AppState<InMemoryStorage>> {
    let cache: Arc<dyn sqt_marketdata::MarketDataCache> = MokaMarketDataCache::new().into();
    let market_data = Arc::new(MarketDataService::new("synthetic", cache));
    market_data.register_provider(Arc::new(SyntheticProvider));

    let dispatcher = Arc::new(build_dispatcher(market_data.clone()));
    let audit_storage = Arc::new(InMemoryStorage::new());
    let audit_writer = Arc::new(AuditWriter::new(audit_storage));

    Arc::new(AppState {
        dispatcher,
        audit_writer,
        market_data,
    })
}

fn app(state: Arc<AppState<InMemoryStorage>>) -> Router {
    Router::new()
        .merge(sqt_api::rest::router(state.clone()))
        .merge(sqt_api::a2a::router(state.clone()))
        .merge(sqt_api::mcp::router(state))
}

async fn send_json(
    state: Arc<AppState<InMemoryStorage>>,
    method: &str,
    uri: &str,
    body: serde_json::Value,
) -> (StatusCode, serde_json::Value) {
    let request = if method == "GET" {
        Request::builder()
            .method(method)
            .uri(uri)
            .body(Body::empty())
            .unwrap()
    } else {
        Request::builder()
            .method(method)
            .uri(uri)
            .header("content-type", "application/json")
            .body(Body::from(body.to_string()))
            .unwrap()
    };
    let response = app(state).oneshot(request).await.unwrap();
    let status = response.status();
    let bytes = response.into_body().collect().await.unwrap().to_bytes();
    let value = serde_json::from_slice(&bytes).unwrap_or(serde_json::Value::Null);
    (status, value)
}

#[tokio::test]
async fn list_agent_tools() {
    let state = test_state();
    let (status, body) = send_json(state, "GET", "/api/v1/agent/tools", json!({})).await;
    assert_eq!(status, StatusCode::OK);
    let tools = body.as_array().expect("tools array");
    assert!(!tools.is_empty());
    assert!(tools.iter().any(|t| t["name"] == "screen_with_indicators"));
}

#[tokio::test]
async fn dispatch_echo_tool_is_audited() {
    let state = test_state();
    let (status, body) = send_json(
        state.clone(),
        "POST",
        "/api/v1/agent/dispatch",
        json!({
            "tool": "health",
            "arguments": {}
        }),
    )
    .await;

    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["output"], json!({ "status": "ok" }));
    assert!(body["error"].is_null());

    let (status, body) = send_json(state, "POST", "/api/v1/audit/verify", json!({})).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body, "Ok");
}

#[tokio::test]
async fn a2a_tasks_send_dispatches_tool() {
    let state = test_state();
    let (status, body) = send_json(
        state,
        "POST",
        "/a2a/tasks/send",
        json!({
            "id": "task-1",
            "message": {
                "role": "user",
                "parts": [
                    {
                        "type": "text",
                        "text": r#"{"tool":"health","arguments":{}}"#
                    }
                ]
            }
        }),
    )
    .await;

    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["id"], "task-1");
    assert_eq!(body["status"], "completed");
    let text = &body["artifacts"][0]["text"];
    assert!(text.as_str().unwrap().contains("ok"));
}

#[tokio::test]
async fn mcp_tools_call_dispatches_tool() {
    let state = test_state();
    let (status, body) = send_json(
        state,
        "POST",
        "/mcp/tools/call",
        json!({
            "method": "tools/call",
            "params": {
                "name": "health",
                "arguments": {}
            }
        }),
    )
    .await;

    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["is_error"], false);
    assert!(body["content"][0]["text"].as_str().unwrap().contains("ok"));
}

#[tokio::test]
async fn market_data_endpoint_returns_bars() {
    let state = test_state();
    let (status, body) = send_json(
        state,
        "GET",
        "/api/v1/market-data/TEST?start=2024-01-01&end=2024-01-05&interval=daily",
        json!({}),
    )
    .await;

    assert_eq!(status, StatusCode::OK);
    let bars = body.as_array().expect("bars array");
    assert_eq!(bars.len(), 5);
}
