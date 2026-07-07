use std::sync::Arc;

use chrono::{DateTime, Datelike, Duration, TimeZone, Utc};
use tl_core::{
    ApprovalRequirement, CreateFinancialActionRequest, CreateFinancialMandateRequest,
    CreateFinancialPolicyRequest, FinancialAction, FinancialActionListResponse,
    FinancialActionOutcome, FinancialActionOutcomeStatus, FinancialActionPrecondition,
    FinancialActionRecord, FinancialActionStatus, FinancialApprovalRequestListResponse,
    FinancialApprovalRequestStatus, FinancialMandate, FinancialMandateListResponse,
    FinancialMandateStatus, FinancialOutcomeListResponse, FinancialPolicyListResponse,
    FinancialPolicyRecord, FinancialPolicySelector, FinancialRail, FinancialReceipt, PolicyAction,
    RecoveryStatus, ReversalCapability, Severity, Verdict, DEFAULT_ENVIRONMENT_ID,
};
use tl_engine::{evaluate_financial_policies, financial_matches, financial_windowed_verdict};
use tl_policy::{validate_family_policy, Action, FamilyPolicy, FinancialPolicy, FinancialWhen};

use super::{
    validation::validate_create_action, FinancialExecutionError, FinancialExecutionResult,
    FinancialExecutor, FinancialLedgerEntryKind, FinancialStore, FinancialStoreError,
};
use crate::budget_alerts::{BudgetAlertRuntime, WindowSpend};
use crate::policies::PolicyStore;
use tl_core::BudgetAlertWindow;

#[derive(Clone)]
pub struct FinancialAuthorizationService {
    store: Arc<dyn FinancialStore>,
    policy_store: Option<Arc<dyn PolicyStore>>,
    executor: Option<Arc<dyn FinancialExecutor>>,
    /// Budget alert evaluation at spend-record time. `None` = alerts
    /// off (tests, minimal wiring); the spend path is unaffected.
    budget_alerts: Option<BudgetAlertRuntime>,
}

impl FinancialAuthorizationService {
    pub fn new(store: Arc<dyn FinancialStore>) -> Self {
        Self {
            store,
            policy_store: None,
            executor: None,
            budget_alerts: None,
        }
    }

    pub fn with_policy_store(
        store: Arc<dyn FinancialStore>,
        policy_store: Arc<dyn PolicyStore>,
    ) -> Self {
        Self {
            store,
            policy_store: Some(policy_store),
            executor: None,
            budget_alerts: None,
        }
    }

    pub fn with_policy_store_and_executor(
        store: Arc<dyn FinancialStore>,
        policy_store: Arc<dyn PolicyStore>,
        executor: Arc<dyn FinancialExecutor>,
    ) -> Self {
        Self {
            store,
            policy_store: Some(policy_store),
            executor: Some(executor),
            budget_alerts: None,
        }
    }

    /// Enable budget alert evaluation after each recorded spend.
    pub fn with_budget_alerts(mut self, runtime: BudgetAlertRuntime) -> Self {
        self.budget_alerts = Some(runtime);
        self
    }

    pub async fn create_action(
        &self,
        workspace_id: &str,
        input: CreateFinancialActionRequest,
    ) -> Result<FinancialActionRecord, FinancialStoreError> {
        self.create_action_in_environment(workspace_id, DEFAULT_ENVIRONMENT_ID, input)
            .await
    }

    pub async fn create_action_in_environment(
        &self,
        workspace_id: &str,
        environment_id: &str,
        input: CreateFinancialActionRequest,
    ) -> Result<FinancialActionRecord, FinancialStoreError> {
        validate_create_action(&input)?;
        let should_execute = input.execute;
        let action = self.store.create_action(workspace_id, input).await?;
        if action.status != FinancialActionStatus::Proposed {
            return Ok(action);
        }
        let action = self.enforce_mandate(workspace_id, action).await?;
        if action.status != FinancialActionStatus::Proposed {
            return Ok(action);
        }
        let action = self
            .apply_financial_policies(workspace_id, environment_id, action)
            .await?;
        if should_execute && action.status == FinancialActionStatus::Proposed {
            let authorized = self.authorize_action(workspace_id, &action.id).await?;
            return self.execute_action(workspace_id, &authorized.id).await;
        }
        Ok(action)
    }

    pub async fn get_action(
        &self,
        workspace_id: &str,
        action_id: &str,
    ) -> Result<FinancialActionRecord, FinancialStoreError> {
        self.store.get_action(workspace_id, action_id).await
    }

    pub async fn list_actions(
        &self,
        workspace_id: &str,
    ) -> Result<FinancialActionListResponse, FinancialStoreError> {
        self.store.list_actions(workspace_id).await
    }

    pub async fn create_financial_policy(
        &self,
        workspace_id: &str,
        environment_id: &str,
        input: CreateFinancialPolicyRequest,
    ) -> Result<FinancialPolicyRecord, FinancialStoreError> {
        let policy = financial_policy_from_request(input)?;
        let family = FamilyPolicy::Financial(policy.clone());
        if let Err(issues) = validate_family_policy(&family) {
            let message = issues
                .into_iter()
                .map(|issue| format!("{}: {}", issue.path, issue.message))
                .collect::<Vec<_>>()
                .join("; ");
            return Err(FinancialStoreError::Validation(message));
        }
        let source_yaml = serde_yaml::to_string(&family)
            .map_err(|e| FinancialStoreError::Internal(format!("financial policy yaml: {e}")))?;
        self.policy_store()?
            .upsert_family(workspace_id, environment_id, &family, &source_yaml)
            .await
            .map_err(|e| FinancialStoreError::Internal(format!("financial policy upsert: {e}")))?;
        Ok(financial_policy_record(&policy, true))
    }

    pub async fn list_financial_policies(
        &self,
        workspace_id: &str,
        environment_id: &str,
    ) -> Result<FinancialPolicyListResponse, FinancialStoreError> {
        let families = self
            .policy_store()?
            .list_enabled_families(workspace_id, environment_id)
            .await
            .map_err(|e| FinancialStoreError::Internal(format!("financial policy list: {e}")))?;
        let policies = families
            .iter()
            .filter_map(|family| match family.as_ref() {
                FamilyPolicy::Financial(policy) => Some(financial_policy_record(policy, true)),
                _ => None,
            })
            .collect();
        Ok(FinancialPolicyListResponse { policies })
    }

    pub async fn create_mandate(
        &self,
        workspace_id: &str,
        input: CreateFinancialMandateRequest,
    ) -> Result<FinancialMandate, FinancialStoreError> {
        self.store.create_mandate(workspace_id, input).await
    }

    pub async fn list_mandates(
        &self,
        workspace_id: &str,
    ) -> Result<FinancialMandateListResponse, FinancialStoreError> {
        self.store.list_mandates(workspace_id).await
    }

    pub async fn get_mandate(
        &self,
        workspace_id: &str,
        mandate_id: &str,
        version: Option<i32>,
    ) -> Result<FinancialMandate, FinancialStoreError> {
        self.store
            .get_mandate(workspace_id, mandate_id, version)
            .await
    }

    pub async fn revoke_mandate(
        &self,
        workspace_id: &str,
        mandate_id: &str,
    ) -> Result<FinancialMandate, FinancialStoreError> {
        self.store.revoke_mandate(workspace_id, mandate_id).await
    }

    pub async fn get_receipt(
        &self,
        workspace_id: &str,
        receipt_id: &str,
    ) -> Result<FinancialReceipt, FinancialStoreError> {
        self.store.get_receipt(workspace_id, receipt_id).await
    }

    pub async fn record_action_outcome(
        &self,
        workspace_id: &str,
        action_id: &str,
        outcome: FinancialActionOutcome,
    ) -> Result<FinancialActionOutcome, FinancialStoreError> {
        self.store
            .record_action_outcome(workspace_id, action_id, outcome)
            .await
    }

    pub async fn list_action_outcomes(
        &self,
        workspace_id: &str,
        action_id: &str,
    ) -> Result<FinancialOutcomeListResponse, FinancialStoreError> {
        self.store
            .list_action_outcomes(workspace_id, action_id)
            .await
    }

    pub async fn list_approval_requests(
        &self,
        workspace_id: &str,
    ) -> Result<FinancialApprovalRequestListResponse, FinancialStoreError> {
        self.store.list_approval_requests(workspace_id).await
    }

    pub async fn hold_action(
        &self,
        workspace_id: &str,
        action_id: &str,
        approval: ApprovalRequirement,
    ) -> Result<FinancialActionRecord, FinancialStoreError> {
        let held = self
            .transition_action(
                workspace_id,
                action_id,
                FinancialActionStatus::Held,
                "approval_required",
            )
            .await?;
        self.record_action_ledger_entry(
            workspace_id,
            &held,
            FinancialLedgerEntryKind::Reserved,
            "reserved",
        )
        .await?;
        self.store
            .create_approval_request(workspace_id, action_id, approval)
            .await?;
        Ok(held)
    }

    pub async fn approve_action(
        &self,
        workspace_id: &str,
        action_id: &str,
    ) -> Result<FinancialActionRecord, FinancialStoreError> {
        self.approve_action_as(workspace_id, action_id, None).await
    }

    pub async fn approve_action_as(
        &self,
        workspace_id: &str,
        action_id: &str,
        actor_id: Option<&str>,
    ) -> Result<FinancialActionRecord, FinancialStoreError> {
        let approved = self
            .transition_action(
                workspace_id,
                action_id,
                FinancialActionStatus::Authorized,
                "approved",
            )
            .await?;
        self.store
            .resolve_pending_approval_requests(
                workspace_id,
                action_id,
                FinancialApprovalRequestStatus::Approved,
                actor_id,
            )
            .await?;
        Ok(approved)
    }

    pub async fn authorize_action(
        &self,
        workspace_id: &str,
        action_id: &str,
    ) -> Result<FinancialActionRecord, FinancialStoreError> {
        let approved = self
            .transition_action(
                workspace_id,
                action_id,
                FinancialActionStatus::Authorized,
                "authorized",
            )
            .await?;
        Ok(approved)
    }

    pub async fn execute_held_action_retryable(
        &self,
        workspace_id: &str,
        action_id: &str,
    ) -> Result<FinancialActionExecutionAttempt, FinancialStoreError> {
        let current = self.store.get_action(workspace_id, action_id).await?;
        if current.status == FinancialActionStatus::Executed {
            return Ok(FinancialActionExecutionAttempt::Executed(current));
        }
        if current.status != FinancialActionStatus::Held {
            let executed = self.execute_action(workspace_id, action_id).await?;
            return Ok(FinancialActionExecutionAttempt::Executed(executed));
        }

        let provider_result = match self
            .execute_provider_if_required(workspace_id, &current)
            .await
        {
            Ok(result) => result,
            Err(reason) => {
                self.record_provider_failure(workspace_id, &current, reason.clone())
                    .await?;
                return Ok(FinancialActionExecutionAttempt::Failed {
                    action: current,
                    reason,
                });
            }
        };
        let executed = self
            .transition_action(
                workspace_id,
                action_id,
                FinancialActionStatus::Executed,
                "executed",
            )
            .await?;
        let mut ledger_event_ids = Vec::new();
        if self
            .ledger_entry_exists(workspace_id, &current, "reserved")
            .await?
        {
            ledger_event_ids.push(
                self.record_action_ledger_entry(
                    workspace_id,
                    &current,
                    FinancialLedgerEntryKind::Released,
                    "released",
                )
                .await?,
            );
        }
        ledger_event_ids.push(
            self.record_action_ledger_entry(
                workspace_id,
                &executed,
                FinancialLedgerEntryKind::Executed,
                "executed",
            )
            .await?,
        );
        self.store
            .resolve_pending_approval_requests(
                workspace_id,
                action_id,
                FinancialApprovalRequestStatus::Approved,
                None,
            )
            .await?;
        self.create_execution_receipt(
            workspace_id,
            DEFAULT_ENVIRONMENT_ID,
            &executed,
            &ledger_event_ids,
            &provider_result,
        )
        .await?;
        if let Some(provider_result) = provider_result {
            self.record_provider_success(workspace_id, &executed, provider_result)
                .await?;
        }
        Ok(FinancialActionExecutionAttempt::Executed(executed))
    }

    pub async fn deny_action(
        &self,
        workspace_id: &str,
        action_id: &str,
    ) -> Result<FinancialActionRecord, FinancialStoreError> {
        self.deny_action_as(workspace_id, action_id, None).await
    }

    pub async fn deny_action_as(
        &self,
        workspace_id: &str,
        action_id: &str,
        actor_id: Option<&str>,
    ) -> Result<FinancialActionRecord, FinancialStoreError> {
        let current = self.store.get_action(workspace_id, action_id).await?;
        let denied = self
            .transition_action(
                workspace_id,
                action_id,
                FinancialActionStatus::Denied,
                "denied",
            )
            .await?;
        self.store
            .resolve_pending_approval_requests(
                workspace_id,
                action_id,
                FinancialApprovalRequestStatus::Denied,
                actor_id,
            )
            .await?;
        if current.status == FinancialActionStatus::Held {
            self.record_action_ledger_entry(
                workspace_id,
                &current,
                FinancialLedgerEntryKind::Released,
                "released",
            )
            .await?;
        }
        Ok(denied)
    }

    pub async fn execute_action(
        &self,
        workspace_id: &str,
        action_id: &str,
    ) -> Result<FinancialActionRecord, FinancialStoreError> {
        let current = self.store.get_action(workspace_id, action_id).await?;
        match current.status {
            FinancialActionStatus::Authorized => {}
            FinancialActionStatus::Held => {
                return Err(FinancialStoreError::Validation(
                    "financial action requires approval before execution".into(),
                ));
            }
            FinancialActionStatus::Proposed => {
                return Err(FinancialStoreError::Validation(
                    "financial action requires authorization before execution".into(),
                ));
            }
            FinancialActionStatus::Executed
            | FinancialActionStatus::Denied
            | FinancialActionStatus::Failed
            | FinancialActionStatus::Reversed
            | FinancialActionStatus::Expired => {
                return Err(FinancialStoreError::Validation(format!(
                    "financial action with status `{}` cannot be executed",
                    financial_status_label(current.status)
                )));
            }
        }
        let provider_result = match self
            .execute_provider_if_required(workspace_id, &current)
            .await
        {
            Ok(result) => result,
            Err(reason) => {
                if self
                    .ledger_entry_exists(workspace_id, &current, "reserved")
                    .await?
                {
                    self.record_action_ledger_entry(
                        workspace_id,
                        &current,
                        FinancialLedgerEntryKind::Released,
                        "released",
                    )
                    .await?;
                }
                let failed = self
                    .transition_action(
                        workspace_id,
                        action_id,
                        FinancialActionStatus::Failed,
                        "provider_failed",
                    )
                    .await?;
                self.record_provider_failure(workspace_id, &failed, reason)
                    .await?;
                return Ok(failed);
            }
        };
        let executed = self
            .transition_action(
                workspace_id,
                action_id,
                FinancialActionStatus::Executed,
                "executed",
            )
            .await?;
        let mut ledger_event_ids = Vec::new();
        if self
            .ledger_entry_exists(workspace_id, &current, "reserved")
            .await?
        {
            ledger_event_ids.push(
                self.record_action_ledger_entry(
                    workspace_id,
                    &current,
                    FinancialLedgerEntryKind::Released,
                    "released",
                )
                .await?,
            );
        }
        ledger_event_ids.push(
            self.record_action_ledger_entry(
                workspace_id,
                &executed,
                FinancialLedgerEntryKind::Executed,
                "executed",
            )
            .await?,
        );
        // Budget alert thresholds are checked right after the spend
        // lands in the ledger. Never fails or delays the spend beyond
        // one indexed config lookup — errors are logged and swallowed.
        self.notify_budget_alerts(workspace_id, &executed).await;
        if current.status == FinancialActionStatus::Held {
            self.store
                .resolve_pending_approval_requests(
                    workspace_id,
                    action_id,
                    FinancialApprovalRequestStatus::Approved,
                    None,
                )
                .await?;
        }
        self.create_execution_receipt(
            workspace_id,
            DEFAULT_ENVIRONMENT_ID,
            &executed,
            &ledger_event_ids,
            &provider_result,
        )
        .await?;
        if let Some(provider_result) = provider_result {
            self.record_provider_success(workspace_id, &executed, provider_result)
                .await?;
        }
        Ok(executed)
    }

    async fn transition_action(
        &self,
        workspace_id: &str,
        action_id: &str,
        status: FinancialActionStatus,
        event_type: &str,
    ) -> Result<FinancialActionRecord, FinancialStoreError> {
        self.store
            .transition_action(workspace_id, action_id, status, event_type)
            .await
    }

    /// Spend-time budget alert hook. Evaluates enabled alert configs
    /// against the acting principal's ledger window sums and the caps
    /// from the matching financial policies. Infallible by design:
    /// alerting must never fail a spend, so every error path is
    /// `tracing::error!` + return.
    async fn notify_budget_alerts(&self, workspace_id: &str, action: &FinancialActionRecord) {
        let Some(runtime) = &self.budget_alerts else {
            return;
        };
        // One indexed lookup; almost always zero rows → early return.
        let configs = match runtime.store.list_enabled_configs(workspace_id).await {
            Ok(configs) => configs,
            Err(error) => {
                tracing::error!(workspace_id, error = %error, "budget alert config lookup failed");
                return;
            }
        };
        if configs.is_empty() {
            return;
        }
        let Some(policy_store) = &self.policy_store else {
            return;
        };
        // Caps come from the same policies the hard limits enforce
        // (execute runs in the default environment, like receipts).
        let families = match policy_store
            .list_enabled_families(workspace_id, DEFAULT_ENVIRONMENT_ID)
            .await
        {
            Ok(families) => families,
            Err(error) => {
                tracing::error!(workspace_id, error = %error, "budget alert policy lookup failed");
                return;
            }
        };
        let (daily_cap, weekly_cap, monthly_cap) =
            crate::budget_alerts::min_window_caps(families.iter().filter_map(|family| {
                match family.as_ref() {
                    FamilyPolicy::Financial(financial)
                        if financial_matches(financial, &action.action) =>
                    {
                        Some(financial)
                    }
                    _ => None,
                }
            }));
        let now = Utc::now();
        let Some((day_start, week_start, month_start)) = crate::budget_alerts::window_starts(now)
        else {
            return;
        };
        let principal_id = &action.action.principal_id;
        let currency = &action.action.amount.currency;
        let mut windows = Vec::new();
        for (window, window_start, cap) in [
            (BudgetAlertWindow::Day, day_start, daily_cap),
            (BudgetAlertWindow::Week, week_start, weekly_cap),
            (BudgetAlertWindow::Month, month_start, monthly_cap),
        ] {
            // Only sum windows that have both a cap and a config
            // watching them.
            let Some(cap_minor) = cap else { continue };
            if !configs.iter().any(|config| config.window == window) {
                continue;
            }
            match self
                .store
                .net_spend_minor(workspace_id, principal_id, currency, window_start, now)
                .await
            {
                Ok(spent_minor) => windows.push(WindowSpend {
                    window,
                    window_start,
                    cap_minor,
                    spent_minor,
                }),
                Err(error) => {
                    tracing::error!(workspace_id, error = %error, "budget alert spend sum failed");
                }
            }
        }
        if windows.is_empty() {
            return;
        }
        crate::budget_alerts::process_spend(
            runtime,
            workspace_id,
            principal_id,
            currency,
            &configs,
            &windows,
        )
        .await;
    }

    async fn record_action_ledger_entry(
        &self,
        workspace_id: &str,
        action: &FinancialActionRecord,
        kind: FinancialLedgerEntryKind,
        suffix: &str,
    ) -> Result<String, FinancialStoreError> {
        self.store
            .record_ledger_entry(
                workspace_id,
                &action.id,
                kind,
                action.action.amount.amount_minor,
                &action.action.amount.currency,
                &ledger_idempotency_key(&action.id, suffix),
                serde_json::json!({
                    "action_id": action.id,
                    "financial_status": action.status,
                    "source": "financial_authorization_service"
                }),
            )
            .await
    }

    async fn ledger_entry_exists(
        &self,
        workspace_id: &str,
        action: &FinancialActionRecord,
        suffix: &str,
    ) -> Result<bool, FinancialStoreError> {
        self.store
            .ledger_entry_exists(workspace_id, &ledger_idempotency_key(&action.id, suffix))
            .await
    }

    async fn create_execution_receipt(
        &self,
        workspace_id: &str,
        environment_id: &str,
        action: &FinancialActionRecord,
        ledger_event_ids: &[String],
        provider_result: &Option<FinancialExecutionResult>,
    ) -> Result<FinancialReceipt, FinancialStoreError> {
        let proof = self
            .execution_receipt_proof(
                workspace_id,
                environment_id,
                action,
                ledger_event_ids,
                provider_result,
            )
            .await?;
        self.store
            .create_receipt(
                workspace_id,
                &action.id,
                None,
                ledger_event_ids.to_vec(),
                proof,
            )
            .await
    }

    async fn execution_receipt_proof(
        &self,
        workspace_id: &str,
        environment_id: &str,
        action: &FinancialActionRecord,
        ledger_event_ids: &[String],
        provider_result: &Option<FinancialExecutionResult>,
    ) -> Result<serde_json::Value, FinancialStoreError> {
        let mandate_snapshot = match &action.action.mandate {
            Some(reference) => Some(
                self.store
                    .get_mandate(workspace_id, &reference.id, reference.version)
                    .await?,
            ),
            None => None,
        };
        let approval_requests = self
            .store
            .list_approval_requests(workspace_id)
            .await?
            .approval_requests
            .into_iter()
            .filter(|request| request.action_id == action.id)
            .collect::<Vec<_>>();
        let policy_snapshots = self
            .matching_policy_snapshots(workspace_id, environment_id, &action.action)
            .await?;

        Ok(serde_json::json!({
            "schema": "financial_execution_receipt.v1",
            "action_id": action.id,
            "action_status": "executed",
            "action_snapshot": action.action,
            "amount": action.action.amount,
            "counterparty": action.action.counterparty,
            "mandate_ref": action.action.mandate,
            "mandate_snapshot": mandate_snapshot,
            "approval_requests": approval_requests,
            "evidence_refs": action.evidence,
            "policy_snapshots": policy_snapshots,
            "ledger_source": "financial_ledger_entries",
            "ledger_event_ids": ledger_event_ids,
            "provider": provider_proof(provider_result),
            "receipt_source": "financial_authorization_service"
        }))
    }

    async fn matching_policy_snapshots(
        &self,
        workspace_id: &str,
        environment_id: &str,
        action: &FinancialAction,
    ) -> Result<Vec<serde_json::Value>, FinancialStoreError> {
        let Some(policy_store) = &self.policy_store else {
            return Ok(vec![]);
        };
        let families = policy_store
            .list_enabled_families(workspace_id, environment_id)
            .await
            .map_err(|e| FinancialStoreError::Internal(format!("financial policies: {e}")))?;
        Ok(families
            .iter()
            .filter(|family| receipt_policy_matches(family.as_ref(), action))
            .map(|family| {
                serde_json::json!({
                    "id": family.id(),
                    "policy": family.as_ref()
                })
            })
            .collect())
    }

    fn policy_store(&self) -> Result<&Arc<dyn PolicyStore>, FinancialStoreError> {
        self.policy_store.as_ref().ok_or_else(|| {
            FinancialStoreError::Internal("financial policy store unavailable".into())
        })
    }

    async fn execute_provider_if_required(
        &self,
        workspace_id: &str,
        action: &FinancialActionRecord,
    ) -> Result<Option<FinancialExecutionResult>, String> {
        if action.action.rail != FinancialRail::PaymentHttp {
            return Ok(None);
        }
        let Some(executor) = &self.executor else {
            return Ok(None);
        };
        executor
            .execute(workspace_id, action, &action.id)
            .await
            .map(Some)
            .map_err(|error| match error {
                FinancialExecutionError::NoProvider => error.to_string(),
                FinancialExecutionError::Failed(reason) => reason,
            })
    }

    async fn record_provider_success(
        &self,
        workspace_id: &str,
        action: &FinancialActionRecord,
        result: FinancialExecutionResult,
    ) -> Result<(), FinancialStoreError> {
        self.store
            .record_action_outcome(
                workspace_id,
                &action.id,
                FinancialActionOutcome {
                    action_id: action.id.clone(),
                    status: FinancialActionOutcomeStatus::Succeeded,
                    reversal_capability: result.reversal_capability,
                    recovery_status: result.recovery_status,
                    provider_status: result.provider_status,
                    provider_reference: result.provider_reference,
                    final_loss_amount: None,
                    occurred_at: Utc::now().to_rfc3339(),
                    metadata: serde_json::json!({
                        "provider_response": result.provider_response,
                        "source": "financial_authorization_service"
                    }),
                },
            )
            .await?;
        Ok(())
    }

    async fn record_provider_failure(
        &self,
        workspace_id: &str,
        action: &FinancialActionRecord,
        reason: String,
    ) -> Result<(), FinancialStoreError> {
        self.store
            .record_action_outcome(
                workspace_id,
                &action.id,
                FinancialActionOutcome {
                    action_id: action.id.clone(),
                    status: FinancialActionOutcomeStatus::Failed,
                    reversal_capability: ReversalCapability::None,
                    recovery_status: RecoveryStatus::NotAvailable,
                    provider_status: Some("failed".into()),
                    provider_reference: None,
                    final_loss_amount: None,
                    occurred_at: Utc::now().to_rfc3339(),
                    metadata: serde_json::json!({
                        "reason": reason,
                        "source": "financial_authorization_service"
                    }),
                },
            )
            .await?;
        Ok(())
    }

    async fn enforce_mandate(
        &self,
        workspace_id: &str,
        action: FinancialActionRecord,
    ) -> Result<FinancialActionRecord, FinancialStoreError> {
        let Some(reference) = &action.action.mandate else {
            return Ok(action);
        };
        let mandate = match self
            .store
            .get_mandate(workspace_id, &reference.id, reference.version)
            .await
        {
            Ok(mandate) => mandate,
            Err(FinancialStoreError::NotFound) => {
                return self
                    .transition_action(
                        workspace_id,
                        &action.id,
                        FinancialActionStatus::Denied,
                        "mandate_not_found",
                    )
                    .await;
            }
            Err(error) => return Err(error),
        };

        if let Some(reason) = mandate_denial_reason(&mandate, &action)? {
            return self
                .transition_action(
                    workspace_id,
                    &action.id,
                    FinancialActionStatus::Denied,
                    &reason,
                )
                .await;
        }
        Ok(action)
    }

    async fn apply_financial_policies(
        &self,
        workspace_id: &str,
        environment_id: &str,
        action: FinancialActionRecord,
    ) -> Result<FinancialActionRecord, FinancialStoreError> {
        let Some(policy_store) = &self.policy_store else {
            return Ok(action);
        };
        let families = policy_store
            .list_enabled_families(workspace_id, environment_id)
            .await
            .map_err(|e| FinancialStoreError::Internal(format!("financial policies: {e}")))?;
        let pure = evaluate_financial_policies(&action.action, families.iter().map(Arc::as_ref));
        let windowed = self
            .evaluate_ledger_windows(workspace_id, &action, &families)
            .await?;
        let eligibility = financial_eligibility_decision(&action, &families);

        let mut decision = compose_policy_decisions(
            pure.verdict.map(|verdict| {
                (
                    verdict,
                    pure.reason
                        .unwrap_or_else(|| "financial policy matched".to_string()),
                )
            }),
            windowed,
        );
        decision = compose_policy_decisions(decision, eligibility);

        match decision {
            Some((Verdict::Block | Verdict::Rewrite, reason)) => {
                self.store
                    .transition_action_with_reason(
                        workspace_id,
                        &action.id,
                        FinancialActionStatus::Denied,
                        "policy_denied",
                        &reason,
                    )
                    .await
            }
            Some((Verdict::Escalate, reason)) => {
                let approver_roles = financial_approver_roles(&families, &action.action);
                self.hold_action(
                    workspace_id,
                    &action.id,
                    ApprovalRequirement {
                        required: true,
                        approver_roles,
                        reason,
                        expires_at: None,
                    },
                )
                .await
            }
            Some((Verdict::Allow, _)) | None => Ok(action),
        }
    }

    async fn evaluate_ledger_windows(
        &self,
        workspace_id: &str,
        action: &FinancialActionRecord,
        families: &[Arc<FamilyPolicy>],
    ) -> Result<Option<(Verdict, String)>, FinancialStoreError> {
        let now = Utc::now();
        let day_start = Utc
            .with_ymd_and_hms(now.year(), now.month(), now.day(), 0, 0, 0)
            .single()
            .ok_or_else(|| FinancialStoreError::Internal("invalid day window".into()))?;
        // ponytail: week starts Monday UTC; make configurable if a customer asks
        let days_from_monday = i64::from(now.weekday().num_days_from_monday());
        let week_start = day_start - Duration::days(days_from_monday);
        let month_start = Utc
            .with_ymd_and_hms(now.year(), now.month(), 1, 0, 0, 0)
            .single()
            .ok_or_else(|| FinancialStoreError::Internal("invalid month window".into()))?;
        let mut decision = None;
        let mut spend_cache = None;
        for family in families {
            let FamilyPolicy::Financial(financial) = family.as_ref() else {
                continue;
            };
            if !financial_matches(financial, &action.action) {
                continue;
            }
            if financial.daily_minor.is_none()
                && financial.weekly_minor.is_none()
                && financial.monthly_minor.is_none()
            {
                continue;
            }
            let (spent_today, spent_week, spent_month) = match spend_cache {
                Some(spend) => spend,
                None => {
                    let spend = (
                        self.store
                            .net_spend_minor(
                                workspace_id,
                                &action.action.principal_id,
                                &action.action.amount.currency,
                                day_start,
                                now,
                            )
                            .await?,
                        self.store
                            .net_spend_minor(
                                workspace_id,
                                &action.action.principal_id,
                                &action.action.amount.currency,
                                week_start,
                                now,
                            )
                            .await?,
                        self.store
                            .net_spend_minor(
                                workspace_id,
                                &action.action.principal_id,
                                &action.action.amount.currency,
                                month_start,
                                now,
                            )
                            .await?,
                    );
                    spend_cache = Some(spend);
                    spend
                }
            };
            let next = financial_windowed_verdict(
                financial,
                spent_today,
                spent_week,
                spent_month,
                action.action.amount.amount_minor,
            );
            let Some(next) = next else { continue };
            decision = compose_policy_decisions(decision, Some(next));
        }
        Ok(decision)
    }
}

pub enum FinancialActionExecutionAttempt {
    Executed(FinancialActionRecord),
    Failed {
        action: FinancialActionRecord,
        reason: String,
    },
}

fn compose_policy_decisions(
    current: Option<(Verdict, String)>,
    next: Option<(Verdict, String)>,
) -> Option<(Verdict, String)> {
    match (current, next) {
        (None, decision) | (decision, None) => decision,
        (Some((current_verdict, current_reason)), Some((next_verdict, next_reason))) => {
            let worst = current_verdict.worst_with(next_verdict);
            if worst == next_verdict && next_verdict != current_verdict {
                Some((worst, next_reason))
            } else {
                Some((worst, current_reason))
            }
        }
    }
}

fn ledger_idempotency_key(action_id: &str, suffix: &str) -> String {
    format!("{action_id}:{suffix}")
}

fn provider_proof(result: &Option<FinancialExecutionResult>) -> serde_json::Value {
    match result {
        Some(result) => serde_json::json!({
            "status": result.provider_status,
            "reference": result.provider_reference,
            "response": result.provider_response,
            "reversal_capability": result.reversal_capability,
            "recovery_status": result.recovery_status
        }),
        None => serde_json::Value::Null,
    }
}

fn receipt_policy_matches(policy: &FamilyPolicy, action: &FinancialAction) -> bool {
    match policy {
        FamilyPolicy::Financial(financial) => financial_matches(financial, action),
        _ => false,
    }
}

fn financial_approver_roles(
    families: &[Arc<FamilyPolicy>],
    action: &FinancialAction,
) -> Vec<String> {
    let mut roles = Vec::new();
    for family in families {
        let FamilyPolicy::Financial(financial) = family.as_ref() else {
            continue;
        };
        if !financial_matches(financial, action) {
            continue;
        }
        for role in &financial.approver_roles {
            if !roles.iter().any(|existing| existing == role) {
                roles.push(role.clone());
            }
        }
    }
    roles
}

fn financial_eligibility_decision(
    action: &FinancialActionRecord,
    families: &[Arc<FamilyPolicy>],
) -> Option<(Verdict, String)> {
    let mut decision = None;
    for family in families {
        let FamilyPolicy::Financial(financial) = family.as_ref() else {
            continue;
        };
        if !financial_matches(financial, &action.action) {
            continue;
        }
        for precondition in &financial.required_preconditions {
            let key = precondition_key(*precondition);
            let next = match evidence_bool(action, key) {
                Some(true) => None,
                Some(false) => Some((
                    policy_action_verdict(financial.failed_precondition_action),
                    format!(
                        "financial policy `{}`: eligibility precondition `{key}` failed",
                        financial.id
                    ),
                )),
                None => Some((
                    policy_action_verdict(financial.missing_evidence_action),
                    format!(
                        "financial policy `{}`: missing eligibility evidence `{key}`",
                        financial.id
                    ),
                )),
            };
            decision = compose_policy_decisions(decision, next);
        }
    }
    decision
}

fn financial_status_label(status: FinancialActionStatus) -> &'static str {
    match status {
        FinancialActionStatus::Proposed => "proposed",
        FinancialActionStatus::Authorized => "authorized",
        FinancialActionStatus::Held => "held",
        FinancialActionStatus::Executed => "executed",
        FinancialActionStatus::Denied => "denied",
        FinancialActionStatus::Failed => "failed",
        FinancialActionStatus::Reversed => "reversed",
        FinancialActionStatus::Expired => "expired",
    }
}

fn financial_policy_from_request(
    input: CreateFinancialPolicyRequest,
) -> Result<FinancialPolicy, FinancialStoreError> {
    Ok(FinancialPolicy {
        id: input.id,
        description: input.description,
        severity: input.severity.unwrap_or(Severity::Medium),
        when: FinancialWhen {
            agents: input.when.agents,
            action_kinds: input.when.action_kinds,
            operations: input.when.operations,
            currencies: input.when.currencies,
            rails: input.when.rails,
        },
        per_transaction_minor: input.per_transaction_minor,
        hold_above_minor: input.hold_above_minor,
        daily_minor: input.daily_minor,
        weekly_minor: input.weekly_minor,
        monthly_minor: input.monthly_minor,
        allowed_counterparty_ids: input.allowed_counterparty_ids,
        denied_counterparty_ids: input.denied_counterparty_ids,
        hold_new_counterparty: input.hold_new_counterparty,
        mandate_required: input.mandate_required,
        approval_threshold_minor: input.approval_threshold_minor,
        approver_roles: input.approver_roles,
        refund_original_method_only: input.refund_original_method_only,
        required_preconditions: input.required_preconditions,
        missing_evidence_action: enforcing_action(
            "missing_evidence_action",
            input
                .missing_evidence_action
                .unwrap_or(PolicyAction::Escalate),
        )?,
        failed_precondition_action: enforcing_action(
            "failed_precondition_action",
            input
                .failed_precondition_action
                .unwrap_or(PolicyAction::Block),
        )?,
        on_breach: enforcing_action("on_breach", input.on_breach.unwrap_or(PolicyAction::Block))?,
    })
}

fn financial_policy_record(policy: &FinancialPolicy, enabled: bool) -> FinancialPolicyRecord {
    FinancialPolicyRecord {
        id: policy.id.clone(),
        description: policy.description.clone(),
        severity: policy.severity,
        when: FinancialPolicySelector {
            agents: policy.when.agents.clone(),
            action_kinds: policy.when.action_kinds.clone(),
            operations: policy.when.operations.clone(),
            currencies: policy.when.currencies.clone(),
            rails: policy.when.rails.clone(),
        },
        per_transaction_minor: policy.per_transaction_minor,
        hold_above_minor: policy.hold_above_minor,
        daily_minor: policy.daily_minor,
        weekly_minor: policy.weekly_minor,
        monthly_minor: policy.monthly_minor,
        allowed_counterparty_ids: policy.allowed_counterparty_ids.clone(),
        denied_counterparty_ids: policy.denied_counterparty_ids.clone(),
        hold_new_counterparty: policy.hold_new_counterparty,
        mandate_required: policy.mandate_required,
        approval_threshold_minor: policy.approval_threshold_minor,
        approver_roles: policy.approver_roles.clone(),
        refund_original_method_only: policy.refund_original_method_only,
        required_preconditions: policy.required_preconditions.clone(),
        missing_evidence_action: policy_action(policy.missing_evidence_action),
        failed_precondition_action: policy_action(policy.failed_precondition_action),
        on_breach: policy_action(policy.on_breach),
        enabled,
    }
}

fn enforcing_action(field: &str, action: PolicyAction) -> Result<Action, FinancialStoreError> {
    match action {
        PolicyAction::Block => Ok(Action::Block),
        PolicyAction::Escalate => Ok(Action::Escalate),
        PolicyAction::Rewrite => Err(FinancialStoreError::Validation(format!(
            "{field}: must be block or escalate"
        ))),
    }
}

fn policy_action(action: Action) -> PolicyAction {
    match action {
        Action::Escalate => PolicyAction::Escalate,
        Action::Allow | Action::Block | Action::Rewrite => PolicyAction::Block,
    }
}

fn evidence_bool(action: &FinancialActionRecord, key: &str) -> Option<bool> {
    action.evidence.iter().find_map(|evidence| {
        evidence
            .metadata
            .get(key)
            .and_then(serde_json::Value::as_bool)
    })
}

fn precondition_key(precondition: FinancialActionPrecondition) -> &'static str {
    match precondition {
        FinancialActionPrecondition::OrderExists => "order_exists",
        FinancialActionPrecondition::PaymentCaptured => "payment_captured",
        FinancialActionPrecondition::RefundWindowOpen => "refund_window_open",
        FinancialActionPrecondition::AmountLteRefundableBalance => "amount_lte_refundable_balance",
        FinancialActionPrecondition::DestinationIsOriginalPaymentMethod => {
            "destination_is_original_payment_method"
        }
        FinancialActionPrecondition::NoDuplicateRefund => "no_duplicate_refund",
        FinancialActionPrecondition::InvoiceMatchesPo => "invoice_matches_po",
        FinancialActionPrecondition::VendorApproved => "vendor_approved",
        FinancialActionPrecondition::MandateValid => "mandate_valid",
        FinancialActionPrecondition::Custom => "custom",
    }
}

fn policy_action_verdict(action: Action) -> Verdict {
    match action {
        Action::Block => Verdict::Block,
        Action::Escalate => Verdict::Escalate,
        Action::Allow | Action::Rewrite => Verdict::Block,
    }
}

fn mandate_denial_reason(
    mandate: &FinancialMandate,
    action: &FinancialActionRecord,
) -> Result<Option<String>, FinancialStoreError> {
    if mandate.status != FinancialMandateStatus::Active {
        return Ok(Some("mandate_inactive".into()));
    }
    if mandate.principal_id != action.action.principal_id {
        return Ok(Some("mandate_principal_mismatch".into()));
    }
    let now = Utc::now();
    if let Some(starts_at) =
        parse_optional_rfc3339("mandate starts_at", mandate.starts_at.as_deref())?
    {
        if starts_at > now {
            return Ok(Some("mandate_not_started".into()));
        }
    }
    if let Some(expires_at) =
        parse_optional_rfc3339("mandate expires_at", mandate.expires_at.as_deref())?
    {
        if expires_at <= now {
            return Ok(Some("mandate_expired".into()));
        }
    }
    mandate_scope_denial_reason(mandate, action)
}

fn mandate_scope_denial_reason(
    mandate: &FinancialMandate,
    action: &FinancialActionRecord,
) -> Result<Option<String>, FinancialStoreError> {
    if let Some(action_kinds) = mandate.scope.get("action_kinds") {
        let expected = serde_json::to_value(action.action.kind)
            .map_err(|e| FinancialStoreError::Internal(format!("action kind encode: {e}")))?;
        let expected = expected
            .as_str()
            .ok_or_else(|| FinancialStoreError::Internal("action kind encode".into()))?;
        if !json_string_array_contains(action_kinds, expected)? {
            return Ok(Some("mandate_scope_action_kind_mismatch".into()));
        }
    }

    if let Some(currency) = mandate.scope.get("currency") {
        let Some(currency) = currency.as_str() else {
            return Ok(Some("mandate_scope_currency_invalid".into()));
        };
        if !currency.eq_ignore_ascii_case(&action.action.amount.currency) {
            return Ok(Some("mandate_scope_currency_mismatch".into()));
        }
    }

    if let Some(currencies) = mandate.scope.get("currencies") {
        if !json_string_array_contains(currencies, &action.action.amount.currency)? {
            return Ok(Some("mandate_scope_currency_mismatch".into()));
        }
    }

    if let Some(max_amount) = mandate.scope.get("max_amount_minor") {
        let Some(max_amount) = max_amount.as_i64() else {
            return Ok(Some("mandate_scope_max_amount_invalid".into()));
        };
        if action.action.amount.amount_minor > max_amount {
            return Ok(Some("mandate_scope_amount_exceeded".into()));
        }
    }

    Ok(None)
}

fn parse_optional_rfc3339(
    field: &str,
    value: Option<&str>,
) -> Result<Option<DateTime<Utc>>, FinancialStoreError> {
    value
        .map(DateTime::parse_from_rfc3339)
        .transpose()
        .map_err(|e| FinancialStoreError::Validation(format!("{field}: {e}")))
        .map(|value| value.map(|dt| dt.with_timezone(&Utc)))
}

fn json_string_array_contains(
    value: &serde_json::Value,
    expected: &str,
) -> Result<bool, FinancialStoreError> {
    let Some(values) = value.as_array() else {
        return Ok(false);
    };
    for value in values {
        let Some(candidate) = value.as_str() else {
            return Err(FinancialStoreError::Validation(
                "mandate scope array values must be strings".into(),
            ));
        };
        if candidate.eq_ignore_ascii_case(expected) {
            return Ok(true);
        }
    }
    Ok(false)
}
