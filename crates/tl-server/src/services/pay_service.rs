//! The pay gate: compatibility wrapper for the MCP pay tools.
//!
//! Transport-independent — the MCP surface (`crate::pay_mcp`) is a thin shim
//! over this service, and tests drive it directly. New spend intent is stored
//! as typed financial actions and executed through the financial authorization
//! service; the JSON statuses stay stable for old MCP clients.

use std::collections::HashMap;
use std::sync::{Arc, Mutex as StdMutex};

use serde_json::json;
use tokio::sync::Mutex as AsyncMutex;

use tl_core::{
    CounterpartyRef, CreateFinancialActionRequest, FinancialAction, FinancialActionStatus,
    FinancialRail, GatewayProviderKind, MoneyAmount,
};
use tl_policy::{Action, FamilyPolicy, PaymentPolicy, PaymentWhen};

use crate::financial::{
    FinancialActionExecutionAttempt, FinancialAuthorizationService, PaymentHttpFinancialExecutor,
};
use crate::AppState;

/// The operation name a `pay` call submits, and the one audit filters on.
pub const PAY_OPERATION: &str = "pay";

#[derive(Debug, Clone)]
pub struct PayRequest {
    pub owner: String,
    pub amount_minor: i64,
    pub merchant: String,
    pub category: Option<String>,
    pub memo: Option<String>,
}

#[derive(Debug, Clone)]
pub struct SpendCaps {
    pub owner: String,
    pub per_transaction_minor: Option<i64>,
    pub daily_minor: Option<i64>,
    pub monthly_minor: Option<i64>,
    pub hold_above_minor: Option<i64>,
}

/// The pay gate: `AppState` + the gateway credential seal key + an HTTP
/// client for provider forwards. Cheap to clone (the per-decision lock map is
/// shared across clones so concurrent `resolve_hold`s serialize).
#[derive(Clone)]
pub struct PayGate {
    pub state: AppState,
    pub seal_key: [u8; 32],
    pub http: reqwest::Client,
    /// Per-decision async locks: serialize concurrent `resolve_hold` for the
    /// same held decision so the check-then-execute critical section is atomic
    /// in-process. Cross-process concurrency (multi-replica) still relies on
    /// the provider-side idempotency key.
    // ponytail: map grows one entry per distinct resolved hold; entries are
    // tiny. Add TTL eviction if hold volume ever makes this matter.
    hold_locks: Arc<StdMutex<HashMap<String, Arc<AsyncMutex<()>>>>>,
}

impl PayGate {
    pub fn new(state: AppState, seal_key: [u8; 32], http: reqwest::Client) -> Self {
        Self {
            state,
            seal_key,
            http,
            hold_locks: Arc::new(StdMutex::new(HashMap::new())),
        }
    }

    /// Get (or create) the async lock guarding a single held decision.
    fn hold_lock(&self, decision_id: &str) -> Arc<AsyncMutex<()>> {
        let mut locks = self.hold_locks.lock().expect("hold lock map");
        locks
            .entry(decision_id.to_string())
            .or_insert_with(|| Arc::new(AsyncMutex::new(())))
            .clone()
    }

    fn financial_service(&self) -> FinancialAuthorizationService {
        let executor = Arc::new(PaymentHttpFinancialExecutor::new(
            self.state.gateway_store.clone(),
            self.seal_key,
            self.http.clone(),
        ));
        FinancialAuthorizationService::with_policy_store_and_executor(
            self.state.financial_store.clone(),
            self.state.policy_store.clone(),
            executor,
        )
    }

    /// Upsert the per-owner spend-cap policy (a `payment`-family policy).
    pub async fn set_policy(
        &self,
        workspace_id: &str,
        environment_id: &str,
        caps: SpendCaps,
    ) -> Result<(), String> {
        let policy = FamilyPolicy::Payment(PaymentPolicy {
            id: format!("pay-{}", caps.owner),
            description: Some(format!("Payment caps for {}", caps.owner)),
            severity: tl_core::Severity::High,
            when: PaymentWhen {
                agents: vec![caps.owner],
                operations: vec![PAY_OPERATION.to_string()],
            },
            per_transaction_minor: caps.per_transaction_minor,
            hold_above_minor: caps.hold_above_minor,
            daily_minor: caps.daily_minor,
            monthly_minor: caps.monthly_minor,
            on_breach: Action::Block,
        });
        let yaml = serde_yaml::to_string(&policy).map_err(|e| format!("policy yaml: {e}"))?;
        self.state
            .policy_store
            .upsert_family(workspace_id, environment_id, &policy, &yaml)
            .await
            .map_err(|e| format!("set_policy: {e}"))
    }

    /// Judge a spend and execute it on allow. The returned JSON always
    /// carries `status` + `decision_id`; an unexecuted payment is never
    /// presented as executed.
    pub async fn pay(
        &self,
        workspace_id: &str,
        environment_id: &str,
        request: PayRequest,
    ) -> Result<serde_json::Value, String> {
        // Chokepoint guard: never forward a non-positive amount, even if no
        // payment policy matches this owner (no policy = no caps, but a
        // negative/zero "charge" must never reach the provider). Defense in
        // depth alongside per_call_verdict in tl-engine.
        if request.amount_minor <= 0 {
            return Ok(json!({
                "status": "block",
                "reason": format!("non-positive amount {} rejected", request.amount_minor),
                "decision_id": serde_json::Value::Null,
            }));
        }
        let service = self.financial_service();
        let action = service
            .create_action_in_environment(
                workspace_id,
                environment_id,
                payment_action_request(environment_id, request),
            )
            .await
            .map_err(|e| format!("payment evaluation failed: {e}"))?;

        match action.status {
            FinancialActionStatus::Denied => Ok(json!({
                "status": "block",
                "decision_id": action.id,
            })),
            FinancialActionStatus::Held => Ok(json!({
                "status": "hold",
                "decision_id": action.id,
            })),
            FinancialActionStatus::Proposed => {
                if !self.payment_provider_configured(workspace_id).await? {
                    return Ok(json!({
                        "status": "allow_no_provider",
                        "decision_id": action.id,
                    }));
                }
                let authorized = service
                    .authorize_action(workspace_id, &action.id)
                    .await
                    .map_err(|e| format!("payment authorization failed: {e}"))?;
                let executed = service
                    .execute_action(workspace_id, &authorized.id)
                    .await
                    .map_err(|e| format!("payment execution failed: {e}"))?;
                self.pay_execution_outcome(&service, workspace_id, executed)
                    .await
            }
            FinancialActionStatus::Authorized => {
                let executed = service
                    .execute_action(workspace_id, &action.id)
                    .await
                    .map_err(|e| format!("payment execution failed: {e}"))?;
                self.pay_execution_outcome(&service, workspace_id, executed)
                    .await
            }
            FinancialActionStatus::Executed | FinancialActionStatus::Failed => {
                self.pay_execution_outcome(&service, workspace_id, action)
                    .await
            }
            FinancialActionStatus::Reversed | FinancialActionStatus::Expired => Ok(json!({
                "status": "block",
                "decision_id": action.id,
            })),
        }
    }

    /// Approve (and execute) or deny a held spend.
    pub async fn resolve_hold(
        &self,
        workspace_id: &str,
        _environment_id: &str,
        decision_id: &str,
        approve: bool,
    ) -> Result<serde_json::Value, String> {
        // Denial is a simple record — no execution, no lock needed.
        if !approve {
            self.financial_service()
                .deny_action(workspace_id, decision_id)
                .await
                .map_err(|e| format!("resolve_hold: {e}"))?;
            return Ok(json!({
                "status": "denied",
                "decision_id": decision_id,
            }));
        }

        // Serialize concurrent approvals of the same hold so check → execute →
        // record is atomic in-process (fixes the TOCTOU double-execute). The
        // provider idempotency key is the cross-process backstop.
        let lock = self.hold_lock(decision_id);
        let _guard = lock.lock().await;

        let service = self.financial_service();
        let current = service
            .get_action(workspace_id, decision_id)
            .await
            .map_err(|e| format!("resolve_hold: {e}"))?;
        if current.status == FinancialActionStatus::Executed {
            return Ok(json!({
                "status": "already_approved",
                "decision_id": decision_id,
            }));
        }

        match service
            .execute_held_action_retryable(workspace_id, decision_id)
            .await
        {
            Ok(FinancialActionExecutionAttempt::Executed(action)) => {
                let provider_response =
                    latest_provider_response(&service, workspace_id, &action.id)
                        .await
                        .unwrap_or(serde_json::Value::Null);
                Ok(json!({
                    "status": "executed",
                    "decision_id": decision_id,
                    "provider_response": provider_response,
                }))
            }
            Ok(FinancialActionExecutionAttempt::Failed { reason, .. }) => {
                tracing::error!(workspace_id, decision_id, %reason, "held payment execution failed");
                Ok(json!({
                    "status": "approved_failed_execute",
                    "reason": reason,
                    "decision_id": decision_id,
                }))
            }
            Err(error) => Err(format!("resolve_hold: {error}")),
        }
    }

    async fn payment_provider_configured(&self, workspace_id: &str) -> Result<bool, String> {
        let connections = self
            .state
            .gateway_store
            .list_provider_connections(workspace_id)
            .await
            .map_err(|e| format!("payment connection lookup failed: {e}"))?;
        Ok(connections
            .into_iter()
            .any(|c| c.kind == GatewayProviderKind::PaymentHttp))
    }

    /// The owner's payment decisions (the audit trail).
    pub async fn export_audit(
        &self,
        workspace_id: &str,
        environment_id: &str,
        owner: &str,
    ) -> Result<serde_json::Value, String> {
        let actions = self
            .financial_service()
            .list_actions(workspace_id)
            .await
            .map_err(|e| format!("export_audit: {e}"))?;
        let entries: Vec<_> = actions
            .actions
            .into_iter()
            .filter(|action| {
                action.action.principal_id == owner
                    && action
                        .action
                        .metadata
                        .get("operation")
                        .and_then(serde_json::Value::as_str)
                        == Some(PAY_OPERATION)
                    && action
                        .action
                        .metadata
                        .get("environment_id")
                        .and_then(serde_json::Value::as_str)
                        == Some(environment_id)
            })
            .map(|action| {
                json!({
                    "decision_id": action.id,
                    "decision": action.status,
                    "created_at": action.created_at,
                    "amount_minor": action.action.amount.amount_minor,
                    "merchant": action.action.counterparty.and_then(|counterparty| counterparty.display_name),
                })
            })
            .collect();
        Ok(serde_json::Value::Array(entries))
    }

    async fn pay_execution_outcome(
        &self,
        service: &FinancialAuthorizationService,
        workspace_id: &str,
        action: tl_core::FinancialActionRecord,
    ) -> Result<serde_json::Value, String> {
        match action.status {
            FinancialActionStatus::Executed => Ok(json!({
                "status": "executed",
                "decision_id": action.id,
                "provider_response": latest_provider_response(service, workspace_id, &action.id)
                    .await
                    .unwrap_or(serde_json::Value::Null),
            })),
            FinancialActionStatus::Failed => Ok(json!({
                "status": "allow_failed_execute",
                "reason": latest_failure_reason(service, workspace_id, &action.id)
                    .await
                    .unwrap_or_else(|| "payment execution failed".to_string()),
                "decision_id": action.id,
            })),
            FinancialActionStatus::Held => Ok(json!({
                "status": "hold",
                "decision_id": action.id,
            })),
            FinancialActionStatus::Denied => Ok(json!({
                "status": "block",
                "decision_id": action.id,
            })),
            _ => Ok(json!({
                "status": "allow_no_provider",
                "decision_id": action.id,
            })),
        }
    }
}

fn payment_action_request(
    environment_id: &str,
    request: PayRequest,
) -> CreateFinancialActionRequest {
    let mut metadata = serde_json::Map::new();
    metadata.insert("operation".into(), PAY_OPERATION.into());
    metadata.insert("environment_id".into(), environment_id.into());
    if let Some(category) = request.category {
        metadata.insert("category".into(), category.into());
    }
    CreateFinancialActionRequest {
        idempotency_key: tl_core::new_trace_id(),
        execute: false,
        action: FinancialAction {
            id: None,
            kind: tl_core::FinancialActionKind::Payment,
            principal_id: request.owner,
            amount: MoneyAmount {
                amount_minor: request.amount_minor,
                currency: "USD".into(),
            },
            counterparty: Some(CounterpartyRef {
                id: request.merchant.clone(),
                display_name: Some(request.merchant),
                kind: "merchant".into(),
                country: None,
                metadata: serde_json::json!({}),
            }),
            rail: FinancialRail::PaymentHttp,
            mandate: None,
            memo: request.memo,
            metadata: serde_json::Value::Object(metadata),
        },
        evidence: vec![],
    }
}

async fn latest_provider_response(
    service: &FinancialAuthorizationService,
    workspace_id: &str,
    action_id: &str,
) -> Option<serde_json::Value> {
    service
        .list_action_outcomes(workspace_id, action_id)
        .await
        .ok()?
        .outcomes
        .into_iter()
        .find_map(|outcome| outcome.metadata.get("provider_response").cloned())
}

async fn latest_failure_reason(
    service: &FinancialAuthorizationService,
    workspace_id: &str,
    action_id: &str,
) -> Option<String> {
    service
        .list_action_outcomes(workspace_id, action_id)
        .await
        .ok()?
        .outcomes
        .into_iter()
        .find_map(|outcome| {
            outcome
                .metadata
                .get("reason")
                .and_then(serde_json::Value::as_str)
                .map(str::to_string)
        })
}
