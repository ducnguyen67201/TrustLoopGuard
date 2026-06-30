//! Payment-gate MCP surface mounted into the server.
//!
//! [`PayBackendImpl`] adapts the durable repos to the `tl_pay_mcp::PayBackend`
//! trait; the HTTP route exposes the four pay tools.

mod backend;
mod route;

pub use backend::PayBackendImpl;
pub use route::pay_mcp_routes;
