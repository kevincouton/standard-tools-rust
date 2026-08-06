//! gRPC service implementations.

use std::sync::Arc;

use sqt_audit::AuditStorage;
use tonic::{Request, Response, Status};

use crate::state::AppState;

pub mod proto {
    pub mod health {
        tonic::include_proto!("standard_tools.health");
    }
    pub mod agent {
        tonic::include_proto!("standard_tools.agent");
    }
}

use proto::agent::{
    agent_service_server::AgentService as Agent, DispatchRequest, DispatchResponse,
    ListToolsRequest, ListToolsResponse, ToolDefinition,
};
use proto::health::{
    health_service_server::HealthService as Health, HealthCheckRequest, HealthCheckResponse,
};
use sqt_agent::ToolCall;

/// gRPC health service.
#[derive(Clone)]
pub struct HealthService;

#[tonic::async_trait]
impl Health for HealthService {
    async fn check(
        &self,
        _request: Request<HealthCheckRequest>,
    ) -> std::result::Result<Response<HealthCheckResponse>, Status> {
        Ok(Response::new(HealthCheckResponse {
            status: "ok".to_string(),
        }))
    }
}

/// gRPC agent service.
#[derive(Clone)]
pub struct AgentService<S: AuditStorage> {
    state: Arc<AppState<S>>,
}

impl<S: AuditStorage> AgentService<S> {
    pub fn new(state: Arc<AppState<S>>) -> Self {
        Self { state }
    }
}

#[tonic::async_trait]
impl<S: AuditStorage + 'static> Agent for AgentService<S> {
    async fn list_tools(
        &self,
        _request: Request<ListToolsRequest>,
    ) -> std::result::Result<Response<ListToolsResponse>, Status> {
        let tools = sqt_agent::registry::list()
            .iter()
            .map(|t| ToolDefinition {
                name: t.name.clone(),
                description: t.description.clone(),
                parameters_json: t.parameters.to_string(),
            })
            .collect();
        Ok(Response::new(ListToolsResponse { tools }))
    }

    async fn dispatch(
        &self,
        request: Request<DispatchRequest>,
    ) -> std::result::Result<Response<DispatchResponse>, Status> {
        let req = request.into_inner();
        let arguments: serde_json::Value = serde_json::from_str(&req.arguments_json)
            .map_err(|e| Status::invalid_argument(format!("invalid arguments JSON: {e}")))?;

        let call = ToolCall {
            name: req.tool_name.clone(),
            arguments,
        };

        match self.state.dispatcher.dispatch(call).await {
            Ok(result) => Ok(Response::new(DispatchResponse {
                output_json: result.output.to_string(),
                error: result.error,
            })),
            Err(e) => Ok(Response::new(DispatchResponse {
                output_json: "{}".to_string(),
                error: Some(e.to_string()),
            })),
        }
    }
}
