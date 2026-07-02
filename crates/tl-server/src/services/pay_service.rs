//! The pay gate: judge every spend through the unified event path, and on
//! ALLOW execute it against the workspace's vaulted `payment_http` provider
//! connection.
//!
//! Transport-independent — the MCP surface (`crate::pay_mcp`) is a thin shim
//! over this service, and tests drive it directly. One flow for every
//! caller: judge (`execute_event_submission`) → act (`forward_payment`),
//! the same judge-then-act composition the LLM gateway uses.

use serde_json::json;
use tl_core::{
    Action as EventAction, CreateHumanReviewEventRequest, EventKind, GatewayProviderKind,
    GuardEvent, HumanReviewOutcome, Principal, Verdict,
};
use tl_policy::{Action, FamilyPolicy, PaymentPolicy, PaymentWhen};

use crate::gateway::{forward_payment, unseal_provider_key};
use crate::services::event_service::execute_event_submission;
use crate::AppState;

/// The operation name a `pay` call submits, and the one audit filters on.
pub const PAY_OPERATION: &str = "pay";

/// Domain stamped on hold-execution traces so they read distinctly in the
/// audit trail while still counting toward windowed spend.
const PAY_EXECUTION_DOMAIN: &str = "payment.execution";

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
/// client for provider forwards. Cheap to clone.
#[derive(Clone)]
pub struct PayGate {
    pub state: AppState,
    pub seal_key: [u8; 32],
    pub http: reqwest::Client,
}

impl PayGate {
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
        let mut parameters = serde_json::Map::new();
        parameters.insert("amount".into(), request.amount_minor.into());
        parameters.insert("merchant".into(), request.merchant.into());
        if let Some(category) = request.category {
            parameters.insert("category".into(), category.into());
        }
        if let Some(memo) = request.memo {
            parameters.insert("memo".into(), memo.into());
        }
        let parameters = serde_json::Value::Object(parameters);
        let event = GuardEvent {
            kind: EventKind::ToolCallProposed,
            principal: Principal {
                workspace_id: workspace_id.to_string(),
                environment_id: environment_id.to_string(),
                agent_id: request.owner,
                user_id: None,
                session_id: None,
                task_id: None,
                run_id: None,
                run_event_id: None,
            },
            action: EventAction {
                operation: PAY_OPERATION.to_string(),
                parameters: parameters.clone(),
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
            workspace_id,
            environment_id,
            event,
            std::time::Instant::now(),
        )
        .await
        .map_err(|_| "payment evaluation failed".to_string())?;

        let status = match decision.verdict {
            // Judge said yes — the inline half: forward with the vaulted
            // credential.
            Verdict::Allow => match self.payment_provider(workspace_id).await {
                Ok(Some((connection, api_key))) => {
                    match forward_payment(
                        &self.http,
                        &connection,
                        &api_key,
                        &decision.trace_id,
                        &parameters,
                    )
                    .await
                    {
                        Ok(provider_response) => {
                            return Ok(json!({
                                "status": "executed",
                                "reason": decision.reason,
                                "decision_id": decision.trace_id,
                                "provider_response": provider_response,
                            }));
                        }
                        Err(reason) => {
                            tracing::error!(workspace_id, decision_id = %decision.trace_id, %reason, "payment forward failed after allow");
                            return Ok(json!({
                                "status": "allow_failed_execute",
                                "reason": reason,
                                "decision_id": decision.trace_id,
                            }));
                        }
                    }
                }
                // No vaulted credential: advice-only mode.
                Ok(None) => "allow_no_provider",
                Err(reason) => {
                    tracing::error!(workspace_id, decision_id = %decision.trace_id, %reason, "payment provider resolution failed");
                    return Ok(json!({
                        "status": "allow_failed_execute",
                        "reason": reason,
                        "decision_id": decision.trace_id,
                    }));
                }
            },
            Verdict::Escalate => "hold",
            Verdict::Block | Verdict::Rewrite => "block",
        };
        Ok(json!({
            "status": status,
            "reason": decision.reason,
            "decision_id": decision.trace_id,
        }))
    }

    /// Approve (and execute) or deny a held spend.
    pub async fn resolve_hold(
        &self,
        workspace_id: &str,
        environment_id: &str,
        decision_id: &str,
        approve: bool,
    ) -> Result<serde_json::Value, String> {
        // Double-execution guard #1: an already-accepted hold never executes
        // again (guard #2 is the provider-side Idempotency-Key).
        if approve {
            let events = self
                .state
                .human_review_store
                .list_events(workspace_id, decision_id, 50)
                .await
                .map_err(|e| format!("resolve_hold: {e}"))?;
            if events
                .iter()
                .any(|e| e.outcome == HumanReviewOutcome::Accepted)
            {
                return Ok(json!({
                    "status": "already_approved",
                    "decision_id": decision_id,
                }));
            }
        }

        let outcome = if approve {
            HumanReviewOutcome::Accepted
        } else {
            HumanReviewOutcome::Rejected
        };
        self.state
            .human_review_store
            .create_event(
                workspace_id,
                decision_id,
                CreateHumanReviewEventRequest {
                    outcome,
                    reason_codes: vec![],
                    note: None,
                    metadata: serde_json::Value::Null,
                },
                None,
            )
            .await
            .map_err(|e| format!("resolve_hold: {e}"))?;

        if !approve {
            return Ok(json!({
                "status": "denied",
                "decision_id": decision_id,
            }));
        }

        match self
            .execute_held_payment(workspace_id, environment_id, decision_id)
            .await
        {
            Ok(provider_response) => Ok(json!({
                "status": "executed",
                "decision_id": decision_id,
                "provider_response": provider_response,
            })),
            Err(reason) => {
                tracing::error!(workspace_id, decision_id, %reason, "held payment execution failed");
                Ok(json!({
                    "status": "approved_failed_execute",
                    "reason": reason,
                    "decision_id": decision_id,
                }))
            }
        }
    }

    /// The workspace's payment provider connection with its credential
    /// unsealed, or `Ok(None)` when no `payment_http` connection exists
    /// (advice-only mode).
    // ponytail: first payment connection wins — one per workspace; add the
    // gateway-routes indirection when a workspace needs several.
    async fn payment_provider(
        &self,
        workspace_id: &str,
    ) -> Result<Option<(tl_core::GatewayProviderConnection, String)>, String> {
        let connections = self
            .state
            .gateway_store
            .list_provider_connections(workspace_id)
            .await
            .map_err(|e| format!("payment connection lookup failed: {e}"))?;
        let Some(connection) = connections
            .into_iter()
            .find(|c| c.kind == GatewayProviderKind::PaymentHttp)
        else {
            return Ok(None);
        };
        let secret = self
            .state
            .gateway_store
            .get_provider_connection_secret(workspace_id, &connection.id)
            .await
            .map_err(|e| format!("payment credential lookup failed: {e}"))?;
        let api_key = unseal_provider_key(&secret.encrypted_api_key, &self.seal_key)?;
        Ok(Some((connection, api_key)))
    }

    /// Execute a previously-held payment from its recorded trace, then record
    /// the execution as an `allow` trace so it counts toward windowed spend.
    async fn execute_held_payment(
        &self,
        workspace_id: &str,
        environment_id: &str,
        decision_id: &str,
    ) -> Result<serde_json::Value, String> {
        // ponytail: holds are found in the recent-trace window; add a
        // get-by-id to TraceStore if hold volume ever outgrows 100.
        let traces = self
            .state
            .trace_store
            .list_recent(workspace_id, environment_id, None, 100)
            .await
            .map_err(|e| format!("trace lookup failed: {e}"))?;
        let trace = traces
            .into_iter()
            .find(|t| t.trace_id == decision_id)
            .ok_or_else(|| "held decision not found in recent traces".to_string())?;

        let event: GuardEvent = trace
            .payload
            .get("event")
            .cloned()
            .ok_or_else(|| "held decision has no event evidence".to_string())
            .and_then(|e| {
                serde_json::from_value(e).map_err(|e| format!("held event unreadable: {e}"))
            })?;

        // Conservative money posture (mirrors the per-call evaluator): never
        // execute a payment whose amount can't be verified.
        if event.action.operation != PAY_OPERATION {
            return Err("held decision is not a payment".to_string());
        }
        if event
            .action
            .parameters
            .get("amount")
            .and_then(serde_json::Value::as_i64)
            .is_none()
        {
            return Err(
                "held payment amount missing or non-integer — refusing to execute".to_string(),
            );
        }

        let (connection, api_key) = self
            .payment_provider(workspace_id)
            .await?
            .ok_or_else(|| "no payment provider connection configured".to_string())?;

        let provider_response = forward_payment(
            &self.http,
            &connection,
            &api_key,
            decision_id, // idempotency: same decision can never charge twice
            &event.action.parameters,
        )
        .await?;

        // The escalate trace stays as judged; the execution is its own
        // `allow` trace, which is what the windowed spend sum counts.
        let mut execution = tl_core::Decision::allow(tl_core::new_trace_id());
        execution.reason = format!("hold {decision_id} approved and executed");
        let write = crate::traces::TraceWriteRequest {
            workspace_id: workspace_id.to_string(),
            environment_id: environment_id.to_string(),
            decision: execution,
            event: Some(event),
            run_id: None,
            run_event_id: None,
            session_id: None,
            domain: PAY_EXECUTION_DOMAIN.to_string(),
        };
        if let Err(e) = self.state.trace_store.record(write).await {
            // Executed but not counted — log loudly; the cap now undercounts
            // until the trace path recovers.
            tracing::error!(workspace_id, decision_id, error = %e, "executed hold not recorded to trace history");
        }

        Ok(provider_response)
    }

    /// The owner's payment decisions (the audit trail).
    pub async fn export_audit(
        &self,
        workspace_id: &str,
        environment_id: &str,
        owner: &str,
    ) -> Result<serde_json::Value, String> {
        let traces = self
            .state
            .trace_store
            .list_recent(workspace_id, environment_id, None, 100)
            .await
            .map_err(|e| format!("export_audit: {e}"))?;
        let entries: Vec<_> = traces
            .into_iter()
            .filter(|t| is_payment_for_owner(&t.payload, owner))
            .map(|t| {
                json!({
                    "decision_id": t.trace_id,
                    "decision": t.decision,
                    "created_at": t.created_at,
                    "amount_minor": payment_field(&t.payload, "amount"),
                    "merchant": payment_field(&t.payload, "merchant"),
                })
            })
            .collect();
        Ok(serde_json::Value::Array(entries))
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
