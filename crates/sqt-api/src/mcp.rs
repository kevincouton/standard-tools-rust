//! MCP (Model Context Protocol) handler.
//!
//! Implements a tiny subset of the MCP protocol:
//! - `tools/list` — list registered tools.
//! - `tools/call` — dispatch a tool.

use std::sync::Arc;

use axum::{extract::State, response::IntoResponse, routing::post, Json, Router};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use sqt_agent::ToolCall;
use sqt_audit::AuditStorage;

use crate::state::AppState;

pub fn router<S: AuditStorage + 'static>(state: Arc<AppState<S>>) -> Router {
    Router::new()
        .route("/mcp/tools/list", post(tools_list::<S>))
        .route("/mcp/tools/call", post(tools_call::<S>))
        .with_state(state)
}

#[derive(Debug, Deserialize)]
struct McpRequest {
    #[allow(dead_code)]
    method: String,
    params: Option<Value>,
}

#[derive(Debug, Serialize)]
struct McpResponse {
    content: Vec<McpContent>,
    is_error: bool,
}

#[derive(Debug, Serialize)]
struct McpContent {
    #[serde(rename = "type")]
    content_type: String,
    text: String,
}

#[derive(Debug, Serialize)]
struct McpTool {
    name: String,
    description: String,
    input_schema: Value,
}

#[derive(Debug, Serialize)]
struct ToolsListResult {
    tools: Vec<McpTool>,
}

async fn tools_list<S: AuditStorage>(State(_state): State<Arc<AppState<S>>>) -> impl IntoResponse {
    let tools = sqt_agent::registry::list()
        .iter()
        .map(|t| McpTool {
            name: t.name.clone(),
            description: t.description.clone(),
            input_schema: t.parameters.clone(),
        })
        .collect();

    let result = ToolsListResult { tools };
    Json(McpResponse {
        content: vec![McpContent {
            content_type: "text".to_string(),
            text: serde_json::to_string(&result).unwrap_or_default(),
        }],
        is_error: false,
    })
}

async fn tools_call<S: AuditStorage>(
    State(state): State<Arc<AppState<S>>>,
    Json(req): Json<McpRequest>,
) -> impl IntoResponse {
    let params = req.params.unwrap_or(Value::Object(Default::default()));
    let name = params
        .get("name")
        .and_then(Value::as_str)
        .unwrap_or("")
        .to_string();
    let arguments = params
        .get("arguments")
        .cloned()
        .unwrap_or(Value::Object(Default::default()));

    let call = ToolCall { name, arguments };
    match state.dispatcher.dispatch(call).await {
        Ok(result) => {
            let is_error = result.error.is_some();
            let text = result.error.unwrap_or_else(|| result.output.to_string());
            Json(McpResponse {
                content: vec![McpContent {
                    content_type: "text".to_string(),
                    text,
                }],
                is_error,
            })
            .into_response()
        }
        Err(e) => Json(McpResponse {
            content: vec![McpContent {
                content_type: "text".to_string(),
                text: e.to_string(),
            }],
            is_error: true,
        })
        .into_response(),
    }
}
