//! Hosted, OAuth-authenticated MCP access gateway.

mod api;
mod bounded_http;
mod handler;
mod memory;
mod naming;
mod service;
mod store;
mod upstream;

#[cfg(test)]
mod regression_tests;

pub use api::*;
pub use handler::HostedMcpHandler;
pub use memory::MemoryMcpGatewayStore;
pub use service::{require_mcp_workspace_access, McpGatewayState};
pub use store::*;
