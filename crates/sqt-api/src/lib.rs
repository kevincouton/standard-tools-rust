//! Standard-Tools API crate.
//!
//! Exposes REST, gRPC, A2A, and MCP endpoints and wires the domain crates
//! together. The binary entry point is `src/main.rs`.

pub mod a2a;
pub mod auth;
pub mod cli;
pub mod config;
pub mod grpc;
pub mod mcp;
pub mod rest;
pub mod server;
pub mod services;
pub mod state;
