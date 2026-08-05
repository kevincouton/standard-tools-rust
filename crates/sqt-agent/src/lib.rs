//! Agent crate for the Standard Tools Rust port.
//!
//! This crate exposes a registry of 42+ quantitative-finance tools and a
//! [`ToolDispatcher`] that routes incoming [`ToolCall`] requests to the
//! appropriate domain service.

pub mod dispatcher;
pub mod registry;
pub mod tool;

pub use dispatcher::ToolDispatcher;
pub use registry::{find, list};
pub use tool::{ToolCall, ToolDefinition, ToolResult};
