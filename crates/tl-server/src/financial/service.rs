use std::sync::Arc;

use chrono::{DateTime, Datelike, TimeZone, Utc};
use tl_core::{
    ApprovalRequirement, CreateFinancialActionRequest, CreateFinancialMandateRequest,
    FinancialAction, FinancialActionListResponse, FinancialActionOutcome,
    FinancialActionOutcomeStatus, FinancialActionRecord, FinancialActionStatus,
    FinancialApprovalRequestListResponse, FinancialApprovalRequestStatus, FinancialMandate,
    FinancialMandateListResponse, FinancialMandateStatus, FinancialOutcomeListResponse,
    FinancialRail, FinancialReceipt, RecoveryStatus, ReversalCapability, Verdict,
    DEFAULT_ENVIRONMENT_ID,
};
use tl_engine::{evaluate_financial_policies, financial_matches, financial_windowed_verdict};
use tl_policy::{Action, FamilyPolicy, PaymentPolicy};

use super::{
    validation::validate_create_action, FinancialExecutionError, FinancialExecutionResult,
    FinancialExecutor, FinancialLedgerEntryKind, FinancialStore, FinancialStoreError,
};
use crate::policies::PolicyStore;

#[derive(Clone)]
pub struct FinancialAuthorizationService {
    store: Arc<dyn FinancialStore>,
    policy_store: Option<Arc<dyn PolicyStore>>,
    executor: Option<Arc<dyn FinancialExecutor>>,
}

impl FinancialAuthorizationService {
    pub fn new(store: Arc<dyn FinancialStore>) -> Self {
        Self {
            store,
            policy_store: None,
            executor: None,
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
        }
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
        if !matches!(
            current.status,
            FinancialActionStatus::Held | FinancialActionStatus::Authorized
        ) {
            return self
                .transition_action(
                    workspace_id,
                    action_id,
                    FinancialActionStatus::Executed,
                    "executed",
                )
                .await;
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
        for family in &families {
            let FamilyPolicy::Payment(payment) = family.as_ref() else {
                continue;
            };
            if !legacy_payment_matches(payment, &action.action) {
                continue;
            }
            decision = compose_policy_decisions(
                decision,
                legacy_payment_per_action_decision(payment, &action.action),
            );
        }

        match decision {
            Some((Verdict::Block | Verdict::Rewrite, _reason)) => {
                self.transition_action(
                    workspace_id,
                    &action.id,
                    FinancialActionStatus::Denied,
                    "policy_denied",
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
        let month_start = Utc
            .with_ymd_and_hms(now.year(), now.month(), 1, 0, 0, 0)
            .single()
            .ok_or_else(|| FinancialStoreError::Internal("invalid month window".into()))?;
        let mut decision = None;
        for family in families {
            let windowed = match family.as_ref() {
                FamilyPolicy::Financial(financial) => {
                    if !financial_matches(financial, &action.action) {
                        continue;
                    }
                    if financial.daily_minor.is_none() && financial.monthly_minor.is_none() {
                        continue;
                    }
                    LegacyOrFinancialWindow::Financial(financial)
                }
                FamilyPolicy::Payment(payment) => {
                    if !legacy_payment_matches(payment, &action.action) {
                        continue;
                    }
                    if payment.daily_minor.is_none() && payment.monthly_minor.is_none() {
                        continue;
                    }
                    LegacyOrFinancialWindow::Payment(payment)
                }
                _ => continue,
            };
            let spent_today = self
                .store
                .net_spend_minor(
                    workspace_id,
                    &action.action.principal_id,
                    &action.action.amount.currency,
                    day_start,
                    now,
                )
                .await?;
            let spent_month = self
                .store
                .net_spend_minor(
                    workspace_id,
                    &action.action.principal_id,
                    &action.action.amount.currency,
                    month_start,
                    now,
                )
                .await?;
            let next = match windowed {
                LegacyOrFinancialWindow::Financial(financial) => financial_windowed_verdict(
                    financial,
                    spent_today,
                    spent_month,
                    action.action.amount.amount_minor,
                ),
                LegacyOrFinancialWindow::Payment(payment) => legacy_payment_windowed_decision(
                    payment,
                    spent_today,
                    spent_month,
                    action.action.amount.amount_minor,
                ),
            };
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

enum LegacyOrFinancialWindow<'a> {
    Financial(&'a tl_policy::FinancialPolicy),
    Payment(&'a PaymentPolicy),
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
        FamilyPolicy::Payment(payment) => legacy_payment_matches(payment, action),
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

fn legacy_payment_matches(policy: &PaymentPolicy, action: &FinancialAction) -> bool {
    if !policy.when.agents.is_empty()
        && !policy
            .when
            .agents
            .iter()
            .any(|agent| agent == &action.principal_id)
    {
        return false;
    }
    if policy.when.operations.is_empty() {
        return false;
    }
    let Some(operation) = action
        .metadata
        .get("operation")
        .and_then(serde_json::Value::as_str)
    else {
        return false;
    };
    policy.when.operations.iter().any(|op| op == operation)
}

fn legacy_payment_per_action_decision(
    policy: &PaymentPolicy,
    action: &FinancialAction,
) -> Option<(Verdict, String)> {
    if let Some(cap) = policy.per_transaction_minor {
        if action.amount.amount_minor > cap {
            return Some((
                policy_action_verdict(policy.on_breach),
                format!(
                    "payment policy `{}`: amount {} over per-transaction cap {cap}",
                    policy.id, action.amount.amount_minor
                ),
            ));
        }
    }
    if let Some(threshold) = policy.hold_above_minor {
        if action.amount.amount_minor >= threshold {
            return Some((
                Verdict::Escalate,
                format!(
                    "payment policy `{}`: amount {} at or above hold threshold {threshold}",
                    policy.id, action.amount.amount_minor
                ),
            ));
        }
    }
    None
}

fn legacy_payment_windowed_decision(
    policy: &PaymentPolicy,
    spent_today: i64,
    spent_month: i64,
    amount: i64,
) -> Option<(Verdict, String)> {
    if let Some(cap) = policy.daily_minor {
        if spent_today.saturating_add(amount) > cap {
            return Some((
                policy_action_verdict(policy.on_breach),
                format!(
                    "payment policy `{}`: daily spend would exceed cap {cap}",
                    policy.id
                ),
            ));
        }
    }
    if let Some(cap) = policy.monthly_minor {
        if spent_month.saturating_add(amount) > cap {
            return Some((
                policy_action_verdict(policy.on_breach),
                format!(
                    "payment policy `{}`: monthly spend would exceed cap {cap}",
                    policy.id
                ),
            ));
        }
    }
    None
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
