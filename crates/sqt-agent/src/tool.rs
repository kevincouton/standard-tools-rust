//! Agent tool types.
//!
//! This module defines the shared data structures used to describe, invoke, and
//! observe the results of tools exposed by the `sqt-agent` crate.

use serde::{Deserialize, Serialize};
use serde_json::Value;

/// Metadata describing a single tool that can be invoked by an agent.
///
/// The `parameters` field is a JSON Schema object describing the expected
/// arguments for the tool. It is intentionally a [`serde_json::Value`] so that
/// callers can embed arbitrary schema definitions.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ToolDefinition {
    /// Unique tool name, e.g. `"compute_sma"`.
    pub name: String,

    /// Human-readable description of what the tool does.
    pub description: String,

    /// JSON Schema object describing the tool's parameters.
    pub parameters: Value,
}

/// A request to invoke a named tool with a set of arguments.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ToolCall {
    /// Name of the tool to invoke.
    pub name: String,

    /// Tool-specific arguments, typically parsed from JSON.
    pub arguments: Value,
}

/// The outcome of a tool invocation.
///
/// A successful invocation returns its payload in `output`. When a known tool
/// fails during execution, the error message is placed in `error` and the
/// dispatcher still returns `Ok`. Unknown tools produce a [`QuantError`](sqt_core::QuantError).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ToolResult {
    /// Tool-specific output value, e.g. a computed price or a list of trades.
    pub output: Value,

    /// Error message when the tool was recognised but failed during execution.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

impl ToolResult {
    /// Creates a successful result containing `output`.
    pub fn ok(output: Value) -> Self {
        Self {
            output,
            error: None,
        }
    }

    /// Creates a result representing a failed execution of a known tool.
    pub fn err(message: impl Into<String>) -> Self {
        Self {
            output: Value::Null,
            error: Some(message.into()),
        }
    }
}
