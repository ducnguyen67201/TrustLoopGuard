//! Payment MCP surface: a thin shim over the unified gate.
//!
//! Each tool maps to existing machinery — no separate evaluator or database:
//! - `pay`          → build a `GuardEvent` and run the normal `/v1/events` path
//! - `set_policy`   → upsert a `payment`-family policy
//! - `resolve_hold` → record a human-review outcome on the decision's trace
//! - `export_audit` → list the owner's payment traces
//!
//! Served as MCP streamable-HTTP, nested into the router at `/mcp/pay` under the
//! bearer-auth layer.
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
use rmcp::transport::streamable_http_server::session::local::LocalSessionManager;
use rmcp::transport::streamable_http_server::{StreamableHttpServerConfig, StreamableHttpService};
use rmcp::{schemars, tool, tool_handler, tool_router, ErrorData as McpError, ServerHandler};
use tl_core::{
    Action as EventAction, CreateHumanReviewEventRequest, EventKind, GuardEvent,
    HumanReviewOutcome, Principal, Verdict, DEFAULT_ENVIRONMENT_ID, DEFAULT_WORKSPACE_ID,
};
use tl_policy::{Action, FamilyPolicy, PaymentPolicy, PaymentWhen};

use crate::services::event_service::execute_event_submission;
use crate::AppState;

/// The operation name a `pay` tool call submits, and the one `export_audit`
/// filters on.
const PAY_OPERATION: &str = "pay";

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

/// MCP server exposing the pay tools. Cheap to clone (just an `AppState`).
#[derive(Clone)]
pub struct PayMcpServer {
    state: AppState,
    // Built by `#[tool_router]` and consumed by `#[tool_handler]` via generated
    // code the dead-code lint can't see; rmcp's own examples allow this.
    #[allow(dead_code)]
    tool_router: ToolRouter<PayMcpServer>,
}

#[tool_router]
impl PayMcpServer {
    pub fn new(state: AppState) -> Self {
        Self {
            state,
            tool_router: Self::tool_router(),
        }
    }

    #[tool(description = "Set per-owner spend caps (amounts in minor units, e.g. cents)")]
    async fn set_policy(
        &self,
        Parameters(args): Parameters<SetPolicyArgs>,
    ) -> Result<CallToolResult, McpError> {
        let policy = FamilyPolicy::Payment(PaymentPolicy {
            id: format!("pay-{}", args.owner),
            description: Some(format!("Payment caps for {}", args.owner)),
            severity: tl_core::Severity::High,
            when: PaymentWhen {
                agents: vec![args.owner],
                operations: vec![PAY_OPERATION.to_string()],
            },
            per_transaction_minor: args.per_transaction_minor,
            hold_above_minor: args.hold_above_minor,
            daily_minor: args.daily_minor,
            monthly_minor: args.monthly_minor,
            on_breach: Action::Block,
        });
        let yaml = serde_yaml::to_string(&policy).map_err(|e| err(format!("policy yaml: {e}")))?;
        self.state
            .policy_store
            .upsert_family(DEFAULT_WORKSPACE_ID, DEFAULT_ENVIRONMENT_ID, &policy, &yaml)
            .await
            .map_err(|e| err(format!("set_policy: {e}")))?;
        Ok(CallToolResult::success(vec![ContentBlock::text(
            "policy set",
        )]))
    }

    #[tool(description = "Gate a spend → {status: allow|block|hold, reason, decision_id}")]
    async fn pay(&self, Parameters(args): Parameters<PayArgs>) -> Result<CallToolResult, McpError> {
        let mut parameters = serde_json::Map::new();
        parameters.insert("amount".into(), args.amount_minor.into());
        parameters.insert("merchant".into(), args.merchant.into());
        if let Some(category) = args.category {
            parameters.insert("category".into(), category.into());
        }
        if let Some(memo) = args.memo {
            parameters.insert("memo".into(), memo.into());
        }
        let event = GuardEvent {
            kind: EventKind::ToolCallProposed,
            principal: Principal {
                workspace_id: DEFAULT_WORKSPACE_ID.to_string(),
                environment_id: DEFAULT_ENVIRONMENT_ID.to_string(),
                agent_id: args.owner,
                user_id: None,
                session_id: None,
                task_id: None,
                run_id: None,
                run_event_id: None,
            },
            action: EventAction {
                operation: PAY_OPERATION.to_string(),
                parameters: serde_json::Value::Object(parameters),
                side_effect: None,
            },
            sources: vec![],
            provenance: Default::default(),
            resolution: None,
            label_resolution: None,
            checks: vec![],
            signals: vec![],
            context: serde_json::Value::Null,
        };

        let decision = execute_event_submission(
            &self.state,
            DEFAULT_WORKSPACE_ID,
            DEFAULT_ENVIRONMENT_ID,
            event,
            std::time::Instant::now(),
        )
        .await
        .map_err(|_| err("payment evaluation failed"))?;

        let status = match decision.verdict {
            Verdict::Allow => "allow",
            Verdict::Escalate => "hold",
            Verdict::Block | Verdict::Rewrite => "block",
        };
        json_result(&serde_json::json!({
            "status": status,
            "reason": decision.reason,
            "decision_id": decision.trace_id,
        }))
    }

    #[tool(description = "Approve or deny a held spend by decision_id (the trace id)")]
    async fn resolve_hold(
        &self,
        Parameters(args): Parameters<ResolveHoldArgs>,
    ) -> Result<CallToolResult, McpError> {
        let outcome = if args.approve {
            HumanReviewOutcome::Accepted
        } else {
            HumanReviewOutcome::Rejected
        };
        self.state
            .human_review_store
            .create_event(
                DEFAULT_WORKSPACE_ID,
                &args.decision_id,
                CreateHumanReviewEventRequest {
                    outcome,
                    reason_codes: vec![],
                    note: None,
                    metadata: serde_json::Value::Null,
                },
                None,
            )
            .await
            .map_err(|e| err(format!("resolve_hold: {e}")))?;
        let msg = if args.approve { "approved" } else { "denied" };
        Ok(CallToolResult::success(vec![ContentBlock::text(msg)]))
    }

    #[tool(description = "List an owner's payment decisions (the audit trail)")]
    async fn export_audit(
        &self,
        Parameters(args): Parameters<ExportAuditArgs>,
    ) -> Result<CallToolResult, McpError> {
        let traces = self
            .state
            .trace_store
            .list_recent(DEFAULT_WORKSPACE_ID, DEFAULT_ENVIRONMENT_ID, None, 100)
            .await
            .map_err(|e| err(format!("export_audit: {e}")))?;
        let entries: Vec<_> = traces
            .into_iter()
            .filter(|t| is_payment_for_owner(&t.payload, &args.owner))
            .map(|t| {
                serde_json::json!({
                    "decision_id": t.trace_id,
                    "decision": t.decision,
                    "created_at": t.created_at,
                    "amount_minor": payment_field(&t.payload, "amount"),
                    "merchant": payment_field(&t.payload, "merchant"),
                })
            })
            .collect();
        json_result(&entries)
    }
}

/// A trace is a payment for `owner` when its event operation is `pay` and the
/// principal matches.
fn is_payment_for_owner(payload: &serde_json::Value, owner: &str) -> bool {
    let event = payload.get("event");
    let op = event
        .and_then(|e| e.get("action"))
        .and_then(|a| a.get("operation"))
        .and_then(|v| v.as_str());
    let agent = event
        .and_then(|e| e.get("principal"))
        .and_then(|p| p.get("agent_id"))
        .and_then(|v| v.as_str());
    op == Some(PAY_OPERATION) && agent == Some(owner)
}

fn payment_field<'a>(payload: &'a serde_json::Value, field: &str) -> Option<&'a serde_json::Value> {
    payload
        .get("event")?
        .get("action")?
        .get("parameters")?
        .get(field)
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
pub fn pay_mcp_routes(state: AppState) -> Router {
    let service = StreamableHttpService::new(
        move || Ok(PayMcpServer::new(state.clone())),
        Arc::new(LocalSessionManager::default()),
        StreamableHttpServerConfig::default(),
    );
    Router::new().nest_service("/mcp/pay", service)
}
