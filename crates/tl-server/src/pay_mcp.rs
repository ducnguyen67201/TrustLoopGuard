//! Payment MCP surface: a thin transport shim over the pay gate.
//!
//! All behavior lives in [`crate::services::pay_service::PayGate`] — judge
//! every spend through the unified `/v1/events` path, and on ALLOW execute
//! it against the workspace's vaulted `payment_http` provider connection.
//! The agent never holds the payment credential: skipping this tool means
//! being unable to pay at all.
//!
//! Tools:
//! - `set_policy`   → upsert a `payment`-family policy (per-owner caps)
//! - `pay`          → judge, then execute on allow
//! - `resolve_hold` → deny, or approve-and-execute a held spend
//! - `export_audit` → the owner's payment decision trail
//!
//! Served as MCP streamable-HTTP, nested into the router at `/mcp/pay` under
//! the bearer-auth layer.
//!
// ponytail: single default workspace/environment for now — the gate is
// single-tenant here. Resolve per-request workspace from auth if this needs to
// be multi-tenant.

use std::sync::Arc;

use axum::Router;
use rmcp::handler::server::router::tool::ToolRouter;
use rmcp::handler::server::wrapper::Parameters;
use rmcp::model::{
    CallToolResult, ContentBlock, Implementation, ProtocolVersion, ServerCapabilities, ServerInfo,
};
use rmcp::service::RequestContext;
use rmcp::transport::streamable_http_server::session::local::LocalSessionManager;
use rmcp::transport::streamable_http_server::{StreamableHttpServerConfig, StreamableHttpService};
use rmcp::{
    schemars, tool, tool_handler, tool_router, ErrorData as McpError, RoleServer, ServerHandler,
};
use tl_core::{DEFAULT_ENVIRONMENT_ID, DEFAULT_WORKSPACE_ID};

use crate::services::pay_service::{PayGate, PayRequest, SpendCaps};
use crate::AppState;

/// Resolve `(workspace_id, environment_id)` from the auth-stamped request
/// headers. The bearer-auth middleware sets `x-tlg-workspace-id` /
/// `x-tlg-environment-id` from the authenticated credential (workspace API key
/// today, OAuth access token next); absent (unauthenticated dev) → defaults.
fn workspace_env(ctx: &RequestContext<RoleServer>) -> (String, String) {
    let headers = ctx
        .extensions
        .get::<axum::http::request::Parts>()
        .map(|parts| &parts.headers);
    let header = |name: &str, fallback: &str| -> String {
        headers
            .and_then(|h| h.get(name))
            .and_then(|v| v.to_str().ok())
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .unwrap_or(fallback)
            .to_string()
    };
    (
        header("x-tlg-workspace-id", DEFAULT_WORKSPACE_ID),
        header("x-tlg-environment-id", DEFAULT_ENVIRONMENT_ID),
    )
}

#[derive(serde::Deserialize, schemars::JsonSchema)]
struct SetPolicyArgs {
    owner: String,
    per_transaction_minor: Option<i64>,
    daily_minor: Option<i64>,
    monthly_minor: Option<i64>,
    hold_above_minor: Option<i64>,
}

#[derive(serde::Deserialize, schemars::JsonSchema)]
struct PayArgs {
    owner: String,
    amount_minor: i64,
    merchant: String,
    category: Option<String>,
    memo: Option<String>,
}

#[derive(serde::Deserialize, schemars::JsonSchema)]
struct ResolveHoldArgs {
    decision_id: String,
    approve: bool,
}

#[derive(serde::Deserialize, schemars::JsonSchema)]
struct ExportAuditArgs {
    owner: String,
}

fn err(message: impl Into<String>) -> McpError {
    McpError::internal_error(message.into(), None)
}

fn json_result<T: serde::Serialize>(value: &T) -> Result<CallToolResult, McpError> {
    let text = serde_json::to_string(value).map_err(|e| err(format!("serialize: {e}")))?;
    Ok(CallToolResult::success(vec![ContentBlock::text(text)]))
}

/// MCP server exposing the pay tools. Cheap to clone (just a `PayGate`).
#[derive(Clone)]
pub struct PayMcpServer {
    gate: PayGate,
    // Built by `#[tool_router]` and consumed by `#[tool_handler]` via generated
    // code the dead-code lint can't see; rmcp's own examples allow this.
    #[allow(dead_code)]
    tool_router: ToolRouter<PayMcpServer>,
}

#[tool_router]
impl PayMcpServer {
    pub fn new(gate: PayGate) -> Self {
        Self {
            gate,
            tool_router: Self::tool_router(),
        }
    }

    #[tool(description = "Set per-owner spend caps (amounts in minor units, e.g. cents)")]
    async fn set_policy(
        &self,
        Parameters(args): Parameters<SetPolicyArgs>,
        ctx: RequestContext<RoleServer>,
    ) -> Result<CallToolResult, McpError> {
        let (workspace_id, environment_id) = workspace_env(&ctx);
        self.gate
            .set_policy(
                &workspace_id,
                &environment_id,
                SpendCaps {
                    owner: args.owner,
                    per_transaction_minor: args.per_transaction_minor,
                    daily_minor: args.daily_minor,
                    monthly_minor: args.monthly_minor,
                    hold_above_minor: args.hold_above_minor,
                },
            )
            .await
            .map_err(err)?;
        Ok(CallToolResult::success(vec![ContentBlock::text(
            "policy set",
        )]))
    }

    #[tool(
        description = "Gate a spend and execute it on allow → {status: executed|allow_no_provider|allow_failed_execute|block|hold, reason, decision_id}"
    )]
    async fn pay(
        &self,
        Parameters(args): Parameters<PayArgs>,
        ctx: RequestContext<RoleServer>,
    ) -> Result<CallToolResult, McpError> {
        let (workspace_id, environment_id) = workspace_env(&ctx);
        let outcome = self
            .gate
            .pay(
                &workspace_id,
                &environment_id,
                PayRequest {
                    owner: args.owner,
                    amount_minor: args.amount_minor,
                    merchant: args.merchant,
                    category: args.category,
                    memo: args.memo,
                },
            )
            .await
            .map_err(err)?;
        json_result(&outcome)
    }

    #[tool(
        description = "Approve (and execute) or deny a held spend by decision_id (the trace id)"
    )]
    async fn resolve_hold(
        &self,
        Parameters(args): Parameters<ResolveHoldArgs>,
        ctx: RequestContext<RoleServer>,
    ) -> Result<CallToolResult, McpError> {
        let (workspace_id, environment_id) = workspace_env(&ctx);
        let outcome = self
            .gate
            .resolve_hold(
                &workspace_id,
                &environment_id,
                &args.decision_id,
                args.approve,
            )
            .await
            .map_err(err)?;
        json_result(&outcome)
    }

    #[tool(description = "List an owner's payment decisions (the audit trail)")]
    async fn export_audit(
        &self,
        Parameters(args): Parameters<ExportAuditArgs>,
        ctx: RequestContext<RoleServer>,
    ) -> Result<CallToolResult, McpError> {
        let (workspace_id, environment_id) = workspace_env(&ctx);
        let entries = self
            .gate
            .export_audit(&workspace_id, &environment_id, &args.owner)
            .await
            .map_err(err)?;
        json_result(&entries)
    }
}

#[tool_handler]
impl ServerHandler for PayMcpServer {
    fn get_info(&self) -> ServerInfo {
        ServerInfo::new(ServerCapabilities::builder().enable_tools().build())
            .with_server_info(Implementation::from_build_env())
            .with_protocol_version(ProtocolVersion::V_2024_11_05)
            .with_instructions(
                "TrustLoop payment gate. Tools: set_policy, pay, resolve_hold, export_audit."
                    .to_string(),
            )
    }
}

/// MCP streamable-HTTP service for the pay tools, nested at `/mcp/pay`.
/// `seal_key` is the gateway credential key — the pay gate unseals the same
/// vaulted provider credentials the LLM gateway routes use.
pub fn pay_mcp_routes(state: AppState, seal_key: [u8; 32]) -> Router {
    let http = reqwest::Client::builder()
        .connect_timeout(std::time::Duration::from_secs(10))
        .timeout(std::time::Duration::from_secs(60))
        .build()
        .expect("pay HTTP client");
    let gate = PayGate::new(state, seal_key, http);
    let service = StreamableHttpService::new(
        move || Ok(PayMcpServer::new(gate.clone())),
        Arc::new(LocalSessionManager::default()),
        StreamableHttpServerConfig::default(),
    );
    Router::new().nest_service("/mcp/pay", service)
}
