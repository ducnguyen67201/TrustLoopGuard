use std::collections::HashMap;

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use tl_core::{
    AgenticPaymentReservation, AgenticPaymentReservationStatus, ApprovalRequirement,
    CreateFinancialActionRequest, CreateFinancialMandateRequest, FinancialActionEvaluation,
    FinancialActionListResponse, FinancialActionOutcome, FinancialActionRecord,
    FinancialActionStatus, FinancialApprovalRequest, FinancialApprovalRequestListResponse,
    FinancialApprovalRequestStatus, FinancialEvaluationOutcome, FinancialExecutionBinding,
    FinancialExecutionConnector, FinancialExecutionConnectorStatus, FinancialExecutionGrant,
    FinancialExecutionGrantStatus, FinancialMandate, FinancialMandateListResponse,
    FinancialMandateStatus, FinancialObservationCurrencySummary, FinancialObservationReasonSummary,
    FinancialObservationReview, FinancialObservationReviewOutcome, FinancialOutcomeListResponse,
    FinancialRail, FinancialReceipt, MoneyAmount,
};
use tokio::sync::RwLock;

use super::{
    validation::{is_valid_transition, validate_create_action},
    AgenticPaymentBudgetReservationRequest, FinancialExecutionFinalization,
    FinancialLedgerEntryKind, FinancialStore, FinancialStoreError,
    StoredFinancialExecutionConnector,
};

type ExecutionCommitIdentity = (Option<String>, Option<String>);

#[derive(Debug, Default)]
pub struct MemoryFinancialStore {
    actions: RwLock<HashMap<String, FinancialActionRecord>>,
    idempotency: RwLock<HashMap<String, String>>,
    approval_requests: RwLock<HashMap<String, FinancialApprovalRequest>>,
    mandates: RwLock<HashMap<String, FinancialMandate>>,
    receipts: RwLock<HashMap<String, FinancialReceipt>>,
    outcomes: RwLock<HashMap<String, Vec<FinancialActionOutcome>>>,
    ledger_entries: RwLock<HashMap<String, MemoryLedgerEntry>>,
    ledger_idempotency: RwLock<HashMap<String, String>>,
    agentic_payments: RwLock<MemoryAgenticPayments>,
    evaluations: RwLock<HashMap<String, FinancialActionEvaluation>>,
    execution_grants: RwLock<HashMap<String, FinancialExecutionGrant>>,
    execution_commits: RwLock<HashMap<String, ExecutionCommitIdentity>>,
    execution_connectors: RwLock<HashMap<String, StoredFinancialExecutionConnector>>,
    observation_reviews: RwLock<HashMap<String, Vec<FinancialObservationReview>>>,
}

impl MemoryFinancialStore {
    pub fn new() -> Self {
        Self::default()
    }
}

#[derive(Debug, Clone)]
struct MemoryLedgerEntry {
    workspace_id: String,
    action_id: String,
    principal_id: String,
    kind: FinancialLedgerEntryKind,
    amount_minor: i64,
    currency: String,
    effective_at: DateTime<Utc>,
}

#[derive(Debug, Default)]
struct MemoryAgenticPayments {
    sessions: HashMap<String, MemoryAgenticPaymentSession>,
    reservations: HashMap<String, AgenticPaymentReservation>,
    by_action: HashMap<String, String>,
    by_requirement: HashMap<String, String>,
}

#[derive(Debug, Clone)]
struct MemoryAgenticPaymentSession {
    workspace_id: String,
    id: String,
    principal_id: String,
    currency: String,
    max_amount_minor: i64,
    reserved_minor: i64,
    committed_minor: i64,
    released_minor: i64,
    expires_at: DateTime<Utc>,
}

#[async_trait]
impl FinancialStore for MemoryFinancialStore {
    async fn create_action(
        &self,
        workspace_id: &str,
        environment_id: &str,
        input: CreateFinancialActionRequest,
    ) -> Result<FinancialActionRecord, FinancialStoreError> {
        validate_create_action(&input)?;
        let idempotency_key = format!("{workspace_id}:{}", input.idempotency_key.trim());
        let mut idempotency = self.idempotency.write().await;
        if let Some(action_id) = idempotency.get(&idempotency_key).cloned() {
            drop(idempotency);
            return self.get_action(workspace_id, &action_id).await;
        }

        let principal_id = clean_required("principal_id", &input.action.principal_id)?;
        let currency = clean_required("currency", &input.action.amount.currency)?.to_uppercase();
        let now = chrono::Utc::now().to_rfc3339();
        let id = input
            .action
            .id
            .clone()
            .unwrap_or_else(|| uuid::Uuid::now_v7().to_string());
        let mut record = FinancialActionRecord {
            id: id.clone(),
            workspace_id: workspace_id.to_string(),
            status: FinancialActionStatus::Proposed,
            status_reason: None,
            environment_id: Some(environment_id.to_string()),
            runtime_mode: None,
            evaluation: None,
            execution_grant: None,
            action: tl_core::FinancialAction {
                id: Some(id.clone()),
                ..input.action
            },
            evidence: input.evidence,
            created_at: now.clone(),
            updated_at: now,
        };
        record.action.principal_id = principal_id;
        record.action.amount.currency = currency;

        let mut actions = self.actions.write().await;
        let action_key = key(workspace_id, &id);
        if actions.contains_key(&action_key) {
            return Err(FinancialStoreError::Conflict);
        }
        actions.insert(action_key, record.clone());
        idempotency.insert(idempotency_key, id);
        Ok(record)
    }

    async fn get_action(
        &self,
        workspace_id: &str,
        action_id: &str,
    ) -> Result<FinancialActionRecord, FinancialStoreError> {
        self.actions
            .read()
            .await
            .get(&key(workspace_id, action_id))
            .cloned()
            .ok_or(FinancialStoreError::NotFound)
    }

    async fn list_actions(
        &self,
        workspace_id: &str,
    ) -> Result<FinancialActionListResponse, FinancialStoreError> {
        let mut actions = self
            .actions
            .read()
            .await
            .values()
            .filter(|action| action.workspace_id == workspace_id)
            .cloned()
            .collect::<Vec<_>>();
        actions.sort_by(|a, b| {
            b.created_at
                .cmp(&a.created_at)
                .then_with(|| b.id.cmp(&a.id))
        });
        Ok(FinancialActionListResponse { actions })
    }

    async fn persist_action_evaluation(
        &self,
        workspace_id: &str,
        evaluation: FinancialActionEvaluation,
    ) -> Result<FinancialActionEvaluation, FinancialStoreError> {
        let evaluation_key = key(workspace_id, &evaluation.action_id);
        let mut evaluations = self.evaluations.write().await;
        let persisted = evaluations
            .entry(evaluation_key.clone())
            .or_insert_with(|| evaluation.clone())
            .clone();
        drop(evaluations);
        let mut actions = self.actions.write().await;
        let action = actions
            .get_mut(&evaluation_key)
            .ok_or(FinancialStoreError::NotFound)?;
        action.runtime_mode = Some(persisted.runtime_mode);
        action.evaluation = Some(persisted.clone());
        Ok(persisted)
    }

    async fn get_action_evaluation(
        &self,
        workspace_id: &str,
        action_id: &str,
    ) -> Result<FinancialActionEvaluation, FinancialStoreError> {
        self.evaluations
            .read()
            .await
            .get(&key(workspace_id, action_id))
            .cloned()
            .ok_or(FinancialStoreError::NotFound)
    }

    async fn issue_execution_grant(
        &self,
        workspace_id: &str,
        action_id: &str,
        action_hash: &str,
        binding: FinancialExecutionBinding,
        expires_at: DateTime<Utc>,
    ) -> Result<FinancialExecutionGrant, FinancialStoreError> {
        let action_key = key(workspace_id, action_id);
        if !self.actions.read().await.contains_key(&action_key) {
            return Err(FinancialStoreError::NotFound);
        }
        let mut grants = self.execution_grants.write().await;
        let now = Utc::now().to_rfc3339();
        let grant = grants
            .entry(action_key.clone())
            .or_insert_with(|| FinancialExecutionGrant {
                id: uuid::Uuid::now_v7().to_string(),
                action_id: action_id.to_string(),
                action_hash: action_hash.to_string(),
                binding,
                status: FinancialExecutionGrantStatus::Issued,
                expires_at: expires_at.to_rfc3339(),
                created_at: now,
            })
            .clone();
        drop(grants);
        if grant.action_hash != action_hash || grant.binding != binding {
            return Err(FinancialStoreError::Conflict);
        }
        if let Some(action) = self.actions.write().await.get_mut(&action_key) {
            action.execution_grant = Some(grant.clone());
        }
        Ok(grant)
    }

    async fn get_execution_grant(
        &self,
        workspace_id: &str,
        action_id: &str,
    ) -> Result<FinancialExecutionGrant, FinancialStoreError> {
        self.execution_grants
            .read()
            .await
            .get(&key(workspace_id, action_id))
            .cloned()
            .ok_or(FinancialStoreError::NotFound)
    }

    async fn claim_execution_grant(
        &self,
        workspace_id: &str,
        action_id: &str,
        binding: FinancialExecutionBinding,
        _claim_id: &str,
        _stale_before: DateTime<Utc>,
    ) -> Result<FinancialExecutionGrant, FinancialStoreError> {
        let action_key = key(workspace_id, action_id);
        let mut grants = self.execution_grants.write().await;
        let grant = grants
            .get_mut(&action_key)
            .ok_or(FinancialStoreError::NotFound)?;
        if grant.binding != binding
            || grant.status != FinancialExecutionGrantStatus::Issued
            || DateTime::parse_from_rfc3339(&grant.expires_at)
                .map_err(|error| FinancialStoreError::Internal(error.to_string()))?
                .with_timezone(&Utc)
                <= Utc::now()
        {
            return Err(FinancialStoreError::Conflict);
        }
        grant.status = FinancialExecutionGrantStatus::Claimed;
        let claimed = grant.clone();
        drop(grants);
        if let Some(action) = self.actions.write().await.get_mut(&action_key) {
            action.execution_grant = Some(claimed.clone());
        }
        Ok(claimed)
    }

    async fn finalize_execution(
        &self,
        workspace_id: &str,
        action_id: &str,
        grant_id: &str,
        finalization: FinancialExecutionFinalization,
    ) -> Result<
        (
            FinancialActionRecord,
            FinancialExecutionGrant,
            FinancialReceipt,
        ),
        FinancialStoreError,
    > {
        let action_key = key(workspace_id, action_id);
        let mut grants = self.execution_grants.write().await;
        let grant = grants
            .get_mut(&action_key)
            .ok_or(FinancialStoreError::NotFound)?;
        if grant.id != grant_id {
            return Err(FinancialStoreError::Conflict);
        }
        if grant.status == FinancialExecutionGrantStatus::Committed {
            drop(grants);
            let commits = self.execution_commits.read().await;
            let existing = commits
                .get(&action_key)
                .ok_or_else(|| FinancialStoreError::Internal("execution commit missing".into()))?;
            if existing
                != &(
                    finalization.commit_idempotency_key.clone(),
                    finalization.attestation_hash.clone(),
                )
            {
                return Err(FinancialStoreError::Conflict);
            }
            drop(commits);
            let action = self.get_action(workspace_id, action_id).await?;
            let receipt = self.get_receipt(workspace_id, action_id).await?;
            let grant = self.get_execution_grant(workspace_id, action_id).await?;
            return Ok((action, grant, receipt));
        }
        if !matches!(
            grant.status,
            FinancialExecutionGrantStatus::Issued | FinancialExecutionGrantStatus::Claimed
        ) {
            return Err(FinancialStoreError::Conflict);
        }
        grant.status = FinancialExecutionGrantStatus::Committed;
        let committed_grant = grant.clone();
        drop(grants);
        self.execution_commits.write().await.insert(
            action_key.clone(),
            (
                finalization.commit_idempotency_key.clone(),
                finalization.attestation_hash.clone(),
            ),
        );

        let executed = self
            .transition_action(
                workspace_id,
                action_id,
                FinancialActionStatus::Executed,
                "execution_committed",
            )
            .await?;
        let mut ledger_ids = Vec::new();
        if self
            .ledger_entry_exists(workspace_id, &format!("{action_id}:reserved"))
            .await?
        {
            ledger_ids.push(
                self.record_ledger_entry(
                    workspace_id,
                    action_id,
                    FinancialLedgerEntryKind::Released,
                    executed.action.amount.amount_minor,
                    &executed.action.amount.currency,
                    &format!("{action_id}:released"),
                    serde_json::json!({"source": "execution_finalize"}),
                )
                .await?,
            );
        }
        ledger_ids.push(
            self.record_ledger_entry(
                workspace_id,
                action_id,
                FinancialLedgerEntryKind::Executed,
                executed.action.amount.amount_minor,
                &executed.action.amount.currency,
                &format!("{action_id}:executed"),
                serde_json::json!({"provider": finalization.provider}),
            )
            .await?,
        );
        let receipt = self
            .create_receipt(
                workspace_id,
                action_id,
                None,
                ledger_ids,
                finalization.proof,
            )
            .await?;
        self.record_action_outcome(
            workspace_id,
            action_id,
            FinancialActionOutcome {
                action_id: action_id.to_string(),
                status: tl_core::FinancialActionOutcomeStatus::Succeeded,
                reversal_capability: tl_core::ReversalCapability::None,
                recovery_status: tl_core::RecoveryStatus::NotAvailable,
                provider_status: Some(finalization.provider_status),
                provider_reference: finalization.provider_reference,
                final_loss_amount: None,
                occurred_at: Utc::now().to_rfc3339(),
                metadata: finalization.provider_response,
            },
        )
        .await?;
        if let Some(action) = self.actions.write().await.get_mut(&action_key) {
            action.execution_grant = Some(committed_grant.clone());
        }
        Ok((
            self.get_action(workspace_id, action_id).await?,
            committed_grant,
            receipt,
        ))
    }

    async fn fail_execution_grant(
        &self,
        workspace_id: &str,
        action_id: &str,
        grant_id: &str,
        reason: &str,
    ) -> Result<FinancialActionRecord, FinancialStoreError> {
        let action_key = key(workspace_id, action_id);
        let mut grants = self.execution_grants.write().await;
        let grant = grants
            .get_mut(&action_key)
            .ok_or(FinancialStoreError::NotFound)?;
        if grant.id != grant_id || grant.status == FinancialExecutionGrantStatus::Committed {
            return Err(FinancialStoreError::Conflict);
        }
        grant.status = FinancialExecutionGrantStatus::Failed;
        let failed_grant = grant.clone();
        drop(grants);
        let mut action = self
            .transition_action_with_reason(
                workspace_id,
                action_id,
                FinancialActionStatus::Failed,
                "execution_failed",
                reason,
            )
            .await?;
        action.execution_grant = Some(failed_grant);
        self.actions
            .write()
            .await
            .insert(action_key, action.clone());
        self.record_action_outcome(
            workspace_id,
            action_id,
            FinancialActionOutcome {
                action_id: action_id.to_string(),
                status: tl_core::FinancialActionOutcomeStatus::Failed,
                reversal_capability: tl_core::ReversalCapability::None,
                recovery_status: tl_core::RecoveryStatus::NotAvailable,
                provider_status: Some("failed".into()),
                provider_reference: None,
                final_loss_amount: None,
                occurred_at: Utc::now().to_rfc3339(),
                metadata: serde_json::json!({"reason": reason}),
            },
        )
        .await?;
        Ok(action)
    }

    async fn create_execution_connector(
        &self,
        workspace_id: &str,
        display_name: &str,
        encrypted_secret: &str,
        allowed_rails: Vec<FinancialRail>,
        allowed_operations: Vec<String>,
    ) -> Result<StoredFinancialExecutionConnector, FinancialStoreError> {
        let id = uuid::Uuid::now_v7().to_string();
        let stored = StoredFinancialExecutionConnector {
            connector: FinancialExecutionConnector {
                id: id.clone(),
                workspace_id: workspace_id.to_string(),
                display_name: display_name.to_string(),
                status: FinancialExecutionConnectorStatus::Active,
                allowed_rails,
                allowed_operations,
                created_at: Utc::now().to_rfc3339(),
                revoked_at: None,
            },
            encrypted_secret: encrypted_secret.to_string(),
        };
        self.execution_connectors
            .write()
            .await
            .insert(key(workspace_id, &id), stored.clone());
        Ok(stored)
    }

    async fn list_execution_connectors(
        &self,
        workspace_id: &str,
    ) -> Result<Vec<FinancialExecutionConnector>, FinancialStoreError> {
        let mut connectors = self
            .execution_connectors
            .read()
            .await
            .values()
            .filter(|stored| stored.connector.workspace_id == workspace_id)
            .map(|stored| stored.connector.clone())
            .collect::<Vec<_>>();
        connectors.sort_by(|a, b| b.created_at.cmp(&a.created_at));
        Ok(connectors)
    }

    async fn get_execution_connector(
        &self,
        workspace_id: &str,
        connector_id: &str,
    ) -> Result<StoredFinancialExecutionConnector, FinancialStoreError> {
        self.execution_connectors
            .read()
            .await
            .get(&key(workspace_id, connector_id))
            .cloned()
            .ok_or(FinancialStoreError::NotFound)
    }

    async fn revoke_execution_connector(
        &self,
        workspace_id: &str,
        connector_id: &str,
    ) -> Result<FinancialExecutionConnector, FinancialStoreError> {
        let mut connectors = self.execution_connectors.write().await;
        let stored = connectors
            .get_mut(&key(workspace_id, connector_id))
            .ok_or(FinancialStoreError::NotFound)?;
        stored.connector.status = FinancialExecutionConnectorStatus::Revoked;
        stored.connector.revoked_at = Some(Utc::now().to_rfc3339());
        Ok(stored.connector.clone())
    }

    async fn create_observation_review(
        &self,
        workspace_id: &str,
        action_id: &str,
        outcome: FinancialObservationReviewOutcome,
        note: Option<String>,
        reviewed_by: &str,
    ) -> Result<FinancialObservationReview, FinancialStoreError> {
        let evaluation = self.get_action_evaluation(workspace_id, action_id).await?;
        if evaluation.runtime_mode != tl_core::FinancialRuntimeMode::Observe
            || !matches!(
                evaluation.outcome,
                FinancialEvaluationOutcome::WouldHold | FinancialEvaluationOutcome::WouldBlock
            )
        {
            return Err(FinancialStoreError::Conflict);
        }
        let review = FinancialObservationReview {
            id: uuid::Uuid::now_v7().to_string(),
            workspace_id: workspace_id.to_string(),
            action_id: action_id.to_string(),
            outcome,
            note,
            reviewed_by: reviewed_by.to_string(),
            created_at: Utc::now().to_rfc3339(),
        };
        self.observation_reviews
            .write()
            .await
            .entry(key(workspace_id, action_id))
            .or_default()
            .push(review.clone());
        Ok(review)
    }

    async fn list_observation_reviews(
        &self,
        workspace_id: &str,
        action_id: &str,
    ) -> Result<Vec<FinancialObservationReview>, FinancialStoreError> {
        let mut reviews = self
            .observation_reviews
            .read()
            .await
            .get(&key(workspace_id, action_id))
            .cloned()
            .unwrap_or_default();
        reviews.sort_by(|a, b| b.created_at.cmp(&a.created_at));
        Ok(reviews)
    }

    async fn observation_summary(
        &self,
        workspace_id: &str,
        environment_id: &str,
        start: DateTime<Utc>,
        end: DateTime<Utc>,
    ) -> Result<
        (
            Vec<FinancialObservationCurrencySummary>,
            Vec<FinancialObservationReasonSummary>,
        ),
        FinancialStoreError,
    > {
        let evaluations = self.evaluations.read().await;
        let reviews = self.observation_reviews.read().await;
        let mut currencies: HashMap<String, FinancialObservationCurrencySummary> = HashMap::new();
        let mut reasons: HashMap<(String, FinancialEvaluationOutcome, String), (i64, i64)> =
            HashMap::new();
        let workspace_prefix = format!("{workspace_id}:");
        for (_, evaluation) in evaluations.iter().filter(|(evaluation_key, evaluation)| {
            evaluation_key.starts_with(&workspace_prefix)
                && evaluation.environment_id == environment_id
                && DateTime::parse_from_rfc3339(&evaluation.created_at)
                    .map(|created| {
                        created.with_timezone(&Utc) >= start && created.with_timezone(&Utc) < end
                    })
                    .unwrap_or(false)
                && evaluation.runtime_mode == tl_core::FinancialRuntimeMode::Observe
        }) {
            let row = currencies
                .entry(evaluation.amount.currency.clone())
                .or_insert_with(|| empty_currency_summary(&evaluation.amount.currency));
            row.total_observed_count += 1;
            row.total_observed_amount_minor += evaluation.amount.amount_minor;
            match evaluation.outcome {
                FinancialEvaluationOutcome::WouldAllow => {
                    row.would_allow_count += 1;
                    row.would_allow_amount_minor += evaluation.amount.amount_minor;
                }
                FinancialEvaluationOutcome::WouldHold => {
                    row.would_hold_count += 1;
                    row.would_hold_amount_minor += evaluation.amount.amount_minor;
                    row.adverse_count += 1;
                    row.estimated_approval_count += 1;
                }
                FinancialEvaluationOutcome::WouldBlock => {
                    row.would_block_count += 1;
                    row.would_block_amount_minor += evaluation.amount.amount_minor;
                    row.adverse_count += 1;
                }
                _ => continue,
            }
            if let Some(latest) = reviews
                .get(&key(workspace_id, &evaluation.action_id))
                .and_then(|items| items.last())
            {
                row.reviewed_adverse_count += 1;
                if latest.outcome == FinancialObservationReviewOutcome::FalsePositive {
                    row.false_positive_count += 1;
                }
            }
            let reason = reasons
                .entry((
                    evaluation.reason.clone(),
                    evaluation.outcome,
                    evaluation.amount.currency.clone(),
                ))
                .or_default();
            reason.0 += 1;
            reason.1 += evaluation.amount.amount_minor;
        }
        for row in currencies.values_mut() {
            row.adverse_rate_bps = rate_bps(row.adverse_count, row.total_observed_count);
            row.estimated_approval_rate_bps =
                rate_bps(row.estimated_approval_count, row.total_observed_count);
            row.false_positive_rate_bps =
                rate_bps(row.false_positive_count, row.reviewed_adverse_count);
        }
        let reason_rows = reasons
            .into_iter()
            .map(|((reason, outcome, currency), (count, amount_minor))| {
                FinancialObservationReasonSummary {
                    reason,
                    outcome,
                    count,
                    amount: MoneyAmount {
                        amount_minor,
                        currency,
                    },
                }
            })
            .collect();
        Ok((currencies.into_values().collect(), reason_rows))
    }

    async fn create_mandate(
        &self,
        workspace_id: &str,
        input: CreateFinancialMandateRequest,
    ) -> Result<FinancialMandate, FinancialStoreError> {
        let principal_id = clean_required("principal_id", &input.principal_id)?;
        let id = input
            .id
            .and_then(clean_optional)
            .unwrap_or_else(|| uuid::Uuid::now_v7().to_string());
        let version = input.version.unwrap_or(1);
        if version <= 0 {
            return Err(FinancialStoreError::Validation(
                "mandate version must be positive".into(),
            ));
        }
        let now = chrono::Utc::now().to_rfc3339();
        let mandate = FinancialMandate {
            id: id.clone(),
            workspace_id: workspace_id.to_string(),
            version,
            status: FinancialMandateStatus::Active,
            principal_id,
            scope: input.scope,
            metadata: input.metadata,
            starts_at: input.starts_at,
            expires_at: input.expires_at,
            created_at: now.clone(),
            updated_at: now,
        };
        let mut mandates = self.mandates.write().await;
        let key = mandate_key(workspace_id, &id, version);
        if mandates.contains_key(&key) {
            return Err(FinancialStoreError::Conflict);
        }
        mandates.insert(key, mandate.clone());
        Ok(mandate)
    }

    async fn list_mandates(
        &self,
        workspace_id: &str,
    ) -> Result<FinancialMandateListResponse, FinancialStoreError> {
        let mut mandates = self
            .mandates
            .read()
            .await
            .values()
            .filter(|mandate| mandate.workspace_id == workspace_id)
            .cloned()
            .collect::<Vec<_>>();
        mandates.sort_by(|a, b| {
            b.created_at
                .cmp(&a.created_at)
                .then_with(|| b.id.cmp(&a.id))
        });
        Ok(FinancialMandateListResponse { mandates })
    }

    async fn get_mandate(
        &self,
        workspace_id: &str,
        mandate_id: &str,
        version: Option<i32>,
    ) -> Result<FinancialMandate, FinancialStoreError> {
        self.mandates
            .read()
            .await
            .values()
            .filter(|mandate| {
                mandate.workspace_id == workspace_id
                    && mandate.id == mandate_id
                    && version.map_or(true, |expected| mandate.version == expected)
            })
            .max_by_key(|mandate| mandate.version)
            .cloned()
            .ok_or(FinancialStoreError::NotFound)
    }

    async fn revoke_mandate(
        &self,
        workspace_id: &str,
        mandate_id: &str,
    ) -> Result<FinancialMandate, FinancialStoreError> {
        let mut mandates = self.mandates.write().await;
        let mut latest_key: Option<String> = None;
        let mut latest_version = i32::MIN;
        for (key, mandate) in mandates.iter() {
            if mandate.workspace_id == workspace_id
                && mandate.id == mandate_id
                && mandate.version > latest_version
            {
                latest_version = mandate.version;
                latest_key = Some(key.clone());
            }
        }
        let latest_key = latest_key.ok_or(FinancialStoreError::NotFound)?;
        let latest = mandates
            .get_mut(&latest_key)
            .ok_or(FinancialStoreError::NotFound)?;
        latest.status = FinancialMandateStatus::Revoked;
        latest.updated_at = chrono::Utc::now().to_rfc3339();
        Ok(latest.clone())
    }

    async fn create_receipt(
        &self,
        workspace_id: &str,
        action_id: &str,
        trace_id: Option<&str>,
        ledger_event_ids: Vec<String>,
        proof: serde_json::Value,
    ) -> Result<FinancialReceipt, FinancialStoreError> {
        self.get_action(workspace_id, action_id).await?;
        let id = action_id.to_string();
        if let Some(receipt) = self
            .receipts
            .read()
            .await
            .get(&key(workspace_id, &id))
            .cloned()
        {
            return Ok(receipt);
        }
        let receipt = FinancialReceipt {
            id: id.clone(),
            action_id: action_id.to_string(),
            trace_id: trace_id.map(str::to_string),
            ledger_event_ids,
            proof,
            created_at: chrono::Utc::now().to_rfc3339(),
        };
        self.receipts
            .write()
            .await
            .insert(key(workspace_id, &id), receipt.clone());
        Ok(receipt)
    }

    async fn get_receipt(
        &self,
        workspace_id: &str,
        receipt_id: &str,
    ) -> Result<FinancialReceipt, FinancialStoreError> {
        self.receipts
            .read()
            .await
            .get(&key(workspace_id, receipt_id))
            .cloned()
            .ok_or(FinancialStoreError::NotFound)
    }

    async fn record_action_outcome(
        &self,
        workspace_id: &str,
        action_id: &str,
        outcome: FinancialActionOutcome,
    ) -> Result<FinancialActionOutcome, FinancialStoreError> {
        self.get_action(workspace_id, action_id).await?;
        if outcome.action_id != action_id {
            return Err(FinancialStoreError::Validation(
                "outcome action_id must match path action id".into(),
            ));
        }
        self.outcomes
            .write()
            .await
            .entry(key(workspace_id, action_id))
            .or_default()
            .insert(0, outcome.clone());
        Ok(outcome)
    }

    async fn list_action_outcomes(
        &self,
        workspace_id: &str,
        action_id: &str,
    ) -> Result<FinancialOutcomeListResponse, FinancialStoreError> {
        self.get_action(workspace_id, action_id).await?;
        let mut outcomes = self
            .outcomes
            .read()
            .await
            .get(&key(workspace_id, action_id))
            .cloned()
            .unwrap_or_default();
        outcomes.sort_by(|a, b| b.occurred_at.cmp(&a.occurred_at));
        Ok(FinancialOutcomeListResponse { outcomes })
    }

    async fn create_approval_request(
        &self,
        workspace_id: &str,
        action_id: &str,
        approval: ApprovalRequirement,
    ) -> Result<FinancialApprovalRequest, FinancialStoreError> {
        self.get_action(workspace_id, action_id).await?;
        if let Some(expires_at) = &approval.expires_at {
            DateTime::parse_from_rfc3339(expires_at).map_err(|_| {
                FinancialStoreError::Validation("approval expires_at must be RFC3339".into())
            })?;
        }
        let now = chrono::Utc::now().to_rfc3339();
        let id = uuid::Uuid::now_v7().to_string();
        let request = FinancialApprovalRequest {
            id: id.clone(),
            workspace_id: workspace_id.to_string(),
            action_id: action_id.to_string(),
            status: FinancialApprovalRequestStatus::Pending,
            reason: approval.reason,
            approver_roles: approval.approver_roles,
            decided_by: None,
            decided_at: None,
            expires_at: approval.expires_at,
            metadata: serde_json::json!({}),
            created_at: now.clone(),
            updated_at: now,
        };
        self.approval_requests
            .write()
            .await
            .insert(key(workspace_id, &id), request.clone());
        Ok(request)
    }

    async fn list_approval_requests(
        &self,
        workspace_id: &str,
    ) -> Result<FinancialApprovalRequestListResponse, FinancialStoreError> {
        let mut approval_requests = self
            .approval_requests
            .read()
            .await
            .values()
            .filter(|request| request.workspace_id == workspace_id)
            .cloned()
            .collect::<Vec<_>>();
        approval_requests.sort_by(|a, b| {
            b.created_at
                .cmp(&a.created_at)
                .then_with(|| b.id.cmp(&a.id))
        });
        Ok(FinancialApprovalRequestListResponse { approval_requests })
    }

    async fn resolve_pending_approval_requests(
        &self,
        workspace_id: &str,
        action_id: &str,
        status: FinancialApprovalRequestStatus,
        decided_by: Option<&str>,
    ) -> Result<(), FinancialStoreError> {
        if !matches!(
            status,
            FinancialApprovalRequestStatus::Approved | FinancialApprovalRequestStatus::Denied
        ) {
            return Err(FinancialStoreError::Validation(
                "approval request resolution must be approved or denied".into(),
            ));
        }
        self.get_action(workspace_id, action_id).await?;
        let now = chrono::Utc::now().to_rfc3339();
        for request in self.approval_requests.write().await.values_mut() {
            if request.workspace_id == workspace_id
                && request.action_id == action_id
                && request.status == FinancialApprovalRequestStatus::Pending
            {
                request.status = status;
                request.decided_by = decided_by.map(str::to_string);
                request.decided_at = Some(now.clone());
                request.updated_at = now.clone();
            }
        }
        Ok(())
    }

    async fn transition_action(
        &self,
        workspace_id: &str,
        action_id: &str,
        status: FinancialActionStatus,
        _event_type: &str,
    ) -> Result<FinancialActionRecord, FinancialStoreError> {
        transition_action_with_status_reason(self, workspace_id, action_id, status, _event_type)
            .await
    }

    async fn transition_action_with_reason(
        &self,
        workspace_id: &str,
        action_id: &str,
        status: FinancialActionStatus,
        _event_type: &str,
        reason: &str,
    ) -> Result<FinancialActionRecord, FinancialStoreError> {
        transition_action_with_status_reason(self, workspace_id, action_id, status, reason).await
    }

    async fn record_ledger_entry(
        &self,
        workspace_id: &str,
        action_id: &str,
        kind: FinancialLedgerEntryKind,
        amount_minor: i64,
        currency: &str,
        idempotency_key: &str,
        _metadata: serde_json::Value,
    ) -> Result<String, FinancialStoreError> {
        if amount_minor < 0 {
            return Err(FinancialStoreError::Validation(
                "financial ledger amount must be non-negative".into(),
            ));
        }
        let currency = clean_required("currency", currency)?.to_uppercase();
        let idempotency_key = clean_required("idempotency_key", idempotency_key)?;
        let scoped_idempotency_key = key(workspace_id, &idempotency_key);
        let mut ledger_idempotency = self.ledger_idempotency.write().await;
        if let Some(entry_id) = ledger_idempotency.get(&scoped_idempotency_key).cloned() {
            let entries = self.ledger_entries.read().await;
            let existing = entries
                .get(&key(workspace_id, &entry_id))
                .ok_or(FinancialStoreError::Conflict)?;
            if existing.action_id != action_id
                || existing.kind != kind
                || existing.amount_minor != amount_minor
                || existing.currency != currency
            {
                return Err(FinancialStoreError::Conflict);
            }
            return Ok(entry_id);
        }

        let action = self.get_action(workspace_id, action_id).await?;
        let id = uuid::Uuid::now_v7().to_string();
        let entry = MemoryLedgerEntry {
            workspace_id: workspace_id.to_string(),
            action_id: action_id.to_string(),
            principal_id: action.action.principal_id,
            kind,
            amount_minor,
            currency,
            effective_at: Utc::now(),
        };
        self.ledger_entries
            .write()
            .await
            .insert(key(workspace_id, &id), entry);
        ledger_idempotency.insert(scoped_idempotency_key, id.clone());
        Ok(id)
    }

    async fn ledger_entry_exists(
        &self,
        workspace_id: &str,
        idempotency_key: &str,
    ) -> Result<bool, FinancialStoreError> {
        let idempotency_key = clean_required("idempotency_key", idempotency_key)?;
        Ok(self
            .ledger_idempotency
            .read()
            .await
            .contains_key(&key(workspace_id, &idempotency_key)))
    }

    async fn net_spend_minor(
        &self,
        workspace_id: &str,
        principal_id: &str,
        currency: &str,
        start: DateTime<Utc>,
        end: DateTime<Utc>,
    ) -> Result<i64, FinancialStoreError> {
        let currency = currency.to_uppercase();
        Ok(self
            .ledger_entries
            .read()
            .await
            .values()
            .filter(|entry| {
                entry.workspace_id == workspace_id
                    && entry.principal_id == principal_id
                    && entry.currency == currency
                    && entry.effective_at >= start
                    && entry.effective_at < end
            })
            .map(|entry| signed_ledger_amount(entry.kind, entry.amount_minor))
            .sum())
    }

    async fn try_reserve_agentic_payment_budget(
        &self,
        request: AgenticPaymentBudgetReservationRequest,
    ) -> Result<AgenticPaymentReservation, FinancialStoreError> {
        let AgenticPaymentBudgetReservationRequest {
            workspace_id,
            session_id,
            principal_id,
            action_id,
            payment_requirement_hash,
            amount,
            session_limit_minor,
            expires_at,
            metadata,
        } = request;
        if amount.amount_minor <= 0 {
            return Err(FinancialStoreError::Validation(
                "agentic payment reservation amount must be positive".into(),
            ));
        }
        if session_limit_minor < amount.amount_minor {
            return Err(FinancialStoreError::Validation(
                "agentic payment session limit is below requested amount".into(),
            ));
        }
        let session_id = clean_required("session_id", &session_id)?;
        let principal_id = clean_required("principal_id", &principal_id)?;
        let payment_requirement_hash =
            clean_required("payment_requirement_hash", &payment_requirement_hash)?;
        let currency = clean_required("currency", &amount.currency)?.to_uppercase();
        let action_key = key(&workspace_id, &action_id);
        let session_key = key(&workspace_id, &session_id);
        let requirement_key = format!("{workspace_id}:{session_id}:{payment_requirement_hash}");
        {
            let actions = self.actions.read().await;
            let action = actions
                .get(&action_key)
                .ok_or(FinancialStoreError::NotFound)?;
            if action.action.principal_id != principal_id
                || !action
                    .action
                    .amount
                    .currency
                    .eq_ignore_ascii_case(&currency)
                || action.action.amount.amount_minor != amount.amount_minor
            {
                return Err(FinancialStoreError::Conflict);
            }
        }
        let mut payments = self.agentic_payments.write().await;
        if let Some(existing_id) = payments.by_requirement.get(&requirement_key).cloned() {
            let existing = payments
                .reservations
                .get(&existing_id)
                .cloned()
                .ok_or(FinancialStoreError::Conflict)?;
            if existing.action_id != action_id
                || existing.amount.amount_minor != amount.amount_minor
                || !existing.amount.currency.eq_ignore_ascii_case(&currency)
            {
                return Err(FinancialStoreError::Conflict);
            }
            return Ok(existing);
        }
        if payments.by_action.contains_key(&action_key) {
            return Err(FinancialStoreError::Conflict);
        }
        let session = payments
            .sessions
            .entry(session_key.clone())
            .or_insert_with(|| MemoryAgenticPaymentSession {
                workspace_id: workspace_id.clone(),
                id: session_id.clone(),
                principal_id: principal_id.clone(),
                currency: currency.clone(),
                max_amount_minor: session_limit_minor,
                reserved_minor: 0,
                committed_minor: 0,
                released_minor: 0,
                expires_at,
            });
        if session.workspace_id != workspace_id
            || session.id != session_id
            || session.principal_id != principal_id
            || !session.currency.eq_ignore_ascii_case(&currency)
        {
            return Err(FinancialStoreError::Conflict);
        }
        if session.expires_at <= Utc::now() {
            return Err(FinancialStoreError::Validation(
                "agentic payment session is expired".into(),
            ));
        }
        let next_reserved = session
            .reserved_minor
            .checked_add(amount.amount_minor)
            .ok_or_else(|| {
                FinancialStoreError::Internal("agentic payment reserved amount overflow".into())
            })?;
        let projected_total = next_reserved
            .checked_add(session.committed_minor)
            .ok_or_else(|| {
                FinancialStoreError::Internal("agentic payment session amount overflow".into())
            })?;
        if projected_total > session.max_amount_minor {
            return Err(FinancialStoreError::Validation(
                "agentic payment session budget exceeded".into(),
            ));
        }
        session.reserved_minor = next_reserved;
        let reservation = AgenticPaymentReservation {
            id: uuid::Uuid::now_v7().to_string(),
            session_id,
            action_id,
            principal_id,
            payment_requirement_hash,
            amount: MoneyAmount {
                amount_minor: amount.amount_minor,
                currency,
            },
            status: AgenticPaymentReservationStatus::Reserved,
            expires_at: expires_at.to_rfc3339(),
            committed_at: None,
            released_at: None,
            metadata,
        };
        let reservation_key = key(&workspace_id, &reservation.id);
        payments
            .by_requirement
            .insert(requirement_key, reservation_key.clone());
        payments
            .by_action
            .insert(action_key, reservation_key.clone());
        payments
            .reservations
            .insert(reservation_key, reservation.clone());
        Ok(reservation)
    }

    async fn get_agentic_payment_reservation(
        &self,
        workspace_id: &str,
        action_id: &str,
    ) -> Result<AgenticPaymentReservation, FinancialStoreError> {
        let payments = self.agentic_payments.read().await;
        let reservation_id = payments
            .by_action
            .get(&key(workspace_id, action_id))
            .ok_or(FinancialStoreError::NotFound)?;
        payments
            .reservations
            .get(reservation_id)
            .cloned()
            .ok_or(FinancialStoreError::NotFound)
    }

    async fn commit_agentic_payment_reservation(
        &self,
        workspace_id: &str,
        action_id: &str,
        proof: serde_json::Value,
    ) -> Result<AgenticPaymentReservation, FinancialStoreError> {
        let mut payments = self.agentic_payments.write().await;
        let reservation_id = payments
            .by_action
            .get(&key(workspace_id, action_id))
            .cloned()
            .ok_or(FinancialStoreError::NotFound)?;
        let mut reservation = payments
            .reservations
            .get(&reservation_id)
            .cloned()
            .ok_or(FinancialStoreError::NotFound)?;
        if reservation.status == AgenticPaymentReservationStatus::Committed {
            return Ok(reservation);
        }
        if reservation.status != AgenticPaymentReservationStatus::Reserved {
            return Err(FinancialStoreError::Conflict);
        }
        let expires_at = chrono::DateTime::parse_from_rfc3339(&reservation.expires_at)
            .map_err(|e| FinancialStoreError::Internal(format!("reservation expires_at: {e}")))?
            .with_timezone(&Utc);
        if expires_at <= Utc::now() {
            return Err(FinancialStoreError::Conflict);
        }
        let session_key = key(workspace_id, &reservation.session_id);
        let session = payments
            .sessions
            .get_mut(&session_key)
            .ok_or(FinancialStoreError::NotFound)?;
        if session.reserved_minor < reservation.amount.amount_minor {
            return Err(FinancialStoreError::Conflict);
        }
        session.reserved_minor -= reservation.amount.amount_minor;
        session.committed_minor = session
            .committed_minor
            .checked_add(reservation.amount.amount_minor)
            .ok_or_else(|| {
                FinancialStoreError::Internal("agentic payment committed amount overflow".into())
            })?;
        reservation.status = AgenticPaymentReservationStatus::Committed;
        reservation.committed_at = Some(Utc::now().to_rfc3339());
        reservation.metadata = merge_metadata(
            reservation.metadata,
            serde_json::json!({
                "commit_proof": proof,
            }),
        );
        payments
            .reservations
            .insert(reservation_id, reservation.clone());
        Ok(reservation)
    }

    async fn release_agentic_payment_reservation(
        &self,
        workspace_id: &str,
        action_id: &str,
        reason: &str,
        metadata: serde_json::Value,
    ) -> Result<AgenticPaymentReservation, FinancialStoreError> {
        let mut payments = self.agentic_payments.write().await;
        let reservation_id = payments
            .by_action
            .get(&key(workspace_id, action_id))
            .cloned()
            .ok_or(FinancialStoreError::NotFound)?;
        let mut reservation = payments
            .reservations
            .get(&reservation_id)
            .cloned()
            .ok_or(FinancialStoreError::NotFound)?;
        if reservation.status == AgenticPaymentReservationStatus::Released {
            return Ok(reservation);
        }
        if reservation.status != AgenticPaymentReservationStatus::Reserved {
            return Err(FinancialStoreError::Conflict);
        }
        let session_key = key(workspace_id, &reservation.session_id);
        let session = payments
            .sessions
            .get_mut(&session_key)
            .ok_or(FinancialStoreError::NotFound)?;
        if session.reserved_minor < reservation.amount.amount_minor {
            return Err(FinancialStoreError::Conflict);
        }
        session.reserved_minor -= reservation.amount.amount_minor;
        session.released_minor = session
            .released_minor
            .checked_add(reservation.amount.amount_minor)
            .ok_or_else(|| {
                FinancialStoreError::Internal("agentic payment released amount overflow".into())
            })?;
        reservation.status = AgenticPaymentReservationStatus::Released;
        reservation.released_at = Some(Utc::now().to_rfc3339());
        reservation.metadata = merge_metadata(
            reservation.metadata,
            serde_json::json!({
                "release_reason": reason,
                "release_metadata": metadata,
            }),
        );
        payments
            .reservations
            .insert(reservation_id, reservation.clone());
        Ok(reservation)
    }
}

fn key(workspace_id: &str, action_id: &str) -> String {
    format!("{workspace_id}:{action_id}")
}

async fn transition_action_with_status_reason(
    store: &MemoryFinancialStore,
    workspace_id: &str,
    action_id: &str,
    status: FinancialActionStatus,
    status_reason: &str,
) -> Result<FinancialActionRecord, FinancialStoreError> {
    let mut actions = store.actions.write().await;
    let record = actions
        .get_mut(&key(workspace_id, action_id))
        .ok_or(FinancialStoreError::NotFound)?;
    if !is_valid_transition(record.status, status) {
        return Err(FinancialStoreError::Conflict);
    }
    record.status = status;
    record.status_reason = Some(status_reason.to_string());
    record.updated_at = chrono::Utc::now().to_rfc3339();
    Ok(record.clone())
}

fn mandate_key(workspace_id: &str, mandate_id: &str, version: i32) -> String {
    format!("{workspace_id}:{mandate_id}:{version}")
}

fn clean_required(name: &str, value: &str) -> Result<String, FinancialStoreError> {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        return Err(FinancialStoreError::Validation(format!(
            "{name} must not be empty"
        )));
    }
    Ok(trimmed.to_string())
}

fn clean_optional(value: String) -> Option<String> {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        None
    } else {
        Some(trimmed.to_string())
    }
}

fn signed_ledger_amount(kind: FinancialLedgerEntryKind, amount_minor: i64) -> i64 {
    match kind {
        FinancialLedgerEntryKind::Reserved | FinancialLedgerEntryKind::Executed => amount_minor,
        FinancialLedgerEntryKind::Released | FinancialLedgerEntryKind::Reversed => -amount_minor,
    }
}

fn merge_metadata(mut base: serde_json::Value, extra: serde_json::Value) -> serde_json::Value {
    match (&mut base, extra) {
        (serde_json::Value::Object(base), serde_json::Value::Object(extra)) => {
            for (key, value) in extra {
                base.insert(key, value);
            }
            serde_json::Value::Object(base.clone())
        }
        (_, extra) => extra,
    }
}

fn empty_currency_summary(currency: &str) -> FinancialObservationCurrencySummary {
    FinancialObservationCurrencySummary {
        currency: currency.to_string(),
        total_observed_count: 0,
        total_observed_amount_minor: 0,
        would_allow_count: 0,
        would_allow_amount_minor: 0,
        would_hold_count: 0,
        would_hold_amount_minor: 0,
        would_block_count: 0,
        would_block_amount_minor: 0,
        adverse_count: 0,
        adverse_rate_bps: 0,
        estimated_approval_count: 0,
        estimated_approval_rate_bps: 0,
        reviewed_adverse_count: 0,
        false_positive_count: 0,
        false_positive_rate_bps: 0,
    }
}

fn rate_bps(numerator: i64, denominator: i64) -> i32 {
    if denominator <= 0 {
        return 0;
    }
    numerator
        .saturating_mul(10_000)
        .checked_div(denominator)
        .unwrap_or(0)
        .clamp(0, 10_000) as i32
}
