//! A2A (Agent-to-Agent) JSON-RPC style handler.
//!
//! Implements a tiny subset of the A2A task protocol:
//! - `tasks/send` — dispatch a tool and return a task object.
//! - `tasks/get` — get task status (placeholder).
//! - `tasks/cancel` — cancel a task (placeholder).

use std::sync::Arc;

use axum::{extract::State, response::IntoResponse, routing::post, Json, Router};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use sqt_agent::ToolCall;
use sqt_audit::AuditStorage;

use crate::state::AppState;

pub fn router<S: AuditStorage + 'static>(state: Arc<AppState<S>>) -> Router {
    Router::new()
        .route("/a2a/tasks/send", post(tasks_send::<S>))
        .route("/a2a/tasks/get", post(tasks_get::<S>))
        .route("/a2a/tasks/cancel", post(tasks_cancel::<S>))
        .with_state(state)
}

#[derive(Debug, Deserialize)]
struct TaskSendRequest {
    id: String,
    message: A2AMessage,
}

#[derive(Debug, Deserialize)]
struct A2AMessage {
    #[allow(dead_code)]
    role: String,
    parts: Vec<A2APart>,
}

#[derive(Debug, Deserialize)]
struct A2APart {
    #[serde(rename = "type")]
    part_type: String,
    text: String,
}

#[derive(Debug, Serialize)]
struct TaskResponse {
    id: String,
    status: TaskStatus,
    artifacts: Vec<Artifact>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "snake_case")]
enum TaskStatus {
    Completed,
    Failed,
}

#[derive(Debug, Serialize)]
struct Artifact {
    #[serde(rename = "type")]
    artifact_type: String,
    text: String,
}

async fn tasks_send<S: AuditStorage>(
    State(state): State<Arc<AppState<S>>>,
    Json(req): Json<TaskSendRequest>,
) -> impl IntoResponse {
    let call = parse_tool_call(&req.message);
    match call {
        Ok(tool_call) => match state.dispatcher.dispatch(tool_call).await {
            Ok(result) => {
                let text = result
                    .error
                    .map(|e| format!("{{\"error\": \"{e}\"}}"))
                    .unwrap_or_else(|| result.output.to_string());
                Json(TaskResponse {
                    id: req.id,
                    status: TaskStatus::Completed,
                    artifacts: vec![Artifact {
                        artifact_type: "text".to_string(),
                        text,
                    }],
                })
                .into_response()
            }
            Err(e) => Json(TaskResponse {
                id: req.id,
                status: TaskStatus::Failed,
                artifacts: vec![Artifact {
                    artifact_type: "text".to_string(),
                    text: e.to_string(),
                }],
            })
            .into_response(),
        },
        Err(e) => Json(TaskResponse {
            id: req.id,
            status: TaskStatus::Failed,
            artifacts: vec![Artifact {
                artifact_type: "text".to_string(),
                text: e,
            }],
        })
        .into_response(),
    }
}

fn parse_tool_call(message: &A2AMessage) -> Result<ToolCall, String> {
    let text = message
        .parts
        .iter()
        .find(|p| p.part_type == "text")
        .map(|p| p.text.clone())
        .ok_or_else(|| "missing text part".to_string())?;

    let value: Value = serde_json::from_str(&text).map_err(|e| e.to_string())?;
    let tool = value
        .get("tool")
        .and_then(Value::as_str)
        .ok_or_else(|| "missing tool".to_string())?
        .to_string();
    let arguments = value
        .get("arguments")
        .cloned()
        .unwrap_or(Value::Object(Default::default()));

    Ok(ToolCall {
        name: tool,
        arguments,
    })
}

async fn tasks_get<S: AuditStorage>(State(_state): State<Arc<AppState<S>>>) -> impl IntoResponse {
    Json(TaskResponse {
        id: "unknown".to_string(),
        status: TaskStatus::Completed,
        artifacts: vec![],
    })
}

async fn tasks_cancel<S: AuditStorage>(
    State(_state): State<Arc<AppState<S>>>,
) -> impl IntoResponse {
    Json(TaskResponse {
        id: "unknown".to_string(),
        status: TaskStatus::Completed,
        artifacts: vec![],
    })
}
