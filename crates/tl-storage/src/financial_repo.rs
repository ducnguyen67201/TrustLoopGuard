use std::collections::HashMap;

use chrono::{DateTime, Utc};
use diesel::prelude::*;
use diesel_async::{AsyncConnection, RunQueryDsl};
use serde::de::DeserializeOwned;
use serde::Serialize;
use tl_core::{
    AgenticPaymentReservation, AgenticPaymentReservationStatus, CreateFinancialActionRequest,
    CreateFinancialMandateRequest, EvidenceRef, FinancialAction, FinancialActionEvaluation,
    FinancialActionKind, FinancialActionOutcome, FinancialActionOutcomeStatus,
    FinancialActionStatus, FinancialApprovalRequestStatus, FinancialEvaluationOutcome,
    FinancialExecutionBinding, FinancialExecutionConnector, FinancialExecutionConnectorStatus,
    FinancialExecutionGrant, FinancialExecutionGrantStatus, FinancialMandate,
    FinancialMandateStatus, FinancialObservationCurrencySummary, FinancialObservationReasonSummary,
    FinancialObservationReview, FinancialObservationReviewOutcome, FinancialRail, FinancialReceipt,
    FinancialRuntimeMode, MoneyAmount, RecoveryStatus, ReversalCapability,
};
use uuid::Uuid;

use crate::models::{
    ApprovalRequestRecord, FinancialActionEvaluationRecord, FinancialActionEventRecord,
    FinancialActionOutcomeRecord, FinancialActionRecord, FinancialExecutionConnectorRecord,
    FinancialExecutionGrantRecord, FinancialLedgerEntryRecord, FinancialObservationReviewRecord,
    FinancialPaymentReservationRecord, FinancialPaymentSessionRecord, FinancialReceiptRecord,
    MandateRecord, NewApprovalRequest, NewFinancialAction, NewFinancialActionEvaluation,
    NewFinancialActionEvent, NewFinancialActionOutcome, NewFinancialExecutionConnector,
    NewFinancialExecutionGrant, NewFinancialLedgerEntry, NewFinancialObservationReview,
    NewFinancialPaymentReservation, NewFinancialPaymentSession, NewFinancialReceipt, NewMandate,
};
use crate::postgres::{DbConnection, DbPool};
use crate::schema::{
    approval_requests, financial_action_evaluations, financial_action_events,
    financial_action_outcomes, financial_actions, financial_execution_connectors,
    financial_execution_grants, financial_ledger_entries, financial_observation_reviews,
    financial_payment_reservations, financial_payment_sessions, financial_receipts, mandates,
};
use crate::StorageError;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FinancialLedgerEntryKind {
    Reserved,
    Released,
    Executed,
    Reversed,
}

impl FinancialLedgerEntryKind {
    fn as_str(self) -> &'static str {
        match self {
            Self::Reserved => "reserved",
            Self::Released => "released",
            Self::Executed => "executed",
            Self::Reversed => "reversed",
        }
    }

    fn signed_amount(self, amount_minor: i64) -> i64 {
        match self {
            Self::Reserved | Self::Executed => amount_minor,
            Self::Released | Self::Reversed => -amount_minor,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct StoredFinancialAction {
    pub workspace_id: String,
    pub id: String,
    pub environment_id: String,
    pub idempotency_key: String,
    pub status: FinancialActionStatus,
    pub status_reason: Option<String>,
    pub action: FinancialAction,
    pub evidence: Vec<EvidenceRef>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct StoredFinancialActionEvent {
    pub workspace_id: String,
    pub id: String,
    pub action_id: String,
    pub event_type: String,
    pub from_status: Option<FinancialActionStatus>,
    pub to_status: Option<FinancialActionStatus>,
    pub actor_id: Option<String>,
    pub reason: Option<String>,
    pub metadata: serde_json::Value,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct StoredFinancialApprovalRequest {
    pub workspace_id: String,
    pub id: String,
    pub action_id: String,
    pub status: FinancialApprovalRequestStatus,
    pub reason: String,
    pub approver_roles: Vec<String>,
    pub decided_by: Option<String>,
    pub decided_at: Option<DateTime<Utc>>,
    pub expires_at: Option<DateTime<Utc>>,
    pub metadata: serde_json::Value,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct StoredFinancialMandate {
    pub workspace_id: String,
    pub id: String,
    pub version: i32,
    pub status: FinancialMandateStatus,
    pub principal_id: String,
    pub scope: serde_json::Value,
    pub metadata: serde_json::Value,
    pub starts_at: Option<DateTime<Utc>>,
    pub expires_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct StoredFinancialReceipt {
    pub workspace_id: String,
    pub id: String,
    pub action_id: String,
    pub trace_id: Option<String>,
    pub ledger_event_ids: Vec<String>,
    pub proof: serde_json::Value,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct StoredFinancialLedgerEntry {
    pub workspace_id: String,
    pub id: String,
    pub action_id: String,
    pub kind: FinancialLedgerEntryKind,
    pub amount_minor: i64,
    pub currency: String,
    pub idempotency_key: String,
    pub metadata: serde_json::Value,
    pub effective_at: DateTime<Utc>,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct StoredFinancialActionOutcome {
    pub workspace_id: String,
    pub id: String,
    pub outcome: FinancialActionOutcome,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct StoredFinancialExecutionConnector {
    pub connector: FinancialExecutionConnector,
    pub encrypted_secret: String,
}

#[derive(Debug, Clone)]
pub struct FinalizeFinancialExecutionParams {
    pub provider: String,
    pub provider_status: String,
    pub provider_reference: Option<String>,
    pub provider_response: serde_json::Value,
    pub proof: serde_json::Value,
    pub commit_idempotency_key: Option<String>,
    pub attestation_hash: Option<String>,
}

#[derive(Debug, Clone)]
pub struct ReserveAgenticPaymentBudgetRequest {
    pub workspace_id: String,
    pub session_id: String,
    pub principal_id: String,
    pub action_id: String,
    pub payment_requirement_hash: String,
    pub amount: MoneyAmount,
    pub session_limit_minor: i64,
    pub expires_at: DateTime<Utc>,
    pub metadata: serde_json::Value,
}

#[derive(Clone)]
pub struct FinancialRepo {
    pool: DbPool,
}

impl FinancialRepo {
    pub fn new(pool: DbPool) -> Self {
        Self { pool }
    }

    pub async fn create_action(
        &self,
        workspace_id: &str,
        input: CreateFinancialActionRequest,
    ) -> Result<StoredFinancialAction, StorageError> {
        self.create_action_in_environment(workspace_id, tl_core::DEFAULT_ENVIRONMENT_ID, input)
            .await
    }

    pub async fn create_action_in_environment(
        &self,
        workspace_id: &str,
        environment_id: &str,
        input: CreateFinancialActionRequest,
    ) -> Result<StoredFinancialAction, StorageError> {
        validate_create_action(&input)?;
        let action_id = input
            .action
            .id
            .as_deref()
            .map(parse_uuid)
            .transpose()?
            .unwrap_or_else(Uuid::now_v7);
        let idempotency_key = input.idempotency_key.trim().to_string();
        let new_action = NewFinancialAction {
            workspace_id: workspace_id.to_string(),
            id: action_id,
            environment_id: environment_id.to_string(),
            idempotency_key: idempotency_key.clone(),
            principal_id: input.action.principal_id.trim().to_string(),
            action_kind: enum_text(input.action.kind)?,
            operation: input.action.operation.trim().to_string(),
            status: enum_text(FinancialActionStatus::Proposed)?,
            amount_minor: input.action.amount.amount_minor,
            currency: input.action.amount.currency.trim().to_uppercase(),
            counterparty: optional_json(&input.action.counterparty)?,
            mandate: optional_json(&input.action.mandate)?,
            rail: enum_text(input.action.rail)?,
            memo: input.action.memo.and_then(clean_optional),
            metadata: input.action.metadata,
            evidence: serde_json::to_value(input.evidence)
                .map_err(|e| StorageError::Internal(format!("financial evidence encode: {e}")))?,
        };

        let mut conn = self.connection().await?;
        let inserted = diesel::insert_into(financial_actions::table)
            .values(&new_action)
            .on_conflict((
                financial_actions::workspace_id,
                financial_actions::idempotency_key,
            ))
            .do_nothing()
            .execute(&mut conn)
            .await
            .map_err(|e| StorageError::Internal(format!("financial action insert: {e}")))?;

        if inserted > 0 {
            self.insert_event(
                &mut conn,
                workspace_id,
                action_id,
                "created",
                None,
                Some(FinancialActionStatus::Proposed),
                serde_json::json!({}),
            )
            .await?;
        }

        drop(conn);
        self.get_action_by_idempotency_key(workspace_id, &idempotency_key)
            .await
    }

    pub async fn get_action(
        &self,
        workspace_id: &str,
        action_id: &str,
    ) -> Result<StoredFinancialAction, StorageError> {
        let action_uuid = parse_uuid(action_id)?;
        let mut conn = self.connection().await?;
        let record = financial_actions::table
            .filter(financial_actions::workspace_id.eq(workspace_id))
            .filter(financial_actions::id.eq(action_uuid))
            .select(FinancialActionRecord::as_select())
            .first::<FinancialActionRecord>(&mut conn)
            .await
            .optional()
            .map_err(|e| StorageError::Internal(format!("financial action get: {e}")))?
            .ok_or(StorageError::NotFound)?;
        let mut action = action_from_record(record)?;
        action.status_reason = self
            .latest_status_reason(&mut conn, workspace_id, action_id)
            .await?;
        Ok(action)
    }

    pub async fn list_actions(
        &self,
        workspace_id: &str,
    ) -> Result<Vec<StoredFinancialAction>, StorageError> {
        let mut conn = self.connection().await?;
        let rows = financial_actions::table
            .filter(financial_actions::workspace_id.eq(workspace_id))
            .select(FinancialActionRecord::as_select())
            .order((
                financial_actions::created_at.desc(),
                financial_actions::id.desc(),
            ))
            .load::<FinancialActionRecord>(&mut conn)
            .await
            .map_err(|e| StorageError::Internal(format!("financial actions list: {e}")))?;
        let mut actions = rows
            .into_iter()
            .map(action_from_record)
            .collect::<Result<Vec<_>, _>>()?;
        for action in &mut actions {
            action.status_reason = self
                .latest_status_reason(&mut conn, workspace_id, &action.id)
                .await?;
        }
        Ok(actions)
    }

    pub async fn persist_action_evaluation(
        &self,
        workspace_id: &str,
        evaluation: FinancialActionEvaluation,
    ) -> Result<FinancialActionEvaluation, StorageError> {
        let action_id = parse_uuid(&evaluation.action_id)?;
        let row = NewFinancialActionEvaluation {
            workspace_id: workspace_id.to_string(),
            action_id,
            environment_id: evaluation.environment_id.clone(),
            runtime_mode: enum_text(evaluation.runtime_mode)?,
            outcome: enum_text(evaluation.outcome)?,
            reason: evaluation.reason,
            risks: serde_json::to_value(evaluation.risks)
                .map_err(|e| StorageError::Internal(format!("evaluation risks encode: {e}")))?,
            policy_ids: serde_json::to_value(evaluation.policy_ids)
                .map_err(|e| StorageError::Internal(format!("evaluation policies encode: {e}")))?,
            amount_minor: evaluation.amount.amount_minor,
            currency: evaluation.amount.currency,
        };
        let mut conn = self.connection().await?;
        diesel::insert_into(financial_action_evaluations::table)
            .values(&row)
            .on_conflict((
                financial_action_evaluations::workspace_id,
                financial_action_evaluations::action_id,
            ))
            .do_nothing()
            .execute(&mut conn)
            .await
            .map_err(|e| StorageError::Internal(format!("financial evaluation insert: {e}")))?;
        drop(conn);
        self.get_action_evaluation(workspace_id, &action_id.to_string())
            .await
    }

    pub async fn get_action_evaluation(
        &self,
        workspace_id: &str,
        action_id: &str,
    ) -> Result<FinancialActionEvaluation, StorageError> {
        let action_id = parse_uuid(action_id)?;
        let mut conn = self.connection().await?;
        let row = financial_action_evaluations::table
            .filter(financial_action_evaluations::workspace_id.eq(workspace_id))
            .filter(financial_action_evaluations::action_id.eq(action_id))
            .select(FinancialActionEvaluationRecord::as_select())
            .first::<FinancialActionEvaluationRecord>(&mut conn)
            .await
            .optional()
            .map_err(|e| StorageError::Internal(format!("financial evaluation get: {e}")))?
            .ok_or(StorageError::NotFound)?;
        evaluation_from_record(row)
    }

    pub async fn issue_execution_grant(
        &self,
        workspace_id: &str,
        action_id: &str,
        action_hash: &str,
        binding: FinancialExecutionBinding,
        expires_at: DateTime<Utc>,
    ) -> Result<FinancialExecutionGrant, StorageError> {
        let action_id = parse_uuid(action_id)?;
        let row = NewFinancialExecutionGrant {
            workspace_id: workspace_id.to_string(),
            id: Uuid::now_v7(),
            action_id,
            action_hash: clean_required("action_hash", action_hash)?,
            binding: enum_text(binding)?,
            status: enum_text(FinancialExecutionGrantStatus::Issued)?,
            expires_at,
        };
        let mut conn = self.connection().await?;
        diesel::insert_into(financial_execution_grants::table)
            .values(&row)
            .on_conflict((
                financial_execution_grants::workspace_id,
                financial_execution_grants::action_id,
            ))
            .do_nothing()
            .execute(&mut conn)
            .await
            .map_err(|e| StorageError::Internal(format!("execution grant insert: {e}")))?;
        drop(conn);
        let grant = self
            .get_execution_grant(workspace_id, &action_id.to_string())
            .await?;
        if grant.action_hash != action_hash || grant.binding != binding {
            return Err(StorageError::Conflict);
        }
        Ok(grant)
    }

    pub async fn get_execution_grant(
        &self,
        workspace_id: &str,
        action_id: &str,
    ) -> Result<FinancialExecutionGrant, StorageError> {
        let action_id = parse_uuid(action_id)?;
        let mut conn = self.connection().await?;
        let row = financial_execution_grants::table
            .filter(financial_execution_grants::workspace_id.eq(workspace_id))
            .filter(financial_execution_grants::action_id.eq(action_id))
            .select(FinancialExecutionGrantRecord::as_select())
            .first::<FinancialExecutionGrantRecord>(&mut conn)
            .await
            .optional()
            .map_err(|e| StorageError::Internal(format!("execution grant get: {e}")))?
            .ok_or(StorageError::NotFound)?;
        execution_grant_from_record(row)
    }

    pub async fn claim_execution_grant(
        &self,
        workspace_id: &str,
        action_id: &str,
        binding: FinancialExecutionBinding,
        claim_id: &str,
        stale_before: DateTime<Utc>,
    ) -> Result<FinancialExecutionGrant, StorageError> {
        let action_id = parse_uuid(action_id)?;
        let claim_id = parse_uuid(claim_id)?;
        let now = Utc::now();
        let issued = enum_text(FinancialExecutionGrantStatus::Issued)?;
        let claimed = enum_text(FinancialExecutionGrantStatus::Claimed)?;
        let binding = enum_text(binding)?;
        let mut conn = self.connection().await?;
        let updated = diesel::update(
            financial_execution_grants::table
                .filter(financial_execution_grants::workspace_id.eq(workspace_id))
                .filter(financial_execution_grants::action_id.eq(action_id))
                .filter(financial_execution_grants::binding.eq(binding))
                .filter(financial_execution_grants::expires_at.gt(now))
                .filter(
                    financial_execution_grants::status.eq(&issued).or(
                        financial_execution_grants::status.eq(&claimed).and(
                            financial_execution_grants::claimed_at
                                .is_not_null()
                                .and(financial_execution_grants::claimed_at.lt(stale_before)),
                        ),
                    ),
                ),
        )
        .set((
            financial_execution_grants::status.eq(claimed.clone()),
            financial_execution_grants::claim_id.eq(Some(claim_id)),
            financial_execution_grants::claimed_at.eq(Some(now)),
            financial_execution_grants::updated_at.eq(now),
        ))
        .execute(&mut conn)
        .await
        .map_err(|e| StorageError::Internal(format!("execution grant claim: {e}")))?;
        if updated != 1 {
            return Err(StorageError::Conflict);
        }
        drop(conn);
        self.get_execution_grant(workspace_id, &action_id.to_string())
            .await
    }

    pub async fn finalize_execution(
        &self,
        workspace_id: &str,
        action_id: &str,
        grant_id: &str,
        params: FinalizeFinancialExecutionParams,
    ) -> Result<
        (
            StoredFinancialAction,
            FinancialExecutionGrant,
            StoredFinancialReceipt,
        ),
        StorageError,
    > {
        let action_id = parse_uuid(action_id)?;
        let grant_id = parse_uuid(grant_id)?;
        let now = Utc::now();
        let mut conn = self.connection().await?;
        conn.transaction::<_, StorageError, _>(async |conn| {
            let grant = financial_execution_grants::table
                .filter(financial_execution_grants::workspace_id.eq(workspace_id))
                .filter(financial_execution_grants::id.eq(grant_id))
                .filter(financial_execution_grants::action_id.eq(action_id))
                .select(FinancialExecutionGrantRecord::as_select())
                .for_update()
                .first::<FinancialExecutionGrantRecord>(conn)
                .await
                .optional()
                .map_err(|e| StorageError::Internal(format!("finalize grant lock: {e}")))?
                .ok_or(StorageError::NotFound)?;
            let action = financial_actions::table
                .filter(financial_actions::workspace_id.eq(workspace_id))
                .filter(financial_actions::id.eq(action_id))
                .select(FinancialActionRecord::as_select())
                .for_update()
                .first::<FinancialActionRecord>(conn)
                .await
                .optional()
                .map_err(|e| StorageError::Internal(format!("finalize action lock: {e}")))?
                .ok_or(StorageError::NotFound)?;

            let grant_status = enum_from_text::<FinancialExecutionGrantStatus>(&grant.status)?;
            if grant_status == FinancialExecutionGrantStatus::Committed {
                if grant.commit_idempotency_key != params.commit_idempotency_key
                    || grant.attestation_hash != params.attestation_hash
                {
                    return Err(StorageError::Conflict);
                }
                let receipt = financial_receipts::table
                    .filter(financial_receipts::workspace_id.eq(workspace_id))
                    .filter(financial_receipts::id.eq(action_id))
                    .select(FinancialReceiptRecord::as_select())
                    .first::<FinancialReceiptRecord>(conn)
                    .await
                    .map_err(|e| StorageError::Internal(format!("finalize receipt replay: {e}")))?;
                return Ok((
                    action_from_record(action)?,
                    execution_grant_from_record(grant)?,
                    receipt_from_record(receipt)?,
                ));
            }
            if !matches!(
                grant_status,
                FinancialExecutionGrantStatus::Issued | FinancialExecutionGrantStatus::Claimed
            ) || status_from_text(&action.status)? != FinancialActionStatus::Authorized
            {
                return Err(StorageError::Conflict);
            }
            if let Some(existing) = &grant.commit_idempotency_key {
                if Some(existing.as_str()) != params.commit_idempotency_key.as_deref()
                    || grant.attestation_hash != params.attestation_hash
                {
                    return Err(StorageError::Conflict);
                }
            }

            let mut ledger_ids = Vec::new();
            let reserved_key = format!("{action_id}:reserved");
            let reserved_exists = financial_ledger_entries::table
                .filter(financial_ledger_entries::workspace_id.eq(workspace_id))
                .filter(financial_ledger_entries::idempotency_key.eq(&reserved_key))
                .select(financial_ledger_entries::id)
                .first::<Uuid>(conn)
                .await
                .optional()
                .map_err(|e| StorageError::Internal(format!("finalize reserve lookup: {e}")))?
                .is_some();
            if reserved_exists {
                let release_id = Uuid::now_v7();
                let release = NewFinancialLedgerEntry {
                    workspace_id: workspace_id.to_string(),
                    id: release_id,
                    action_id,
                    entry_kind: FinancialLedgerEntryKind::Released.as_str().into(),
                    amount_minor: action.amount_minor,
                    currency: action.currency.clone(),
                    idempotency_key: format!("{action_id}:released"),
                    metadata: serde_json::json!({"source": "execution_finalize"}),
                };
                diesel::insert_into(financial_ledger_entries::table)
                    .values(&release)
                    .on_conflict((
                        financial_ledger_entries::workspace_id,
                        financial_ledger_entries::idempotency_key,
                    ))
                    .do_nothing()
                    .execute(conn)
                    .await
                    .map_err(|e| StorageError::Internal(format!("finalize release: {e}")))?;
                ledger_ids.push(release_id.to_string());
            }
            let executed_id = Uuid::now_v7();
            let executed = NewFinancialLedgerEntry {
                workspace_id: workspace_id.to_string(),
                id: executed_id,
                action_id,
                entry_kind: FinancialLedgerEntryKind::Executed.as_str().into(),
                amount_minor: action.amount_minor,
                currency: action.currency.clone(),
                idempotency_key: format!("{action_id}:executed"),
                metadata: serde_json::json!({"provider": params.provider}),
            };
            diesel::insert_into(financial_ledger_entries::table)
                .values(&executed)
                .on_conflict((
                    financial_ledger_entries::workspace_id,
                    financial_ledger_entries::idempotency_key,
                ))
                .do_nothing()
                .execute(conn)
                .await
                .map_err(|e| StorageError::Internal(format!("finalize executed ledger: {e}")))?;
            ledger_ids.push(executed_id.to_string());

            diesel::update(
                financial_actions::table
                    .filter(financial_actions::workspace_id.eq(workspace_id))
                    .filter(financial_actions::id.eq(action_id)),
            )
            .set((
                financial_actions::status.eq(enum_text(FinancialActionStatus::Executed)?),
                financial_actions::updated_at.eq(now),
            ))
            .execute(conn)
            .await
            .map_err(|e| StorageError::Internal(format!("finalize action: {e}")))?;

            self.insert_event(
                conn,
                workspace_id,
                action_id,
                "execution_committed",
                Some(FinancialActionStatus::Authorized),
                Some(FinancialActionStatus::Executed),
                serde_json::json!({"grant_id": grant_id}),
            )
            .await?;

            let receipt = NewFinancialReceipt {
                workspace_id: workspace_id.to_string(),
                id: action_id,
                action_id,
                trace_id: None,
                ledger_event_ids: serde_json::to_value(&ledger_ids).map_err(|e| {
                    StorageError::Internal(format!("finalize ledger ids encode: {e}"))
                })?,
                proof: params.proof,
            };
            diesel::insert_into(financial_receipts::table)
                .values(&receipt)
                .on_conflict((financial_receipts::workspace_id, financial_receipts::id))
                .do_nothing()
                .execute(conn)
                .await
                .map_err(|e| StorageError::Internal(format!("finalize receipt: {e}")))?;

            let outcome = NewFinancialActionOutcome {
                workspace_id: workspace_id.to_string(),
                id: Uuid::now_v7(),
                action_id,
                status: enum_text(FinancialActionOutcomeStatus::Succeeded)?,
                reversal_capability: enum_text(ReversalCapability::None)?,
                recovery_status: enum_text(RecoveryStatus::NotAvailable)?,
                provider_status: Some(params.provider_status),
                provider_reference: params.provider_reference,
                final_loss_amount_minor: None,
                final_loss_currency: None,
                occurred_at: now,
                metadata: params.provider_response,
            };
            diesel::insert_into(financial_action_outcomes::table)
                .values(&outcome)
                .execute(conn)
                .await
                .map_err(|e| StorageError::Internal(format!("finalize outcome: {e}")))?;

            diesel::update(
                financial_execution_grants::table
                    .filter(financial_execution_grants::workspace_id.eq(workspace_id))
                    .filter(financial_execution_grants::id.eq(grant_id)),
            )
            .set((
                financial_execution_grants::status
                    .eq(enum_text(FinancialExecutionGrantStatus::Committed)?),
                financial_execution_grants::claim_id.eq(None::<Uuid>),
                financial_execution_grants::claimed_at.eq(None::<DateTime<Utc>>),
                financial_execution_grants::commit_idempotency_key
                    .eq(params.commit_idempotency_key),
                financial_execution_grants::attestation_hash.eq(params.attestation_hash),
                financial_execution_grants::committed_at.eq(Some(now)),
                financial_execution_grants::updated_at.eq(now),
            ))
            .execute(conn)
            .await
            .map_err(|e| StorageError::Internal(format!("finalize grant: {e}")))?;

            let action = financial_actions::table
                .filter(financial_actions::workspace_id.eq(workspace_id))
                .filter(financial_actions::id.eq(action_id))
                .select(FinancialActionRecord::as_select())
                .first::<FinancialActionRecord>(conn)
                .await
                .map_err(|e| StorageError::Internal(format!("finalized action get: {e}")))?;
            let grant = financial_execution_grants::table
                .filter(financial_execution_grants::workspace_id.eq(workspace_id))
                .filter(financial_execution_grants::id.eq(grant_id))
                .select(FinancialExecutionGrantRecord::as_select())
                .first::<FinancialExecutionGrantRecord>(conn)
                .await
                .map_err(|e| StorageError::Internal(format!("finalized grant get: {e}")))?;
            let receipt = financial_receipts::table
                .filter(financial_receipts::workspace_id.eq(workspace_id))
                .filter(financial_receipts::id.eq(action_id))
                .select(FinancialReceiptRecord::as_select())
                .first::<FinancialReceiptRecord>(conn)
                .await
                .map_err(|e| StorageError::Internal(format!("finalized receipt get: {e}")))?;
            Ok((
                action_from_record(action)?,
                execution_grant_from_record(grant)?,
                receipt_from_record(receipt)?,
            ))
        })
        .await
    }

    pub async fn fail_execution_grant(
        &self,
        workspace_id: &str,
        action_id: &str,
        grant_id: &str,
        reason: &str,
    ) -> Result<StoredFinancialAction, StorageError> {
        let action_id = parse_uuid(action_id)?;
        let grant_id = parse_uuid(grant_id)?;
        let now = Utc::now();
        let mut conn = self.connection().await?;
        conn.transaction::<_, StorageError, _>(async |conn| {
            let updated = diesel::update(
                financial_execution_grants::table
                    .filter(financial_execution_grants::workspace_id.eq(workspace_id))
                    .filter(financial_execution_grants::id.eq(grant_id))
                    .filter(financial_execution_grants::action_id.eq(action_id))
                    .filter(
                        financial_execution_grants::status
                            .ne(enum_text(FinancialExecutionGrantStatus::Committed)?),
                    ),
            )
            .set((
                financial_execution_grants::status
                    .eq(enum_text(FinancialExecutionGrantStatus::Failed)?),
                financial_execution_grants::claim_id.eq(None::<Uuid>),
                financial_execution_grants::claimed_at.eq(None::<DateTime<Utc>>),
                financial_execution_grants::updated_at.eq(now),
            ))
            .execute(conn)
            .await
            .map_err(|e| StorageError::Internal(format!("fail execution grant: {e}")))?;
            if updated != 1 {
                return Err(StorageError::Conflict);
            }
            diesel::update(
                financial_actions::table
                    .filter(financial_actions::workspace_id.eq(workspace_id))
                    .filter(financial_actions::id.eq(action_id))
                    .filter(
                        financial_actions::status.eq(enum_text(FinancialActionStatus::Authorized)?),
                    ),
            )
            .set((
                financial_actions::status.eq(enum_text(FinancialActionStatus::Failed)?),
                financial_actions::updated_at.eq(now),
            ))
            .execute(conn)
            .await
            .map_err(|e| StorageError::Internal(format!("fail execution action: {e}")))?;
            self.insert_event(
                conn,
                workspace_id,
                action_id,
                "execution_failed",
                Some(FinancialActionStatus::Authorized),
                Some(FinancialActionStatus::Failed),
                serde_json::json!({"reason": reason, "grant_id": grant_id}),
            )
            .await?;
            let outcome = NewFinancialActionOutcome {
                workspace_id: workspace_id.to_string(),
                id: Uuid::now_v7(),
                action_id,
                status: enum_text(FinancialActionOutcomeStatus::Failed)?,
                reversal_capability: enum_text(ReversalCapability::None)?,
                recovery_status: enum_text(RecoveryStatus::NotAvailable)?,
                provider_status: Some("failed".into()),
                provider_reference: None,
                final_loss_amount_minor: None,
                final_loss_currency: None,
                occurred_at: now,
                metadata: serde_json::json!({"reason": reason}),
            };
            diesel::insert_into(financial_action_outcomes::table)
                .values(&outcome)
                .execute(conn)
                .await
                .map_err(|e| StorageError::Internal(format!("failed outcome insert: {e}")))?;
            let action = financial_actions::table
                .filter(financial_actions::workspace_id.eq(workspace_id))
                .filter(financial_actions::id.eq(action_id))
                .select(FinancialActionRecord::as_select())
                .first::<FinancialActionRecord>(conn)
                .await
                .map_err(|e| StorageError::Internal(format!("failed action get: {e}")))?;
            action_from_record(action)
        })
        .await
    }

    pub async fn create_execution_connector(
        &self,
        workspace_id: &str,
        display_name: &str,
        encrypted_secret: &str,
        allowed_rails: Vec<FinancialRail>,
        allowed_operations: Vec<String>,
    ) -> Result<StoredFinancialExecutionConnector, StorageError> {
        let id = Uuid::now_v7();
        let row = NewFinancialExecutionConnector {
            workspace_id: workspace_id.to_string(),
            id,
            display_name: clean_required("display_name", display_name)?,
            encrypted_secret: clean_required("encrypted_secret", encrypted_secret)?,
            allowed_rails: serde_json::to_value(allowed_rails)
                .map_err(|e| StorageError::Internal(format!("connector rails encode: {e}")))?,
            allowed_operations: serde_json::to_value(allowed_operations)
                .map_err(|e| StorageError::Internal(format!("connector operations encode: {e}")))?,
            status: enum_text(FinancialExecutionConnectorStatus::Active)?,
        };
        let mut conn = self.connection().await?;
        diesel::insert_into(financial_execution_connectors::table)
            .values(&row)
            .execute(&mut conn)
            .await
            .map_err(|e| StorageError::Internal(format!("execution connector insert: {e}")))?;
        drop(conn);
        self.get_execution_connector(workspace_id, &id.to_string())
            .await
    }

    pub async fn list_execution_connectors(
        &self,
        workspace_id: &str,
    ) -> Result<Vec<FinancialExecutionConnector>, StorageError> {
        let mut conn = self.connection().await?;
        financial_execution_connectors::table
            .filter(financial_execution_connectors::workspace_id.eq(workspace_id))
            .select(FinancialExecutionConnectorRecord::as_select())
            .order(financial_execution_connectors::created_at.desc())
            .load::<FinancialExecutionConnectorRecord>(&mut conn)
            .await
            .map_err(|e| StorageError::Internal(format!("execution connectors list: {e}")))?
            .into_iter()
            .map(|row| connector_from_record(row).map(|stored| stored.connector))
            .collect()
    }

    pub async fn get_execution_connector(
        &self,
        workspace_id: &str,
        connector_id: &str,
    ) -> Result<StoredFinancialExecutionConnector, StorageError> {
        let connector_id = parse_uuid(connector_id)?;
        let mut conn = self.connection().await?;
        let row = financial_execution_connectors::table
            .filter(financial_execution_connectors::workspace_id.eq(workspace_id))
            .filter(financial_execution_connectors::id.eq(connector_id))
            .select(FinancialExecutionConnectorRecord::as_select())
            .first::<FinancialExecutionConnectorRecord>(&mut conn)
            .await
            .optional()
            .map_err(|e| StorageError::Internal(format!("execution connector get: {e}")))?
            .ok_or(StorageError::NotFound)?;
        connector_from_record(row)
    }

    pub async fn revoke_execution_connector(
        &self,
        workspace_id: &str,
        connector_id: &str,
    ) -> Result<FinancialExecutionConnector, StorageError> {
        let connector_id = parse_uuid(connector_id)?;
        let now = Utc::now();
        let mut conn = self.connection().await?;
        let updated = diesel::update(
            financial_execution_connectors::table
                .filter(financial_execution_connectors::workspace_id.eq(workspace_id))
                .filter(financial_execution_connectors::id.eq(connector_id)),
        )
        .set((
            financial_execution_connectors::status
                .eq(enum_text(FinancialExecutionConnectorStatus::Revoked)?),
            financial_execution_connectors::revoked_at.eq(Some(now)),
            financial_execution_connectors::updated_at.eq(now),
        ))
        .execute(&mut conn)
        .await
        .map_err(|e| StorageError::Internal(format!("execution connector revoke: {e}")))?;
        if updated != 1 {
            return Err(StorageError::NotFound);
        }
        drop(conn);
        Ok(self
            .get_execution_connector(workspace_id, &connector_id.to_string())
            .await?
            .connector)
    }

    pub async fn create_observation_review(
        &self,
        workspace_id: &str,
        action_id: &str,
        outcome: FinancialObservationReviewOutcome,
        note: Option<String>,
        reviewed_by: &str,
    ) -> Result<FinancialObservationReview, StorageError> {
        let action_id = parse_uuid(action_id)?;
        let id = Uuid::now_v7();
        let row = NewFinancialObservationReview {
            workspace_id: workspace_id.to_string(),
            id,
            action_id,
            outcome: enum_text(outcome)?,
            note,
            reviewed_by: clean_required("reviewed_by", reviewed_by)?,
        };
        let mut conn = self.connection().await?;
        diesel::insert_into(financial_observation_reviews::table)
            .values(&row)
            .execute(&mut conn)
            .await
            .map_err(|e| StorageError::Internal(format!("observation review insert: {e}")))?;
        let row = financial_observation_reviews::table
            .filter(financial_observation_reviews::workspace_id.eq(workspace_id))
            .filter(financial_observation_reviews::id.eq(id))
            .select(FinancialObservationReviewRecord::as_select())
            .first::<FinancialObservationReviewRecord>(&mut conn)
            .await
            .map_err(|e| StorageError::Internal(format!("observation review get: {e}")))?;
        observation_review_from_record(row)
    }

    pub async fn list_observation_reviews(
        &self,
        workspace_id: &str,
        action_id: &str,
    ) -> Result<Vec<FinancialObservationReview>, StorageError> {
        let action_id = parse_uuid(action_id)?;
        let mut conn = self.connection().await?;
        financial_observation_reviews::table
            .filter(financial_observation_reviews::workspace_id.eq(workspace_id))
            .filter(financial_observation_reviews::action_id.eq(action_id))
            .select(FinancialObservationReviewRecord::as_select())
            .order((
                financial_observation_reviews::created_at.desc(),
                financial_observation_reviews::id.desc(),
            ))
            .load::<FinancialObservationReviewRecord>(&mut conn)
            .await
            .map_err(|e| StorageError::Internal(format!("observation reviews list: {e}")))?
            .into_iter()
            .map(observation_review_from_record)
            .collect()
    }

    pub async fn observation_summary(
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
        StorageError,
    > {
        let mut conn = self.connection().await?;
        let evaluation_rows = financial_action_evaluations::table
            .filter(financial_action_evaluations::workspace_id.eq(workspace_id))
            .filter(financial_action_evaluations::environment_id.eq(environment_id))
            .filter(
                financial_action_evaluations::runtime_mode
                    .eq(enum_text(FinancialRuntimeMode::Observe)?),
            )
            .filter(financial_action_evaluations::created_at.ge(start))
            .filter(financial_action_evaluations::created_at.lt(end))
            .select(FinancialActionEvaluationRecord::as_select())
            .load::<FinancialActionEvaluationRecord>(&mut conn)
            .await
            .map_err(|e| StorageError::Internal(format!("observation evaluations list: {e}")))?;
        let action_ids = evaluation_rows
            .iter()
            .map(|row| row.action_id)
            .collect::<Vec<_>>();
        let review_rows = if action_ids.is_empty() {
            Vec::new()
        } else {
            financial_observation_reviews::table
                .filter(financial_observation_reviews::workspace_id.eq(workspace_id))
                .filter(financial_observation_reviews::action_id.eq_any(action_ids))
                .select(FinancialObservationReviewRecord::as_select())
                .order((
                    financial_observation_reviews::created_at.desc(),
                    financial_observation_reviews::id.desc(),
                ))
                .load::<FinancialObservationReviewRecord>(&mut conn)
                .await
                .map_err(|e| {
                    StorageError::Internal(format!("observation latest reviews list: {e}"))
                })?
        };
        let mut latest_reviews = HashMap::new();
        for review in review_rows {
            latest_reviews.entry(review.action_id).or_insert(review);
        }

        let mut currencies = HashMap::new();
        let mut reasons: HashMap<(String, FinancialEvaluationOutcome, String), (i64, i64)> =
            HashMap::new();
        for row in evaluation_rows {
            let evaluation = evaluation_from_record(row)?;
            let currency = evaluation.amount.currency.clone();
            let summary = currencies
                .entry(currency.clone())
                .or_insert_with(|| empty_observation_currency(&currency));
            summary.total_observed_count += 1;
            summary.total_observed_amount_minor += evaluation.amount.amount_minor;
            match evaluation.outcome {
                FinancialEvaluationOutcome::WouldAllow => {
                    summary.would_allow_count += 1;
                    summary.would_allow_amount_minor += evaluation.amount.amount_minor;
                }
                FinancialEvaluationOutcome::WouldHold => {
                    summary.would_hold_count += 1;
                    summary.would_hold_amount_minor += evaluation.amount.amount_minor;
                    summary.adverse_count += 1;
                    summary.estimated_approval_count += 1;
                }
                FinancialEvaluationOutcome::WouldBlock => {
                    summary.would_block_count += 1;
                    summary.would_block_amount_minor += evaluation.amount.amount_minor;
                    summary.adverse_count += 1;
                }
                _ => continue,
            }
            if let Some(review) = latest_reviews.get(&parse_uuid(&evaluation.action_id)?) {
                summary.reviewed_adverse_count += 1;
                if enum_from_text::<FinancialObservationReviewOutcome>(&review.outcome)?
                    == FinancialObservationReviewOutcome::FalsePositive
                {
                    summary.false_positive_count += 1;
                }
            }
            let reason = reasons
                .entry((evaluation.reason, evaluation.outcome, currency))
                .or_default();
            reason.0 += 1;
            reason.1 += evaluation.amount.amount_minor;
        }
        for summary in currencies.values_mut() {
            summary.adverse_rate_bps =
                observation_rate_bps(summary.adverse_count, summary.total_observed_count);
            summary.estimated_approval_rate_bps = observation_rate_bps(
                summary.estimated_approval_count,
                summary.total_observed_count,
            );
            summary.false_positive_rate_bps =
                observation_rate_bps(summary.false_positive_count, summary.reviewed_adverse_count);
        }
        let reasons = reasons
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
        Ok((currencies.into_values().collect(), reasons))
    }

    pub async fn create_approval_request(
        &self,
        workspace_id: &str,
        action_id: &str,
        reason: &str,
        approver_roles: Vec<String>,
        expires_at: Option<DateTime<Utc>>,
        metadata: serde_json::Value,
    ) -> Result<StoredFinancialApprovalRequest, StorageError> {
        let action_uuid = parse_uuid(action_id)?;
        let clean_reason = clean_required("reason", reason)?;
        if approver_roles.iter().any(|role| role.trim().is_empty()) {
            return Err(StorageError::Internal(
                "approver_roles must not contain empty roles".into(),
            ));
        }
        let id = Uuid::now_v7();
        let new_request = NewApprovalRequest {
            workspace_id: workspace_id.to_string(),
            id,
            action_id: action_uuid,
            status: enum_text(FinancialApprovalRequestStatus::Pending)?,
            reason: clean_reason,
            approver_roles: serde_json::to_value(approver_roles)
                .map_err(|e| StorageError::Internal(format!("approver roles encode: {e}")))?,
            expires_at,
            metadata,
        };

        let mut conn = self.connection().await?;
        diesel::insert_into(approval_requests::table)
            .values(&new_request)
            .execute(&mut conn)
            .await
            .map_err(|e| StorageError::Internal(format!("approval request insert: {e}")))?;
        let row = approval_requests::table
            .filter(approval_requests::workspace_id.eq(workspace_id))
            .filter(approval_requests::id.eq(id))
            .select(ApprovalRequestRecord::as_select())
            .first::<ApprovalRequestRecord>(&mut conn)
            .await
            .map_err(|e| StorageError::Internal(format!("approval request get: {e}")))?;
        approval_from_record(row)
    }

    pub async fn create_mandate(
        &self,
        workspace_id: &str,
        input: CreateFinancialMandateRequest,
    ) -> Result<StoredFinancialMandate, StorageError> {
        let id = input
            .id
            .and_then(clean_optional)
            .unwrap_or_else(|| Uuid::now_v7().to_string());
        let version = input.version.unwrap_or(1);
        if version <= 0 {
            return Err(StorageError::Internal(
                "mandate version must be positive".into(),
            ));
        }
        let starts_at = parse_optional_rfc3339("starts_at", input.starts_at.as_deref())?;
        let expires_at = parse_optional_rfc3339("expires_at", input.expires_at.as_deref())?;
        let new_mandate = NewMandate {
            workspace_id: workspace_id.to_string(),
            id: id.clone(),
            version,
            status: enum_text(FinancialMandateStatus::Active)?,
            principal_id: clean_required("principal_id", &input.principal_id)?,
            scope: input.scope,
            metadata: input.metadata,
            starts_at,
            expires_at,
        };

        let mut conn = self.connection().await?;
        diesel::insert_into(mandates::table)
            .values(&new_mandate)
            .execute(&mut conn)
            .await
            .map_err(|e| StorageError::Internal(format!("mandate insert: {e}")))?;
        drop(conn);
        self.get_mandate(workspace_id, &id, Some(version)).await
    }

    pub async fn list_mandates(
        &self,
        workspace_id: &str,
    ) -> Result<Vec<StoredFinancialMandate>, StorageError> {
        let mut conn = self.connection().await?;
        let rows = mandates::table
            .filter(mandates::workspace_id.eq(workspace_id))
            .order((mandates::created_at.desc(), mandates::id.desc()))
            .select(MandateRecord::as_select())
            .load::<MandateRecord>(&mut conn)
            .await
            .map_err(|e| StorageError::Internal(format!("mandates list: {e}")))?;
        rows.into_iter().map(mandate_from_record).collect()
    }

    pub async fn get_mandate(
        &self,
        workspace_id: &str,
        mandate_id: &str,
        version: Option<i32>,
    ) -> Result<StoredFinancialMandate, StorageError> {
        let clean_id = clean_required("mandate_id", mandate_id)?;
        let mut conn = self.connection().await?;
        let mut query = mandates::table
            .filter(mandates::workspace_id.eq(workspace_id))
            .filter(mandates::id.eq(&clean_id))
            .into_boxed();
        if let Some(version) = version {
            query = query.filter(mandates::version.eq(version));
        }
        let record = query
            .order(mandates::version.desc())
            .select(MandateRecord::as_select())
            .first::<MandateRecord>(&mut conn)
            .await
            .optional()
            .map_err(|e| StorageError::Internal(format!("mandate get: {e}")))?
            .ok_or(StorageError::NotFound)?;
        mandate_from_record(record)
    }

    pub async fn revoke_mandate(
        &self,
        workspace_id: &str,
        mandate_id: &str,
    ) -> Result<StoredFinancialMandate, StorageError> {
        let clean_id = clean_required("mandate_id", mandate_id)?;
        let mut conn = self.connection().await?;
        let current = mandates::table
            .filter(mandates::workspace_id.eq(workspace_id))
            .filter(mandates::id.eq(&clean_id))
            .order(mandates::version.desc())
            .select(MandateRecord::as_select())
            .first::<MandateRecord>(&mut conn)
            .await
            .optional()
            .map_err(|e| StorageError::Internal(format!("mandate get: {e}")))?
            .ok_or(StorageError::NotFound)?;

        diesel::update(
            mandates::table
                .filter(mandates::workspace_id.eq(workspace_id))
                .filter(mandates::id.eq(&clean_id))
                .filter(mandates::version.eq(current.version)),
        )
        .set((
            mandates::status.eq(enum_text(FinancialMandateStatus::Revoked)?),
            mandates::updated_at.eq(Utc::now()),
        ))
        .execute(&mut conn)
        .await
        .map_err(|e| StorageError::Internal(format!("mandate revoke: {e}")))?;
        drop(conn);
        self.get_mandate(workspace_id, &clean_id, Some(current.version))
            .await
    }

    pub async fn create_receipt(
        &self,
        workspace_id: &str,
        action_id: &str,
        trace_id: Option<&str>,
        ledger_event_ids: Vec<String>,
        proof: serde_json::Value,
    ) -> Result<StoredFinancialReceipt, StorageError> {
        let action_uuid = parse_uuid(action_id)?;
        let trace_uuid = trace_id.map(parse_uuid).transpose()?;
        let mut conn = self.connection().await?;
        let action_exists = financial_actions::table
            .filter(financial_actions::workspace_id.eq(workspace_id))
            .filter(financial_actions::id.eq(action_uuid))
            .select(financial_actions::id)
            .first::<Uuid>(&mut conn)
            .await
            .optional()
            .map_err(|e| StorageError::Internal(format!("financial receipt action lookup: {e}")))?
            .is_some();
        if !action_exists {
            return Err(StorageError::NotFound);
        }

        let receipt = NewFinancialReceipt {
            workspace_id: workspace_id.to_string(),
            id: action_uuid,
            action_id: action_uuid,
            trace_id: trace_uuid,
            ledger_event_ids: serde_json::to_value(ledger_event_ids)
                .map_err(|e| StorageError::Internal(format!("ledger ids encode: {e}")))?,
            proof,
        };
        diesel::insert_into(financial_receipts::table)
            .values(&receipt)
            .on_conflict((financial_receipts::workspace_id, financial_receipts::id))
            .do_nothing()
            .execute(&mut conn)
            .await
            .map_err(|e| StorageError::Internal(format!("financial receipt insert: {e}")))?;
        drop(conn);
        self.get_receipt(workspace_id, action_id).await
    }

    pub async fn get_receipt(
        &self,
        workspace_id: &str,
        receipt_id: &str,
    ) -> Result<StoredFinancialReceipt, StorageError> {
        let receipt_uuid = parse_uuid(receipt_id)?;
        let mut conn = self.connection().await?;
        let row = financial_receipts::table
            .filter(financial_receipts::workspace_id.eq(workspace_id))
            .filter(financial_receipts::id.eq(receipt_uuid))
            .select(FinancialReceiptRecord::as_select())
            .first::<FinancialReceiptRecord>(&mut conn)
            .await
            .optional()
            .map_err(|e| StorageError::Internal(format!("financial receipt get: {e}")))?
            .ok_or(StorageError::NotFound)?;
        receipt_from_record(row)
    }

    pub async fn record_action_outcome(
        &self,
        workspace_id: &str,
        action_id: &str,
        outcome: FinancialActionOutcome,
    ) -> Result<StoredFinancialActionOutcome, StorageError> {
        if outcome.action_id != action_id {
            return Err(StorageError::Internal(
                "outcome action_id must match path action id".into(),
            ));
        }
        let action_uuid = parse_uuid(action_id)?;
        let occurred_at = parse_rfc3339("occurred_at", &outcome.occurred_at)?;
        let final_loss_amount_minor = outcome
            .final_loss_amount
            .as_ref()
            .map(|amount| amount.amount_minor);
        let final_loss_currency = outcome
            .final_loss_amount
            .as_ref()
            .map(|amount| amount.currency.trim().to_uppercase());
        let new_outcome = NewFinancialActionOutcome {
            workspace_id: workspace_id.to_string(),
            id: Uuid::now_v7(),
            action_id: action_uuid,
            status: enum_text(outcome.status)?,
            reversal_capability: enum_text(outcome.reversal_capability)?,
            recovery_status: enum_text(outcome.recovery_status)?,
            provider_status: outcome.provider_status.and_then(clean_optional),
            provider_reference: outcome.provider_reference.and_then(clean_optional),
            final_loss_amount_minor,
            final_loss_currency,
            occurred_at,
            metadata: outcome.metadata,
        };

        let mut conn = self.connection().await?;
        let action_exists = financial_actions::table
            .filter(financial_actions::workspace_id.eq(workspace_id))
            .filter(financial_actions::id.eq(action_uuid))
            .select(financial_actions::id)
            .first::<Uuid>(&mut conn)
            .await
            .optional()
            .map_err(|e| StorageError::Internal(format!("financial outcome action lookup: {e}")))?
            .is_some();
        if !action_exists {
            return Err(StorageError::NotFound);
        }

        let row = diesel::insert_into(financial_action_outcomes::table)
            .values(&new_outcome)
            .returning(FinancialActionOutcomeRecord::as_returning())
            .get_result::<FinancialActionOutcomeRecord>(&mut conn)
            .await
            .map_err(|e| StorageError::Internal(format!("financial outcome insert: {e}")))?;
        outcome_from_record(row)
    }

    pub async fn list_action_outcomes(
        &self,
        workspace_id: &str,
        action_id: &str,
    ) -> Result<Vec<StoredFinancialActionOutcome>, StorageError> {
        let action_uuid = parse_uuid(action_id)?;
        let mut conn = self.connection().await?;
        let action_exists = financial_actions::table
            .filter(financial_actions::workspace_id.eq(workspace_id))
            .filter(financial_actions::id.eq(action_uuid))
            .select(financial_actions::id)
            .first::<Uuid>(&mut conn)
            .await
            .optional()
            .map_err(|e| StorageError::Internal(format!("financial outcome action lookup: {e}")))?
            .is_some();
        if !action_exists {
            return Err(StorageError::NotFound);
        }

        let rows = financial_action_outcomes::table
            .filter(financial_action_outcomes::workspace_id.eq(workspace_id))
            .filter(financial_action_outcomes::action_id.eq(action_uuid))
            .order((
                financial_action_outcomes::occurred_at.desc(),
                financial_action_outcomes::created_at.desc(),
                financial_action_outcomes::id.desc(),
            ))
            .select(FinancialActionOutcomeRecord::as_select())
            .load::<FinancialActionOutcomeRecord>(&mut conn)
            .await
            .map_err(|e| StorageError::Internal(format!("financial outcomes list: {e}")))?;
        rows.into_iter().map(outcome_from_record).collect()
    }

    pub async fn list_approval_requests(
        &self,
        workspace_id: &str,
    ) -> Result<Vec<StoredFinancialApprovalRequest>, StorageError> {
        let mut conn = self.connection().await?;
        let rows = approval_requests::table
            .filter(approval_requests::workspace_id.eq(workspace_id))
            .order((
                approval_requests::created_at.desc(),
                approval_requests::id.desc(),
            ))
            .select(ApprovalRequestRecord::as_select())
            .load::<ApprovalRequestRecord>(&mut conn)
            .await
            .map_err(|e| StorageError::Internal(format!("approval requests list: {e}")))?;
        rows.into_iter().map(approval_from_record).collect()
    }

    pub async fn resolve_pending_approval_requests(
        &self,
        workspace_id: &str,
        action_id: &str,
        status: FinancialApprovalRequestStatus,
        decided_by: Option<&str>,
    ) -> Result<(), StorageError> {
        if !matches!(
            status,
            FinancialApprovalRequestStatus::Approved | FinancialApprovalRequestStatus::Denied
        ) {
            return Err(StorageError::Internal(
                "approval request resolution must be approved or denied".into(),
            ));
        }
        let action_uuid = parse_uuid(action_id)?;
        let mut conn = self.connection().await?;
        let action_exists = financial_actions::table
            .filter(financial_actions::workspace_id.eq(workspace_id))
            .filter(financial_actions::id.eq(action_uuid))
            .select(financial_actions::id)
            .first::<Uuid>(&mut conn)
            .await
            .optional()
            .map_err(|e| StorageError::Internal(format!("approval action lookup: {e}")))?
            .is_some();
        if !action_exists {
            return Err(StorageError::NotFound);
        }

        diesel::update(
            approval_requests::table
                .filter(approval_requests::workspace_id.eq(workspace_id))
                .filter(approval_requests::action_id.eq(action_uuid))
                .filter(
                    approval_requests::status
                        .eq(enum_text(FinancialApprovalRequestStatus::Pending)?),
                ),
        )
        .set((
            approval_requests::status.eq(enum_text(status)?),
            approval_requests::decided_by.eq(decided_by.map(str::to_string)),
            approval_requests::decided_at.eq(Some(Utc::now())),
            approval_requests::updated_at.eq(Utc::now()),
        ))
        .execute(&mut conn)
        .await
        .map_err(|e| StorageError::Internal(format!("approval request resolve: {e}")))?;
        Ok(())
    }

    pub async fn transition_status(
        &self,
        workspace_id: &str,
        action_id: &str,
        next_status: FinancialActionStatus,
        event_type: &str,
        metadata: serde_json::Value,
    ) -> Result<StoredFinancialAction, StorageError> {
        let action_uuid = parse_uuid(action_id)?;
        let clean_event_type = clean_required("event_type", event_type)?;
        let mut conn = self.connection().await?;
        let record = financial_actions::table
            .filter(financial_actions::workspace_id.eq(workspace_id))
            .filter(financial_actions::id.eq(action_uuid))
            .select(FinancialActionRecord::as_select())
            .first::<FinancialActionRecord>(&mut conn)
            .await
            .optional()
            .map_err(|e| StorageError::Internal(format!("financial action transition get: {e}")))?
            .ok_or(StorageError::NotFound)?;
        let current_status = status_from_text(&record.status)?;
        if !is_valid_transition(current_status, next_status) {
            return Err(StorageError::Conflict);
        }

        diesel::update(
            financial_actions::table
                .filter(financial_actions::workspace_id.eq(workspace_id))
                .filter(financial_actions::id.eq(action_uuid))
                .filter(financial_actions::status.eq(enum_text(current_status)?)),
        )
        .set((
            financial_actions::status.eq(enum_text(next_status)?),
            financial_actions::updated_at.eq(Utc::now()),
        ))
        .execute(&mut conn)
        .await
        .map_err(|e| StorageError::Internal(format!("financial action transition update: {e}")))
        .and_then(|updated| {
            if updated == 0 {
                Err(StorageError::Conflict)
            } else {
                Ok(updated)
            }
        })?;

        self.insert_event(
            &mut conn,
            workspace_id,
            action_uuid,
            &clean_event_type,
            Some(current_status),
            Some(next_status),
            metadata,
        )
        .await?;

        drop(conn);
        self.get_action(workspace_id, action_id).await
    }

    pub async fn list_action_events(
        &self,
        workspace_id: &str,
        action_id: &str,
    ) -> Result<Vec<StoredFinancialActionEvent>, StorageError> {
        let action_uuid = parse_uuid(action_id)?;
        let mut conn = self.connection().await?;
        let rows = financial_action_events::table
            .filter(financial_action_events::workspace_id.eq(workspace_id))
            .filter(financial_action_events::action_id.eq(action_uuid))
            .select(FinancialActionEventRecord::as_select())
            .order(financial_action_events::created_at.asc())
            .load::<FinancialActionEventRecord>(&mut conn)
            .await
            .map_err(|e| StorageError::Internal(format!("financial action events list: {e}")))?;
        rows.into_iter().map(event_from_record).collect()
    }

    async fn latest_status_reason(
        &self,
        conn: &mut DbConnection<'_>,
        workspace_id: &str,
        action_id: &str,
    ) -> Result<Option<String>, StorageError> {
        let action_uuid = parse_uuid(action_id)?;
        let row = financial_action_events::table
            .filter(financial_action_events::workspace_id.eq(workspace_id))
            .filter(financial_action_events::action_id.eq(action_uuid))
            .filter(financial_action_events::reason.is_not_null())
            .select(FinancialActionEventRecord::as_select())
            .order(financial_action_events::created_at.desc())
            .first::<FinancialActionEventRecord>(conn)
            .await
            .optional()
            .map_err(|e| StorageError::Internal(format!("financial action reason get: {e}")))?;
        Ok(row.and_then(|event| event.reason))
    }

    pub async fn record_ledger_entry(
        &self,
        workspace_id: &str,
        action_id: &str,
        kind: FinancialLedgerEntryKind,
        amount_minor: i64,
        currency: &str,
        idempotency_key: &str,
        metadata: serde_json::Value,
    ) -> Result<StoredFinancialLedgerEntry, StorageError> {
        if amount_minor < 0 {
            return Err(StorageError::Internal(
                "financial ledger amount must be non-negative".into(),
            ));
        }
        let action_uuid = parse_uuid(action_id)?;
        let clean_currency = clean_required("currency", currency)?.to_uppercase();
        let clean_idempotency_key = clean_required("idempotency_key", idempotency_key)?;
        let mut conn = self.connection().await?;
        let action_exists = financial_actions::table
            .filter(financial_actions::workspace_id.eq(workspace_id))
            .filter(financial_actions::id.eq(action_uuid))
            .select(financial_actions::id)
            .first::<Uuid>(&mut conn)
            .await
            .optional()
            .map_err(|e| StorageError::Internal(format!("financial ledger action lookup: {e}")))?
            .is_some();
        if !action_exists {
            return Err(StorageError::NotFound);
        }

        let entry = NewFinancialLedgerEntry {
            workspace_id: workspace_id.to_string(),
            id: Uuid::now_v7(),
            action_id: action_uuid,
            entry_kind: kind.as_str().to_string(),
            amount_minor,
            currency: clean_currency.clone(),
            idempotency_key: clean_idempotency_key.clone(),
            metadata,
        };
        diesel::insert_into(financial_ledger_entries::table)
            .values(&entry)
            .on_conflict((
                financial_ledger_entries::workspace_id,
                financial_ledger_entries::idempotency_key,
            ))
            .do_nothing()
            .execute(&mut conn)
            .await
            .map_err(|e| StorageError::Internal(format!("financial ledger insert: {e}")))?;
        drop(conn);
        let existing = self
            .get_ledger_entry_by_idempotency_key(workspace_id, &clean_idempotency_key)
            .await?;
        if existing.action_id != action_id
            || existing.kind != kind
            || existing.amount_minor != amount_minor
            || existing.currency != clean_currency
        {
            return Err(StorageError::Conflict);
        }
        Ok(existing)
    }

    pub async fn net_spend_minor(
        &self,
        workspace_id: &str,
        principal_id: &str,
        currency: &str,
        start: DateTime<Utc>,
        end: DateTime<Utc>,
    ) -> Result<i64, StorageError> {
        let clean_principal = clean_required("principal_id", principal_id)?;
        let clean_currency = clean_required("currency", currency)?.to_uppercase();
        let mut conn = self.connection().await?;
        let action_ids = financial_actions::table
            .filter(financial_actions::workspace_id.eq(workspace_id))
            .filter(financial_actions::principal_id.eq(clean_principal))
            .select(financial_actions::id)
            .load::<Uuid>(&mut conn)
            .await
            .map_err(|e| StorageError::Internal(format!("financial spend actions: {e}")))?;
        if action_ids.is_empty() {
            return Ok(0);
        }
        let rows = financial_ledger_entries::table
            .filter(financial_ledger_entries::workspace_id.eq(workspace_id))
            .filter(financial_ledger_entries::action_id.eq_any(action_ids))
            .filter(financial_ledger_entries::currency.eq(clean_currency))
            .filter(financial_ledger_entries::effective_at.ge(start))
            .filter(financial_ledger_entries::effective_at.lt(end))
            .select(FinancialLedgerEntryRecord::as_select())
            .load::<FinancialLedgerEntryRecord>(&mut conn)
            .await
            .map_err(|e| StorageError::Internal(format!("financial spend ledger: {e}")))?;

        rows.into_iter().try_fold(0_i64, |total, row| {
            let kind = ledger_kind_from_text(&row.entry_kind)?;
            Ok(total + kind.signed_amount(row.amount_minor))
        })
    }

    pub async fn try_reserve_agentic_payment_budget(
        &self,
        request: ReserveAgenticPaymentBudgetRequest,
    ) -> Result<AgenticPaymentReservation, StorageError> {
        let ReserveAgenticPaymentBudgetRequest {
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
            return Err(StorageError::Internal(
                "agentic payment amount must be positive".into(),
            ));
        }
        if session_limit_minor < amount.amount_minor {
            return Err(StorageError::Internal(
                "agentic payment session limit must cover the reservation amount".into(),
            ));
        }
        if expires_at <= Utc::now() {
            return Err(StorageError::Internal(
                "agentic payment reservation expires_at must be in the future".into(),
            ));
        }
        let clean_session_id = clean_required("session_id", &session_id)?;
        let clean_principal_id = clean_required("principal_id", &principal_id)?;
        let clean_hash = clean_required("payment_requirement_hash", &payment_requirement_hash)?;
        let clean_currency = clean_required("currency", &amount.currency)?.to_uppercase();
        let action_uuid = parse_uuid(&action_id)?;
        let now = Utc::now();
        let mut conn = self.connection().await?;

        conn.transaction::<_, StorageError, _>(async |conn| {
            let action = financial_actions::table
                .filter(financial_actions::workspace_id.eq(&workspace_id))
                .filter(financial_actions::id.eq(action_uuid))
                .select(FinancialActionRecord::as_select())
                .for_update()
                .first::<FinancialActionRecord>(conn)
                .await
                .optional()
                .map_err(|e| StorageError::Internal(format!("agentic payment action get: {e}")))?
                .ok_or(StorageError::NotFound)?;
            if action.principal_id != clean_principal_id
                || action.currency != clean_currency
                || action.amount_minor != amount.amount_minor
            {
                return Err(StorageError::Conflict);
            }

            let existing = financial_payment_reservations::table
                .filter(financial_payment_reservations::workspace_id.eq(&workspace_id))
                .filter(financial_payment_reservations::session_id.eq(&clean_session_id))
                .filter(financial_payment_reservations::payment_requirement_hash.eq(&clean_hash))
                .select(FinancialPaymentReservationRecord::as_select())
                .for_update()
                .first::<FinancialPaymentReservationRecord>(conn)
                .await
                .optional()
                .map_err(|e| {
                    StorageError::Internal(format!("agentic payment reservation get: {e}"))
                })?;
            if let Some(existing) = existing {
                if existing.action_id != action_uuid
                    || existing.principal_id != clean_principal_id
                    || existing.amount_minor != amount.amount_minor
                    || existing.currency != clean_currency
                {
                    return Err(StorageError::Conflict);
                }
                return reservation_from_record(existing);
            }

            let duplicate_action = financial_payment_reservations::table
                .filter(financial_payment_reservations::workspace_id.eq(&workspace_id))
                .filter(financial_payment_reservations::action_id.eq(action_uuid))
                .select(FinancialPaymentReservationRecord::as_select())
                .for_update()
                .first::<FinancialPaymentReservationRecord>(conn)
                .await
                .optional()
                .map_err(|e| {
                    StorageError::Internal(format!("agentic payment reservation action get: {e}"))
                })?;
            if duplicate_action.is_some() {
                return Err(StorageError::Conflict);
            }

            let session = NewFinancialPaymentSession {
                workspace_id: workspace_id.clone(),
                id: clean_session_id.clone(),
                principal_id: clean_principal_id.clone(),
                currency: clean_currency.clone(),
                max_amount_minor: session_limit_minor,
                expires_at,
                metadata: metadata.clone(),
            };
            diesel::insert_into(financial_payment_sessions::table)
                .values(&session)
                .on_conflict((
                    financial_payment_sessions::workspace_id,
                    financial_payment_sessions::id,
                ))
                .do_nothing()
                .execute(conn)
                .await
                .map_err(|e| {
                    StorageError::Internal(format!("agentic payment session insert: {e}"))
                })?;

            let session = financial_payment_sessions::table
                .filter(financial_payment_sessions::workspace_id.eq(&workspace_id))
                .filter(financial_payment_sessions::id.eq(&clean_session_id))
                .select(FinancialPaymentSessionRecord::as_select())
                .for_update()
                .first::<FinancialPaymentSessionRecord>(conn)
                .await
                .map_err(|e| {
                    StorageError::Internal(format!("agentic payment session lock: {e}"))
                })?;
            if session.status != "active"
                || session.expires_at <= now
                || session.principal_id != clean_principal_id
                || session.currency != clean_currency
            {
                return Err(StorageError::Conflict);
            }
            let next_reserved = session
                .reserved_minor
                .checked_add(amount.amount_minor)
                .ok_or_else(|| {
                    StorageError::Internal("agentic payment reserved amount overflow".into())
                })?;
            let projected_total = next_reserved
                .checked_add(session.committed_minor)
                .ok_or_else(|| {
                    StorageError::Internal("agentic payment session amount overflow".into())
                })?;
            if projected_total > session.max_amount_minor {
                return Err(StorageError::Conflict);
            }

            diesel::update(
                financial_payment_sessions::table
                    .filter(financial_payment_sessions::workspace_id.eq(&workspace_id))
                    .filter(financial_payment_sessions::id.eq(&clean_session_id)),
            )
            .set((
                financial_payment_sessions::reserved_minor.eq(next_reserved),
                financial_payment_sessions::updated_at.eq(now),
            ))
            .execute(conn)
            .await
            .map_err(|e| StorageError::Internal(format!("agentic payment session reserve: {e}")))?;

            let reservation = NewFinancialPaymentReservation {
                workspace_id,
                id: Uuid::now_v7(),
                action_id: action_uuid,
                session_id: clean_session_id,
                principal_id: clean_principal_id,
                payment_requirement_hash: clean_hash,
                amount_minor: amount.amount_minor,
                currency: clean_currency,
                expires_at,
                metadata,
            };
            let row = diesel::insert_into(financial_payment_reservations::table)
                .values(&reservation)
                .returning(FinancialPaymentReservationRecord::as_returning())
                .get_result::<FinancialPaymentReservationRecord>(conn)
                .await
                .map_err(|e| {
                    StorageError::Internal(format!("agentic payment reservation insert: {e}"))
                })?;
            reservation_from_record(row)
        })
        .await
    }

    pub async fn get_agentic_payment_reservation(
        &self,
        workspace_id: &str,
        action_id: &str,
    ) -> Result<AgenticPaymentReservation, StorageError> {
        let action_uuid = parse_uuid(action_id)?;
        let mut conn = self.connection().await?;
        let row = financial_payment_reservations::table
            .filter(financial_payment_reservations::workspace_id.eq(workspace_id))
            .filter(financial_payment_reservations::action_id.eq(action_uuid))
            .select(FinancialPaymentReservationRecord::as_select())
            .first::<FinancialPaymentReservationRecord>(&mut conn)
            .await
            .optional()
            .map_err(|e| StorageError::Internal(format!("agentic payment reservation get: {e}")))?
            .ok_or(StorageError::NotFound)?;
        reservation_from_record(row)
    }

    pub async fn commit_agentic_payment_reservation(
        &self,
        workspace_id: &str,
        action_id: &str,
        proof: serde_json::Value,
    ) -> Result<AgenticPaymentReservation, StorageError> {
        let action_uuid = parse_uuid(action_id)?;
        let now = Utc::now();
        let mut conn = self.connection().await?;
        conn.transaction::<_, StorageError, _>(async |conn| {
            let reservation = financial_payment_reservations::table
                .filter(financial_payment_reservations::workspace_id.eq(workspace_id))
                .filter(financial_payment_reservations::action_id.eq(action_uuid))
                .select(FinancialPaymentReservationRecord::as_select())
                .for_update()
                .first::<FinancialPaymentReservationRecord>(conn)
                .await
                .optional()
                .map_err(|e| {
                    StorageError::Internal(format!("agentic payment reservation lock: {e}"))
                })?
                .ok_or(StorageError::NotFound)?;
            let status = enum_from_text::<AgenticPaymentReservationStatus>(&reservation.status)?;
            match status {
                AgenticPaymentReservationStatus::Committed => {
                    return reservation_from_record(reservation);
                }
                AgenticPaymentReservationStatus::Reserved => {}
                AgenticPaymentReservationStatus::Released
                | AgenticPaymentReservationStatus::Expired => return Err(StorageError::Conflict),
            }
            if reservation.expires_at <= now {
                return Err(StorageError::Conflict);
            }

            let session = financial_payment_sessions::table
                .filter(financial_payment_sessions::workspace_id.eq(workspace_id))
                .filter(financial_payment_sessions::id.eq(&reservation.session_id))
                .select(FinancialPaymentSessionRecord::as_select())
                .for_update()
                .first::<FinancialPaymentSessionRecord>(conn)
                .await
                .map_err(|e| {
                    StorageError::Internal(format!("agentic payment session lock: {e}"))
                })?;
            if session.reserved_minor < reservation.amount_minor {
                return Err(StorageError::Conflict);
            }
            let next_reserved = session.reserved_minor - reservation.amount_minor;
            let next_committed = session
                .committed_minor
                .checked_add(reservation.amount_minor)
                .ok_or_else(|| {
                    StorageError::Internal("agentic payment committed amount overflow".into())
                })?;

            diesel::update(
                financial_payment_sessions::table
                    .filter(financial_payment_sessions::workspace_id.eq(workspace_id))
                    .filter(financial_payment_sessions::id.eq(&reservation.session_id)),
            )
            .set((
                financial_payment_sessions::reserved_minor.eq(next_reserved),
                financial_payment_sessions::committed_minor.eq(next_committed),
                financial_payment_sessions::updated_at.eq(now),
            ))
            .execute(conn)
            .await
            .map_err(|e| StorageError::Internal(format!("agentic payment session commit: {e}")))?;

            let row = diesel::update(
                financial_payment_reservations::table
                    .filter(financial_payment_reservations::workspace_id.eq(workspace_id))
                    .filter(financial_payment_reservations::action_id.eq(action_uuid)),
            )
            .set((
                financial_payment_reservations::status
                    .eq(enum_text(AgenticPaymentReservationStatus::Committed)?),
                financial_payment_reservations::commit_proof.eq(Some(proof)),
                financial_payment_reservations::committed_at.eq(Some(now)),
                financial_payment_reservations::updated_at.eq(now),
            ))
            .returning(FinancialPaymentReservationRecord::as_returning())
            .get_result::<FinancialPaymentReservationRecord>(conn)
            .await
            .map_err(|e| {
                StorageError::Internal(format!("agentic payment reservation commit: {e}"))
            })?;
            reservation_from_record(row)
        })
        .await
    }

    pub async fn release_agentic_payment_reservation(
        &self,
        workspace_id: &str,
        action_id: &str,
        reason: &str,
        metadata: serde_json::Value,
    ) -> Result<AgenticPaymentReservation, StorageError> {
        let action_uuid = parse_uuid(action_id)?;
        let clean_reason = clean_required("reason", reason)?;
        let now = Utc::now();
        let mut conn = self.connection().await?;
        conn.transaction::<_, StorageError, _>(async |conn| {
            let reservation = financial_payment_reservations::table
                .filter(financial_payment_reservations::workspace_id.eq(workspace_id))
                .filter(financial_payment_reservations::action_id.eq(action_uuid))
                .select(FinancialPaymentReservationRecord::as_select())
                .for_update()
                .first::<FinancialPaymentReservationRecord>(conn)
                .await
                .optional()
                .map_err(|e| {
                    StorageError::Internal(format!("agentic payment reservation lock: {e}"))
                })?
                .ok_or(StorageError::NotFound)?;
            let status = enum_from_text::<AgenticPaymentReservationStatus>(&reservation.status)?;
            match status {
                AgenticPaymentReservationStatus::Released => {
                    return reservation_from_record(reservation);
                }
                AgenticPaymentReservationStatus::Reserved => {}
                AgenticPaymentReservationStatus::Committed
                | AgenticPaymentReservationStatus::Expired => return Err(StorageError::Conflict),
            }

            let session = financial_payment_sessions::table
                .filter(financial_payment_sessions::workspace_id.eq(workspace_id))
                .filter(financial_payment_sessions::id.eq(&reservation.session_id))
                .select(FinancialPaymentSessionRecord::as_select())
                .for_update()
                .first::<FinancialPaymentSessionRecord>(conn)
                .await
                .map_err(|e| {
                    StorageError::Internal(format!("agentic payment session lock: {e}"))
                })?;
            if session.reserved_minor < reservation.amount_minor {
                return Err(StorageError::Conflict);
            }
            let next_reserved = session.reserved_minor - reservation.amount_minor;
            let next_released = session
                .released_minor
                .checked_add(reservation.amount_minor)
                .ok_or_else(|| {
                    StorageError::Internal("agentic payment released amount overflow".into())
                })?;

            diesel::update(
                financial_payment_sessions::table
                    .filter(financial_payment_sessions::workspace_id.eq(workspace_id))
                    .filter(financial_payment_sessions::id.eq(&reservation.session_id)),
            )
            .set((
                financial_payment_sessions::reserved_minor.eq(next_reserved),
                financial_payment_sessions::released_minor.eq(next_released),
                financial_payment_sessions::updated_at.eq(now),
            ))
            .execute(conn)
            .await
            .map_err(|e| StorageError::Internal(format!("agentic payment session release: {e}")))?;

            let next_metadata =
                reservation_release_metadata(reservation.metadata, &clean_reason, metadata);
            let row = diesel::update(
                financial_payment_reservations::table
                    .filter(financial_payment_reservations::workspace_id.eq(workspace_id))
                    .filter(financial_payment_reservations::action_id.eq(action_uuid)),
            )
            .set((
                financial_payment_reservations::status
                    .eq(enum_text(AgenticPaymentReservationStatus::Released)?),
                financial_payment_reservations::metadata.eq(next_metadata),
                financial_payment_reservations::released_at.eq(Some(now)),
                financial_payment_reservations::updated_at.eq(now),
            ))
            .returning(FinancialPaymentReservationRecord::as_returning())
            .get_result::<FinancialPaymentReservationRecord>(conn)
            .await
            .map_err(|e| {
                StorageError::Internal(format!("agentic payment reservation release: {e}"))
            })?;
            reservation_from_record(row)
        })
        .await
    }

    pub async fn ledger_entry_exists(
        &self,
        workspace_id: &str,
        idempotency_key: &str,
    ) -> Result<bool, StorageError> {
        let clean_idempotency_key = clean_required("idempotency_key", idempotency_key)?;
        let mut conn = self.connection().await?;
        let exists = financial_ledger_entries::table
            .filter(financial_ledger_entries::workspace_id.eq(workspace_id))
            .filter(financial_ledger_entries::idempotency_key.eq(clean_idempotency_key))
            .select(financial_ledger_entries::id)
            .first::<Uuid>(&mut conn)
            .await
            .optional()
            .map_err(|e| StorageError::Internal(format!("financial ledger exists: {e}")))?
            .is_some();
        Ok(exists)
    }

    async fn get_action_by_idempotency_key(
        &self,
        workspace_id: &str,
        idempotency_key: &str,
    ) -> Result<StoredFinancialAction, StorageError> {
        let mut conn = self.connection().await?;
        let record = financial_actions::table
            .filter(financial_actions::workspace_id.eq(workspace_id))
            .filter(financial_actions::idempotency_key.eq(idempotency_key))
            .select(FinancialActionRecord::as_select())
            .first::<FinancialActionRecord>(&mut conn)
            .await
            .optional()
            .map_err(|e| StorageError::Internal(format!("financial action get idempotency: {e}")))?
            .ok_or(StorageError::NotFound)?;
        action_from_record(record)
    }

    async fn get_ledger_entry_by_idempotency_key(
        &self,
        workspace_id: &str,
        idempotency_key: &str,
    ) -> Result<StoredFinancialLedgerEntry, StorageError> {
        let mut conn = self.connection().await?;
        let record = financial_ledger_entries::table
            .filter(financial_ledger_entries::workspace_id.eq(workspace_id))
            .filter(financial_ledger_entries::idempotency_key.eq(idempotency_key))
            .select(FinancialLedgerEntryRecord::as_select())
            .first::<FinancialLedgerEntryRecord>(&mut conn)
            .await
            .optional()
            .map_err(|e| StorageError::Internal(format!("financial ledger get: {e}")))?
            .ok_or(StorageError::NotFound)?;
        ledger_entry_from_record(record)
    }

    async fn insert_event(
        &self,
        conn: &mut DbConnection<'_>,
        workspace_id: &str,
        action_id: Uuid,
        event_type: &str,
        from_status: Option<FinancialActionStatus>,
        to_status: Option<FinancialActionStatus>,
        metadata: serde_json::Value,
    ) -> Result<(), StorageError> {
        let event = NewFinancialActionEvent {
            workspace_id: workspace_id.to_string(),
            id: Uuid::now_v7(),
            action_id,
            event_type: event_type.to_string(),
            from_status: from_status.map(enum_text).transpose()?,
            to_status: to_status.map(enum_text).transpose()?,
            actor_id: None,
            reason: metadata
                .get("reason")
                .and_then(serde_json::Value::as_str)
                .map(str::to_string),
            metadata,
        };
        diesel::insert_into(financial_action_events::table)
            .values(&event)
            .execute(conn)
            .await
            .map_err(|e| StorageError::Internal(format!("financial action event insert: {e}")))?;
        Ok(())
    }

    async fn connection(&self) -> Result<DbConnection<'_>, StorageError> {
        self.pool
            .get()
            .await
            .map_err(|e| StorageError::Internal(format!("db pool: {e}")))
    }
}

impl std::fmt::Debug for FinancialRepo {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("FinancialRepo").finish_non_exhaustive()
    }
}

fn validate_create_action(input: &CreateFinancialActionRequest) -> Result<(), StorageError> {
    clean_required("idempotency_key", &input.idempotency_key)?;
    clean_operation(&input.action.operation)?;
    clean_required("principal_id", &input.action.principal_id)?;
    clean_required("currency", &input.action.amount.currency)?;
    if input.action.amount.amount_minor <= 0 {
        return Err(StorageError::Internal(
            "financial action amount must be positive".into(),
        ));
    }
    Ok(())
}

fn is_valid_transition(from: FinancialActionStatus, to: FinancialActionStatus) -> bool {
    use FinancialActionStatus::*;
    matches!(
        (from, to),
        (Proposed, Authorized | Held | Denied | Failed | Expired)
            | (Held, Authorized | Executed | Denied | Failed | Expired)
            | (Authorized, Executed | Denied | Failed | Expired)
            | (Executed, Reversed)
    )
}

fn clean_operation(operation: &str) -> Result<(), StorageError> {
    let trimmed = operation.trim();
    if trimmed.is_empty() {
        return Err(StorageError::Internal(
            "financial action operation must not be empty".into(),
        ));
    }
    if trimmed.len() > 128
        || !trimmed
            .chars()
            .all(|ch| ch.is_ascii_lowercase() || ch.is_ascii_digit() || ch == '_' || ch == '-')
    {
        return Err(StorageError::Internal(
            "financial action operation must be lowercase ASCII, digits, '_' or '-'".into(),
        ));
    }
    Ok(())
}

fn action_from_record(
    record: FinancialActionRecord,
) -> Result<StoredFinancialAction, StorageError> {
    let status = status_from_text(&record.status)?;
    let kind = enum_from_text::<FinancialActionKind>(&record.action_kind)?;
    let rail = enum_from_text::<FinancialRail>(&record.rail)?;
    let counterparty = optional_from_value(record.counterparty)?;
    let mandate = optional_from_value(record.mandate)?;
    let evidence = from_value::<Vec<EvidenceRef>>(record.evidence)?;
    Ok(StoredFinancialAction {
        workspace_id: record.workspace_id,
        id: record.id.to_string(),
        environment_id: record.environment_id,
        idempotency_key: record.idempotency_key,
        status,
        status_reason: None,
        action: FinancialAction {
            id: Some(record.id.to_string()),
            kind,
            operation: record.operation,
            principal_id: record.principal_id,
            amount: MoneyAmount {
                amount_minor: record.amount_minor,
                currency: record.currency,
            },
            counterparty,
            rail,
            mandate,
            memo: record.memo,
            metadata: record.metadata,
        },
        evidence,
        created_at: record.created_at,
        updated_at: record.updated_at,
    })
}

fn evaluation_from_record(
    record: FinancialActionEvaluationRecord,
) -> Result<FinancialActionEvaluation, StorageError> {
    Ok(FinancialActionEvaluation {
        action_id: record.action_id.to_string(),
        environment_id: record.environment_id,
        runtime_mode: enum_from_text::<FinancialRuntimeMode>(&record.runtime_mode)?,
        outcome: enum_from_text::<FinancialEvaluationOutcome>(&record.outcome)?,
        reason: record.reason,
        risks: from_value(record.risks)?,
        policy_ids: from_value(record.policy_ids)?,
        amount: MoneyAmount {
            amount_minor: record.amount_minor,
            currency: record.currency,
        },
        created_at: record.created_at.to_rfc3339(),
    })
}

fn execution_grant_from_record(
    record: FinancialExecutionGrantRecord,
) -> Result<FinancialExecutionGrant, StorageError> {
    Ok(FinancialExecutionGrant {
        id: record.id.to_string(),
        action_id: record.action_id.to_string(),
        action_hash: record.action_hash,
        binding: enum_from_text::<FinancialExecutionBinding>(&record.binding)?,
        status: enum_from_text::<FinancialExecutionGrantStatus>(&record.status)?,
        expires_at: record.expires_at.to_rfc3339(),
        created_at: record.created_at.to_rfc3339(),
    })
}

fn connector_from_record(
    record: FinancialExecutionConnectorRecord,
) -> Result<StoredFinancialExecutionConnector, StorageError> {
    Ok(StoredFinancialExecutionConnector {
        connector: FinancialExecutionConnector {
            id: record.id.to_string(),
            workspace_id: record.workspace_id,
            display_name: record.display_name,
            status: enum_from_text::<FinancialExecutionConnectorStatus>(&record.status)?,
            allowed_rails: from_value(record.allowed_rails)?,
            allowed_operations: from_value(record.allowed_operations)?,
            created_at: record.created_at.to_rfc3339(),
            revoked_at: record.revoked_at.map(|value| value.to_rfc3339()),
        },
        encrypted_secret: record.encrypted_secret,
    })
}

fn observation_review_from_record(
    record: FinancialObservationReviewRecord,
) -> Result<FinancialObservationReview, StorageError> {
    Ok(FinancialObservationReview {
        id: record.id.to_string(),
        workspace_id: record.workspace_id,
        action_id: record.action_id.to_string(),
        outcome: enum_from_text::<FinancialObservationReviewOutcome>(&record.outcome)?,
        note: record.note,
        reviewed_by: record.reviewed_by,
        created_at: record.created_at.to_rfc3339(),
    })
}

fn event_from_record(
    record: FinancialActionEventRecord,
) -> Result<StoredFinancialActionEvent, StorageError> {
    Ok(StoredFinancialActionEvent {
        workspace_id: record.workspace_id,
        id: record.id.to_string(),
        action_id: record.action_id.to_string(),
        event_type: record.event_type,
        from_status: record
            .from_status
            .as_deref()
            .map(status_from_text)
            .transpose()?,
        to_status: record
            .to_status
            .as_deref()
            .map(status_from_text)
            .transpose()?,
        actor_id: record.actor_id,
        reason: record.reason,
        metadata: record.metadata,
        created_at: record.created_at,
    })
}

fn approval_from_record(
    record: ApprovalRequestRecord,
) -> Result<StoredFinancialApprovalRequest, StorageError> {
    Ok(StoredFinancialApprovalRequest {
        workspace_id: record.workspace_id,
        id: record.id.to_string(),
        action_id: record.action_id.to_string(),
        status: enum_from_text(&record.status)?,
        reason: record.reason,
        approver_roles: from_value::<Vec<String>>(record.approver_roles)?,
        decided_by: record.decided_by,
        decided_at: record.decided_at,
        expires_at: record.expires_at,
        metadata: record.metadata,
        created_at: record.created_at,
        updated_at: record.updated_at,
    })
}

fn mandate_from_record(record: MandateRecord) -> Result<StoredFinancialMandate, StorageError> {
    Ok(StoredFinancialMandate {
        workspace_id: record.workspace_id,
        id: record.id,
        version: record.version,
        status: enum_from_text(&record.status)?,
        principal_id: record.principal_id,
        scope: record.scope,
        metadata: record.metadata,
        starts_at: record.starts_at,
        expires_at: record.expires_at,
        created_at: record.created_at,
        updated_at: record.updated_at,
    })
}

fn receipt_from_record(
    record: FinancialReceiptRecord,
) -> Result<StoredFinancialReceipt, StorageError> {
    Ok(StoredFinancialReceipt {
        workspace_id: record.workspace_id,
        id: record.id.to_string(),
        action_id: record.action_id.to_string(),
        trace_id: record.trace_id.map(|value| value.to_string()),
        ledger_event_ids: from_value::<Vec<String>>(record.ledger_event_ids)?,
        proof: record.proof,
        created_at: record.created_at,
    })
}

fn ledger_entry_from_record(
    record: FinancialLedgerEntryRecord,
) -> Result<StoredFinancialLedgerEntry, StorageError> {
    Ok(StoredFinancialLedgerEntry {
        workspace_id: record.workspace_id,
        id: record.id.to_string(),
        action_id: record.action_id.to_string(),
        kind: ledger_kind_from_text(&record.entry_kind)?,
        amount_minor: record.amount_minor,
        currency: record.currency,
        idempotency_key: record.idempotency_key,
        metadata: record.metadata,
        effective_at: record.effective_at,
        created_at: record.created_at,
    })
}

fn reservation_from_record(
    record: FinancialPaymentReservationRecord,
) -> Result<AgenticPaymentReservation, StorageError> {
    Ok(AgenticPaymentReservation {
        id: record.id.to_string(),
        session_id: record.session_id,
        action_id: record.action_id.to_string(),
        principal_id: record.principal_id,
        payment_requirement_hash: record.payment_requirement_hash,
        amount: MoneyAmount {
            amount_minor: record.amount_minor,
            currency: record.currency,
        },
        status: enum_from_text::<AgenticPaymentReservationStatus>(&record.status)?,
        expires_at: record.expires_at.to_rfc3339(),
        committed_at: record.committed_at.map(|value| value.to_rfc3339()),
        released_at: record.released_at.map(|value| value.to_rfc3339()),
        metadata: record.metadata,
    })
}

fn outcome_from_record(
    record: FinancialActionOutcomeRecord,
) -> Result<StoredFinancialActionOutcome, StorageError> {
    let final_loss_amount = match (
        record.final_loss_amount_minor,
        record.final_loss_currency.clone(),
    ) {
        (Some(amount_minor), Some(currency)) => Some(MoneyAmount {
            amount_minor,
            currency,
        }),
        (None, None) => None,
        _ => {
            return Err(StorageError::Internal(
                "financial outcome final loss amount must include amount and currency".into(),
            ))
        }
    };
    Ok(StoredFinancialActionOutcome {
        workspace_id: record.workspace_id,
        id: record.id.to_string(),
        outcome: FinancialActionOutcome {
            action_id: record.action_id.to_string(),
            status: enum_from_text::<FinancialActionOutcomeStatus>(&record.status)?,
            reversal_capability: enum_from_text::<ReversalCapability>(&record.reversal_capability)?,
            recovery_status: enum_from_text::<RecoveryStatus>(&record.recovery_status)?,
            provider_status: record.provider_status,
            provider_reference: record.provider_reference,
            final_loss_amount,
            occurred_at: record.occurred_at.to_rfc3339(),
            metadata: record.metadata,
        },
        created_at: record.created_at,
    })
}

impl From<StoredFinancialMandate> for FinancialMandate {
    fn from(row: StoredFinancialMandate) -> Self {
        Self {
            id: row.id,
            workspace_id: row.workspace_id,
            version: row.version,
            status: row.status,
            principal_id: row.principal_id,
            scope: row.scope,
            metadata: row.metadata,
            starts_at: row.starts_at.map(|value| value.to_rfc3339()),
            expires_at: row.expires_at.map(|value| value.to_rfc3339()),
            created_at: row.created_at.to_rfc3339(),
            updated_at: row.updated_at.to_rfc3339(),
        }
    }
}

impl From<StoredFinancialReceipt> for FinancialReceipt {
    fn from(row: StoredFinancialReceipt) -> Self {
        Self {
            id: row.id,
            action_id: row.action_id,
            trace_id: row.trace_id,
            ledger_event_ids: row.ledger_event_ids,
            proof: row.proof,
            created_at: row.created_at.to_rfc3339(),
        }
    }
}

impl From<StoredFinancialActionOutcome> for FinancialActionOutcome {
    fn from(row: StoredFinancialActionOutcome) -> Self {
        row.outcome
    }
}

fn empty_observation_currency(currency: &str) -> FinancialObservationCurrencySummary {
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

fn observation_rate_bps(numerator: i64, denominator: i64) -> i32 {
    if denominator <= 0 {
        return 0;
    }
    numerator
        .saturating_mul(10_000)
        .checked_div(denominator)
        .unwrap_or(0)
        .clamp(0, 10_000) as i32
}

fn parse_uuid(value: &str) -> Result<Uuid, StorageError> {
    Uuid::parse_str(value)
        .map_err(|e| StorageError::Internal(format!("invalid financial action uuid: {e}")))
}

fn parse_rfc3339(name: &str, value: &str) -> Result<DateTime<Utc>, StorageError> {
    DateTime::parse_from_rfc3339(value)
        .map(|value| value.with_timezone(&Utc))
        .map_err(|e| StorageError::Internal(format!("{name}: {e}")))
}

fn parse_optional_rfc3339(
    name: &str,
    value: Option<&str>,
) -> Result<Option<DateTime<Utc>>, StorageError> {
    value
        .map(|value| {
            DateTime::parse_from_rfc3339(value)
                .map(|value| value.with_timezone(&Utc))
                .map_err(|e| StorageError::Internal(format!("{name}: {e}")))
        })
        .transpose()
}

fn clean_required(name: &str, value: &str) -> Result<String, StorageError> {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        return Err(StorageError::Internal(format!("{name} must not be empty")));
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

fn enum_text<T>(value: T) -> Result<String, StorageError>
where
    T: Serialize,
{
    match serde_json::to_value(value)
        .map_err(|e| StorageError::Internal(format!("financial enum encode: {e}")))?
    {
        serde_json::Value::String(value) => Ok(value),
        _ => Err(StorageError::Internal(
            "financial enum encoded to non-string".into(),
        )),
    }
}

fn status_from_text(value: &str) -> Result<FinancialActionStatus, StorageError> {
    enum_from_text(value)
}

fn ledger_kind_from_text(value: &str) -> Result<FinancialLedgerEntryKind, StorageError> {
    match value {
        "reserved" => Ok(FinancialLedgerEntryKind::Reserved),
        "released" => Ok(FinancialLedgerEntryKind::Released),
        "executed" => Ok(FinancialLedgerEntryKind::Executed),
        "reversed" => Ok(FinancialLedgerEntryKind::Reversed),
        other => Err(StorageError::Internal(format!(
            "unknown financial ledger kind: {other}"
        ))),
    }
}

fn enum_from_text<T>(value: &str) -> Result<T, StorageError>
where
    T: DeserializeOwned,
{
    from_value(serde_json::Value::String(value.to_string()))
}

fn optional_json<T>(value: &Option<T>) -> Result<Option<serde_json::Value>, StorageError>
where
    T: Serialize,
{
    value
        .as_ref()
        .map(|value| {
            serde_json::to_value(value)
                .map_err(|e| StorageError::Internal(format!("financial json encode: {e}")))
        })
        .transpose()
}

fn optional_from_value<T>(value: Option<serde_json::Value>) -> Result<Option<T>, StorageError>
where
    T: DeserializeOwned,
{
    value.map(from_value).transpose()
}

fn from_value<T>(value: serde_json::Value) -> Result<T, StorageError>
where
    T: DeserializeOwned,
{
    serde_json::from_value(value)
        .map_err(|e| StorageError::Internal(format!("financial json decode: {e}")))
}

fn reservation_release_metadata(
    current: serde_json::Value,
    reason: &str,
    release_metadata: serde_json::Value,
) -> serde_json::Value {
    let mut current = match current {
        serde_json::Value::Object(map) => map,
        _ => serde_json::Map::new(),
    };
    current.insert(
        "release_reason".into(),
        serde_json::Value::String(reason.to_string()),
    );
    current.insert("release_metadata".into(), release_metadata);
    serde_json::Value::Object(current)
}
