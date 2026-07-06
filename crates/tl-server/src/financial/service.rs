use std::sync::Arc;

use chrono::{Datelike, TimeZone, Utc};
use tl_core::{
    ApprovalRequirement, CreateFinancialActionRequest, CreateFinancialMandateRequest,
    FinancialActionListResponse, FinancialActionOutcome, FinancialActionRecord,
    FinancialActionStatus, FinancialApprovalRequestListResponse, FinancialApprovalRequestStatus,
    FinancialMandate, FinancialMandateListResponse, FinancialOutcomeListResponse, FinancialReceipt,
    Verdict, DEFAULT_ENVIRONMENT_ID,
};
use tl_engine::{evaluate_financial_policies, financial_matches, financial_windowed_verdict};
use tl_policy::FamilyPolicy;

use super::{validation::validate_create_action, FinancialStore, FinancialStoreError};
use crate::policies::PolicyStore;

#[derive(Clone)]
pub struct FinancialAuthorizationService {
    store: Arc<dyn FinancialStore>,
    policy_store: Option<Arc<dyn PolicyStore>>,
}

impl FinancialAuthorizationService {
    pub fn new(store: Arc<dyn FinancialStore>) -> Self {
        Self {
            store,
            policy_store: None,
        }
    }

    pub fn with_policy_store(
        store: Arc<dyn FinancialStore>,
        policy_store: Arc<dyn PolicyStore>,
    ) -> Self {
        Self {
            store,
            policy_store: Some(policy_store),
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
        let action = self.store.create_action(workspace_id, input).await?;
        if action.status != FinancialActionStatus::Proposed {
            return Ok(action);
        }
        self.apply_financial_policies(workspace_id, environment_id, action)
            .await
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
            )
            .await?;
        Ok(approved)
    }

    pub async fn deny_action(
        &self,
        workspace_id: &str,
        action_id: &str,
    ) -> Result<FinancialActionRecord, FinancialStoreError> {
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
            )
            .await?;
        Ok(denied)
    }

    pub async fn execute_action(
        &self,
        workspace_id: &str,
        action_id: &str,
    ) -> Result<FinancialActionRecord, FinancialStoreError> {
        let executed = self
            .transition_action(
                workspace_id,
                action_id,
                FinancialActionStatus::Executed,
                "executed",
            )
            .await?;
        self.store
            .create_receipt(
                workspace_id,
                action_id,
                None,
                vec![],
                serde_json::json!({
                    "action_id": action_id,
                    "action_status": "executed",
                    "receipt_source": "financial_authorization_service"
                }),
            )
            .await?;
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

        let decision = compose_policy_decisions(
            pure.verdict.map(|verdict| {
                (
                    verdict,
                    pure.reason
                        .unwrap_or_else(|| "financial policy matched".to_string()),
                )
            }),
            windowed,
        );

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
                self.hold_action(
                    workspace_id,
                    &action.id,
                    ApprovalRequirement {
                        required: true,
                        approver_roles: vec![],
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
            let FamilyPolicy::Financial(financial) = family.as_ref() else {
                continue;
            };
            if !financial_matches(financial, &action.action) {
                continue;
            }
            if financial.daily_minor.is_none() && financial.monthly_minor.is_none() {
                continue;
            }
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
            let Some(next) = financial_windowed_verdict(
                financial,
                spent_today,
                spent_month,
                action.action.amount.amount_minor,
            ) else {
                continue;
            };
            decision = compose_policy_decisions(decision, Some(next));
        }
        Ok(decision)
    }
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
