//! The four pay tools exposed as an MCP streamable-HTTP service, nested into
//! the server's axum router so `cargo run -p tl-server` serves them.
//!
//! Tool input schemas are derived with rmcp's re-exported `schemars`, so the
//! local arg DTOs (mapped to tl-core types) never depend on tl-core's own
//! schema feature.
//!
// ponytail: single default workspace for now — the standalone gate was
// single-tenant. Resolve per-request workspace from auth/Mcp-Session if this
// ever needs to be multi-tenant.

use std::sync::Arc;

use axum::Router;
use rmcp::handler::server::router::tool::ToolRouter;
use rmcp::handler::server::wrapper::Parameters;
use rmcp::model::{
    CallToolResult, ContentBlock, Implementation, ProtocolVersion, ServerCapabilities, ServerInfo,
};
use rmcp::transport::streamable_http_server::session::local::LocalSessionManager;
use rmcp::transport::streamable_http_server::{StreamableHttpServerConfig, StreamableHttpService};
use rmcp::{schemars, tool, tool_handler, tool_router, ErrorData as McpError, ServerHandler};
use tl_core::{PayPolicy, PaySpendRequest, DEFAULT_WORKSPACE_ID};
use tl_pay_mcp::PayError;
use tl_storage::{PayDecisionRepo, PayPolicyRepo};

use super::backend::PayBackendImpl;

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

fn to_mcp_err(e: PayError) -> McpError {
    McpError::internal_error(e.to_string(), None)
}

fn json_result<T: serde::Serialize>(value: &T) -> Result<CallToolResult, McpError> {
    let text = serde_json::to_string(value)
        .map_err(|e| McpError::internal_error(format!("serialize: {e}"), None))?;
    Ok(CallToolResult::success(vec![ContentBlock::text(text)]))
}

/// MCP server exposing the pay tools. One instance per session; cheap to
/// build (just clones the repo `Arc`s).
#[derive(Clone)]
pub struct PayMcpServer {
    policy: Arc<PayPolicyRepo>,
    decisions: Arc<PayDecisionRepo>,
    tool_router: ToolRouter<PayMcpServer>,
}

#[tool_router]
impl PayMcpServer {
    pub fn new(policy: Arc<PayPolicyRepo>, decisions: Arc<PayDecisionRepo>) -> Self {
        Self {
            policy,
            decisions,
            tool_router: Self::tool_router(),
        }
    }

    fn backend(&self) -> PayBackendImpl {
        PayBackendImpl::new(
            DEFAULT_WORKSPACE_ID.to_string(),
            self.policy.clone(),
            self.decisions.clone(),
        )
    }

    #[tool(description = "Set per-owner spend caps (amounts in minor units, e.g. cents)")]
    async fn set_policy(
        &self,
        Parameters(args): Parameters<SetPolicyArgs>,
    ) -> Result<CallToolResult, McpError> {
        let policy = PayPolicy {
            owner: args.owner,
            per_transaction_minor: args.per_transaction_minor,
            daily_minor: args.daily_minor,
            monthly_minor: args.monthly_minor,
            hold_above_minor: args.hold_above_minor,
        };
        tl_pay_mcp::tools::set_policy(&self.backend(), policy)
            .await
            .map_err(to_mcp_err)?;
        Ok(CallToolResult::success(vec![ContentBlock::text(
            "policy set",
        )]))
    }

    #[tool(description = "Gate a spend → {status: allow|block|hold, reason, decision_id}")]
    async fn pay(&self, Parameters(args): Parameters<PayArgs>) -> Result<CallToolResult, McpError> {
        let req = PaySpendRequest {
            owner: args.owner,
            amount_minor: args.amount_minor,
            merchant: args.merchant,
            category: args.category,
            memo: args.memo,
        };
        let decision = tl_pay_mcp::tools::pay(&self.backend(), req)
            .await
            .map_err(to_mcp_err)?;
        json_result(&decision)
    }

    #[tool(description = "Approve or deny a held spend by decision_id")]
    async fn resolve_hold(
        &self,
        Parameters(args): Parameters<ResolveHoldArgs>,
    ) -> Result<CallToolResult, McpError> {
        let found =
            tl_pay_mcp::tools::resolve_hold(&self.backend(), &args.decision_id, args.approve)
                .await
                .map_err(to_mcp_err)?;
        let msg = if found {
            if args.approve {
                "approved"
            } else {
                "denied"
            }
        } else {
            "no matching hold"
        };
        Ok(CallToolResult::success(vec![ContentBlock::text(msg)]))
    }

    #[tool(description = "Every decision for an owner, oldest first")]
    async fn export_audit(
        &self,
        Parameters(args): Parameters<ExportAuditArgs>,
    ) -> Result<CallToolResult, McpError> {
        let entries = tl_pay_mcp::tools::export_audit(&self.backend(), &args.owner)
            .await
            .map_err(to_mcp_err)?;
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
pub fn pay_mcp_routes(policy: Arc<PayPolicyRepo>, decisions: Arc<PayDecisionRepo>) -> Router {
    let service = StreamableHttpService::new(
        move || Ok(PayMcpServer::new(policy.clone(), decisions.clone())),
        LocalSessionManager::default().into(),
        StreamableHttpServerConfig::default(),
    );
    Router::new().nest_service("/mcp/pay", service)
}
