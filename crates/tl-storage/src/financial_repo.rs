use chrono::{DateTime, Utc};
use diesel::prelude::*;
use diesel_async::{AsyncConnection, RunQueryDsl};
use serde::de::DeserializeOwned;
use serde::Serialize;
use tl_core::{
    AgenticPaymentReservation, AgenticPaymentReservationStatus, AuthorizationEffect,
    AuthorizationIntentStatus, CreateFinancialActionRequest, EvidenceRef, FinancialAction,
    FinancialActionKind, FinancialActionOutcome, FinancialActionOutcomeStatus,
    FinancialExecutionStatus, FinancialRail, FinancialReceipt, MoneyAmount, RecoveryStatus,
    ReversalCapability,
};
use uuid::Uuid;

use crate::models::{
    FinancialActionEventRecord, FinancialActionOutcomeRecord, FinancialActionRecord,
    FinancialLedgerEntryRecord, FinancialPaymentReservationRecord, FinancialPaymentSessionRecord,
    FinancialReceiptRecord, NewFinancialAction, NewFinancialActionEvent, NewFinancialActionOutcome,
    NewFinancialBudgetPrincipalLock, NewFinancialLedgerEntry, NewFinancialPaymentReservation,
    NewFinancialPaymentSession, NewFinancialReceipt,
};
use crate::postgres::{DbConnection, DbPool};
use crate::schema::{
    authorization_intents, authorization_receipts, financial_action_events,
    financial_action_outcomes, financial_actions, financial_budget_principal_locks,
    financial_ledger_entries, financial_payment_reservations, financial_payment_sessions,
    financial_receipts,
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
    pub environment_id: String,
    pub id: String,
    pub idempotency_key: String,
    pub authorization_intent_id: Option<String>,
    pub authorization_receipt_id: Option<String>,
    pub authorization_effect: AuthorizationEffect,
    pub authorization_status: AuthorizationIntentStatus,
    pub execution_status: FinancialExecutionStatus,
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
    pub from_status: Option<FinancialExecutionStatus>,
    pub to_status: Option<FinancialExecutionStatus>,
    pub actor_id: Option<String>,
    pub reason: Option<String>,
    pub metadata: serde_json::Value,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct StoredFinancialReceipt {
    pub workspace_id: String,
    pub environment_id: String,
    pub id: String,
    pub action_id: String,
    pub authorization_receipt_id: String,
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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FinancialBudgetWindow {
    Day,
    Week,
    Month,
}

#[derive(Debug, Clone)]
pub struct FinancialBudgetConstraint {
    pub policy_id: String,
    pub window: FinancialBudgetWindow,
    pub cap_minor: i64,
    pub block_on_breach: bool,
}

#[derive(Debug, Clone)]
pub struct ReserveFinancialActionBudgetRequest {
    pub workspace_id: String,
    pub action_id: String,
    pub principal_id: String,
    pub amount: MoneyAmount,
    pub idempotency_key: String,
    pub day_start: DateTime<Utc>,
    pub week_start: DateTime<Utc>,
    pub month_start: DateTime<Utc>,
    pub now: DateTime<Utc>,
    pub constraints: Vec<FinancialBudgetConstraint>,
    pub metadata: serde_json::Value,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FinancialBudgetViolation {
    pub policy_id: String,
    pub window: FinancialBudgetWindow,
    pub cap_minor: i64,
    pub committed_minor: i64,
    pub requested_minor: i64,
    pub block_on_breach: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ReserveFinancialActionBudgetResult {
    Reserved {
        ledger_entry_id: String,
        violations: Vec<FinancialBudgetViolation>,
    },
    Denied {
        violations: Vec<FinancialBudgetViolation>,
    },
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
        environment_id: &str,
        input: CreateFinancialActionRequest,
    ) -> Result<StoredFinancialAction, StorageError> {
        validate_create_action(&input)?;
        let environment_id = clean_required("environment_id", environment_id)?;
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
            environment_id: environment_id.clone(),
            id: action_id,
            idempotency_key: idempotency_key.clone(),
            principal_id: input.action.principal_id.trim().to_string(),
            action_kind: enum_text(input.action.kind)?,
            operation: input.action.operation.trim().to_string(),
            amount_minor: input.action.amount.amount_minor,
            currency: input.action.amount.currency.trim().to_uppercase(),
            counterparty: optional_json(&input.action.counterparty)?,
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
                financial_actions::environment_id,
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
                Some(FinancialExecutionStatus::NotStarted),
                serde_json::json!({}),
            )
            .await?;
        }

        drop(conn);
        self.get_action_by_idempotency_key(workspace_id, &environment_id, &idempotency_key)
            .await
    }

    pub async fn get_action(
        &self,
        workspace_id: &str,
        environment_id: &str,
        action_id: &str,
    ) -> Result<StoredFinancialAction, StorageError> {
        let action_uuid = parse_uuid(action_id)?;
        let mut conn = self.connection().await?;
        let record = financial_actions::table
            .filter(financial_actions::workspace_id.eq(workspace_id))
            .filter(financial_actions::environment_id.eq(environment_id))
            .filter(financial_actions::id.eq(action_uuid))
            .select(FinancialActionRecord::as_select())
            .first::<FinancialActionRecord>(&mut conn)
            .await
            .optional()
            .map_err(|e| StorageError::Internal(format!("financial action get: {e}")))?
            .ok_or(StorageError::NotFound)?;
        let mut action = self
            .hydrate_action(&mut conn, action_from_record(record)?)
            .await?;
        action.status_reason = self
            .latest_status_reason(&mut conn, workspace_id, action_id)
            .await?;
        Ok(action)
    }

    pub async fn list_actions(
        &self,
        workspace_id: &str,
        environment_id: Option<&str>,
    ) -> Result<Vec<StoredFinancialAction>, StorageError> {
        let mut conn = self.connection().await?;
        let mut query = financial_actions::table
            .filter(financial_actions::workspace_id.eq(workspace_id))
            .into_boxed();
        if let Some(environment_id) = environment_id {
            query = query.filter(financial_actions::environment_id.eq(environment_id));
        }
        let rows = query
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
            *action = self.hydrate_action(&mut conn, action.clone()).await?;
            action.status_reason = self
                .latest_status_reason(&mut conn, workspace_id, &action.id)
                .await?;
        }
        Ok(actions)
    }

    pub async fn create_receipt(
        &self,
        workspace_id: &str,
        action_id: &str,
        authorization_receipt_id: &str,
        trace_id: Option<&str>,
        ledger_event_ids: Vec<String>,
        proof: serde_json::Value,
    ) -> Result<StoredFinancialReceipt, StorageError> {
        let action_uuid = parse_uuid(action_id)?;
        let authorization_receipt_uuid = parse_uuid(authorization_receipt_id)?;
        let trace_uuid = trace_id.map(parse_uuid).transpose()?;
        let mut conn = self.connection().await?;
        let environment_id = financial_actions::table
            .filter(financial_actions::workspace_id.eq(workspace_id))
            .filter(financial_actions::id.eq(action_uuid))
            .select(financial_actions::environment_id)
            .first::<String>(&mut conn)
            .await
            .optional()
            .map_err(|e| StorageError::Internal(format!("financial receipt action lookup: {e}")))?
            .ok_or(StorageError::NotFound)?;

        let receipt = NewFinancialReceipt {
            workspace_id: workspace_id.to_string(),
            environment_id,
            id: action_uuid,
            action_id: action_uuid,
            authorization_receipt_id: Some(authorization_receipt_uuid),
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

    pub async fn update_authorization(
        &self,
        workspace_id: &str,
        environment_id: &str,
        action_id: &str,
        intent_id: Option<&str>,
    ) -> Result<StoredFinancialAction, StorageError> {
        let action_id = parse_uuid(action_id)?;
        let intent_id = intent_id.map(parse_uuid).transpose()?;
        let mut conn = self.connection().await?;
        let updated = diesel::update(
            financial_actions::table
                .filter(financial_actions::workspace_id.eq(workspace_id))
                .filter(financial_actions::environment_id.eq(environment_id))
                .filter(financial_actions::id.eq(action_id)),
        )
        .set((
            financial_actions::authorization_intent_id.eq(intent_id),
            financial_actions::updated_at.eq(Utc::now()),
        ))
        .execute(&mut conn)
        .await
        .map_err(|e| StorageError::Internal(format!("financial authorization update: {e}")))?;
        if updated == 0 {
            return Err(StorageError::NotFound);
        }
        drop(conn);
        self.get_action(workspace_id, environment_id, &action_id.to_string())
            .await
    }

    pub async fn transition_execution(
        &self,
        workspace_id: &str,
        environment_id: &str,
        action_id: &str,
        next_status: FinancialExecutionStatus,
        reason: Option<&str>,
    ) -> Result<StoredFinancialAction, StorageError> {
        let action_uuid = parse_uuid(action_id)?;
        let mut conn = self.connection().await?;
        let current = financial_actions::table
            .filter(financial_actions::workspace_id.eq(workspace_id))
            .filter(financial_actions::environment_id.eq(environment_id))
            .filter(financial_actions::id.eq(action_uuid))
            .select(financial_actions::execution_status)
            .first::<String>(&mut conn)
            .await
            .optional()
            .map_err(|e| StorageError::Internal(format!("financial execution get: {e}")))?
            .ok_or(StorageError::NotFound)?;
        let current_status = execution_status_from_text(&current)?;
        if current_status != next_status
            && !is_valid_execution_transition(current_status, next_status)
        {
            return Err(StorageError::Conflict);
        }
        let updated = diesel::update(
            financial_actions::table
                .filter(financial_actions::workspace_id.eq(workspace_id))
                .filter(financial_actions::environment_id.eq(environment_id))
                .filter(financial_actions::id.eq(action_uuid))
                .filter(financial_actions::execution_status.eq(&current)),
        )
        .set((
            financial_actions::execution_status.eq(enum_text(next_status)?),
            financial_actions::updated_at.eq(Utc::now()),
        ))
        .execute(&mut conn)
        .await
        .map_err(|e| StorageError::Internal(format!("financial execution update: {e}")))?;
        if updated == 0 {
            return Err(StorageError::Conflict);
        }
        self.insert_event(
            &mut conn,
            workspace_id,
            action_uuid,
            "execution_status_changed",
            Some(current_status),
            Some(next_status),
            serde_json::json!({ "reason": reason }),
        )
        .await?;
        drop(conn);
        self.get_action(workspace_id, environment_id, action_id)
            .await
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

    /// Atomically check every matching action-budget constraint and reserve
    /// the action amount before authorization. A durable principal/currency
    /// row serializes replicas around the ledger read and reservation insert.
    pub async fn reserve_action_budget(
        &self,
        request: ReserveFinancialActionBudgetRequest,
    ) -> Result<ReserveFinancialActionBudgetResult, StorageError> {
        if request.amount.amount_minor <= 0 {
            return Err(StorageError::Internal(
                "financial budget reservation amount must be positive".into(),
            ));
        }
        let action_uuid = parse_uuid(&request.action_id)?;
        let currency = clean_required("currency", &request.amount.currency)?.to_uppercase();
        let idempotency_key = clean_required("idempotency_key", &request.idempotency_key)?;
        let workspace_id = request.workspace_id.clone();
        let mut conn = self.connection().await?;
        conn.transaction::<_, StorageError, _>(async move |conn| {
            diesel::insert_into(financial_budget_principal_locks::table)
                .values(&NewFinancialBudgetPrincipalLock {
                    workspace_id: workspace_id.clone(),
                    principal_id: request.principal_id.clone(),
                    currency: currency.clone(),
                })
                .on_conflict((
                    financial_budget_principal_locks::workspace_id,
                    financial_budget_principal_locks::principal_id,
                    financial_budget_principal_locks::currency,
                ))
                .do_nothing()
                .execute(conn)
                .await
                .map_err(|error| {
                    StorageError::Internal(format!("financial budget lock insert: {error}"))
                })?;
            financial_budget_principal_locks::table
                .filter(financial_budget_principal_locks::workspace_id.eq(&workspace_id))
                .filter(financial_budget_principal_locks::principal_id.eq(&request.principal_id))
                .filter(financial_budget_principal_locks::currency.eq(&currency))
                .select((
                    financial_budget_principal_locks::workspace_id,
                    financial_budget_principal_locks::principal_id,
                    financial_budget_principal_locks::currency,
                ))
                .for_update()
                .first::<(String, String, String)>(conn)
                .await
                .map_err(|error| {
                    StorageError::Internal(format!("financial budget principal lock: {error}"))
                })?;

            if let Some(existing) = financial_ledger_entries::table
                .filter(financial_ledger_entries::workspace_id.eq(&workspace_id))
                .filter(financial_ledger_entries::idempotency_key.eq(&idempotency_key))
                .select(FinancialLedgerEntryRecord::as_select())
                .first::<FinancialLedgerEntryRecord>(conn)
                .await
                .optional()
                .map_err(|error| {
                    StorageError::Internal(format!("financial budget reservation get: {error}"))
                })?
            {
                if existing.action_id != action_uuid
                    || existing.entry_kind != FinancialLedgerEntryKind::Reserved.as_str()
                    || existing.amount_minor != request.amount.amount_minor
                    || existing.currency != currency
                {
                    return Err(StorageError::Conflict);
                }
                return Ok(ReserveFinancialActionBudgetResult::Reserved {
                    ledger_entry_id: existing.id.to_string(),
                    violations: vec![],
                });
            }

            let action = financial_actions::table
                .filter(financial_actions::workspace_id.eq(&workspace_id))
                .filter(financial_actions::id.eq(action_uuid))
                .select(FinancialActionRecord::as_select())
                .first::<FinancialActionRecord>(conn)
                .await
                .optional()
                .map_err(|error| {
                    StorageError::Internal(format!("financial budget action get: {error}"))
                })?
                .ok_or(StorageError::NotFound)?;
            if action.principal_id != request.principal_id
                || action.amount_minor != request.amount.amount_minor
                || action.currency != currency
            {
                return Err(StorageError::Conflict);
            }
            let action_ids = financial_action_ids_in_transaction(
                conn,
                &workspace_id,
                &request.principal_id,
                &currency,
            )
            .await?;

            let mut violations = Vec::new();
            for constraint in request.constraints {
                let start = match constraint.window {
                    FinancialBudgetWindow::Day => request.day_start,
                    FinancialBudgetWindow::Week => request.week_start,
                    FinancialBudgetWindow::Month => request.month_start,
                };
                let committed_minor = net_spend_minor_in_transaction(
                    conn,
                    &workspace_id,
                    &action_ids,
                    &currency,
                    start,
                    request.now,
                )
                .await?;
                if committed_minor.saturating_add(request.amount.amount_minor)
                    > constraint.cap_minor
                {
                    violations.push(FinancialBudgetViolation {
                        policy_id: constraint.policy_id,
                        window: constraint.window,
                        cap_minor: constraint.cap_minor,
                        committed_minor,
                        requested_minor: request.amount.amount_minor,
                        block_on_breach: constraint.block_on_breach,
                    });
                }
            }
            if violations.iter().any(|violation| violation.block_on_breach) {
                return Ok(ReserveFinancialActionBudgetResult::Denied { violations });
            }

            let entry = NewFinancialLedgerEntry {
                workspace_id: workspace_id.clone(),
                id: Uuid::now_v7(),
                action_id: action_uuid,
                entry_kind: FinancialLedgerEntryKind::Reserved.as_str().to_string(),
                amount_minor: request.amount.amount_minor,
                currency,
                idempotency_key,
                metadata: request.metadata,
            };
            let row = diesel::insert_into(financial_ledger_entries::table)
                .values(&entry)
                .returning(FinancialLedgerEntryRecord::as_returning())
                .get_result::<FinancialLedgerEntryRecord>(conn)
                .await
                .map_err(|error| {
                    StorageError::Internal(format!("financial budget reservation insert: {error}"))
                })?;
            Ok(ReserveFinancialActionBudgetResult::Reserved {
                ledger_entry_id: row.id.to_string(),
                violations,
            })
        })
        .await
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
        let action_ids = financial_action_ids_in_transaction(
            &mut conn,
            workspace_id,
            &clean_principal,
            &clean_currency,
        )
        .await?;
        net_spend_minor_in_transaction(
            &mut conn,
            workspace_id,
            &action_ids,
            &clean_currency,
            start,
            end,
        )
        .await
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
        environment_id: &str,
        idempotency_key: &str,
    ) -> Result<StoredFinancialAction, StorageError> {
        let mut conn = self.connection().await?;
        let record = financial_actions::table
            .filter(financial_actions::workspace_id.eq(workspace_id))
            .filter(financial_actions::environment_id.eq(environment_id))
            .filter(financial_actions::idempotency_key.eq(idempotency_key))
            .select(FinancialActionRecord::as_select())
            .first::<FinancialActionRecord>(&mut conn)
            .await
            .optional()
            .map_err(|e| StorageError::Internal(format!("financial action get idempotency: {e}")))?
            .ok_or(StorageError::NotFound)?;
        self.hydrate_action(&mut conn, action_from_record(record)?)
            .await
    }

    async fn hydrate_action(
        &self,
        conn: &mut DbConnection<'_>,
        mut action: StoredFinancialAction,
    ) -> Result<StoredFinancialAction, StorageError> {
        let Some(intent_id) = action
            .authorization_intent_id
            .as_deref()
            .map(parse_uuid)
            .transpose()?
        else {
            return Ok(action);
        };
        let projection = authorization_intents::table
            .filter(authorization_intents::workspace_id.eq(&action.workspace_id))
            .filter(authorization_intents::environment_id.eq(&action.environment_id))
            .filter(authorization_intents::id.eq(intent_id))
            .select((
                authorization_intents::current_effect,
                authorization_intents::status,
            ))
            .first::<(String, String)>(conn)
            .await
            .optional()
            .map_err(|e| StorageError::Internal(format!("financial authorization hydrate: {e}")))?;
        if let Some((effect, status)) = projection {
            action.authorization_effect = enum_from_text(&effect)?;
            action.authorization_status = enum_from_text(&status)?;
        }
        action.authorization_receipt_id = authorization_receipts::table
            .filter(authorization_receipts::workspace_id.eq(&action.workspace_id))
            .filter(authorization_receipts::environment_id.eq(&action.environment_id))
            .filter(authorization_receipts::intent_id.eq(Some(intent_id)))
            .order(authorization_receipts::created_at.desc())
            .select(authorization_receipts::id)
            .first::<Uuid>(conn)
            .await
            .optional()
            .map_err(|e| StorageError::Internal(format!("financial receipt hydrate: {e}")))?
            .map(|id| id.to_string());
        Ok(action)
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
        from_status: Option<FinancialExecutionStatus>,
        to_status: Option<FinancialExecutionStatus>,
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

fn is_valid_execution_transition(
    from: FinancialExecutionStatus,
    to: FinancialExecutionStatus,
) -> bool {
    use FinancialExecutionStatus::*;
    matches!(
        (from, to),
        (NotStarted, Executing | Canceled)
            | (Executing, Succeeded | Failed | Canceled)
            | (Failed, Executing | Canceled)
            | (Succeeded, Reversed)
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
    let kind = enum_from_text::<FinancialActionKind>(&record.action_kind)?;
    let rail = enum_from_text::<FinancialRail>(&record.rail)?;
    let counterparty = optional_from_value(record.counterparty)?;
    let evidence = from_value::<Vec<EvidenceRef>>(record.evidence)?;
    Ok(StoredFinancialAction {
        workspace_id: record.workspace_id,
        environment_id: record.environment_id,
        id: record.id.to_string(),
        idempotency_key: record.idempotency_key,
        authorization_intent_id: record.authorization_intent_id.map(|id| id.to_string()),
        authorization_receipt_id: None,
        authorization_effect: AuthorizationEffect::Defer,
        authorization_status: AuthorizationIntentStatus::Evaluating,
        execution_status: execution_status_from_text(&record.execution_status)?,
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
            memo: record.memo,
            metadata: record.metadata,
        },
        evidence,
        created_at: record.created_at,
        updated_at: record.updated_at,
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
            .map(execution_status_from_text)
            .transpose()?,
        to_status: record
            .to_status
            .as_deref()
            .map(execution_status_from_text)
            .transpose()?,
        actor_id: record.actor_id,
        reason: record.reason,
        metadata: record.metadata,
        created_at: record.created_at,
    })
}

fn receipt_from_record(
    record: FinancialReceiptRecord,
) -> Result<StoredFinancialReceipt, StorageError> {
    Ok(StoredFinancialReceipt {
        workspace_id: record.workspace_id,
        environment_id: record.environment_id,
        id: record.id.to_string(),
        action_id: record.action_id.to_string(),
        authorization_receipt_id: record
            .authorization_receipt_id
            .ok_or_else(|| {
                StorageError::Internal("financial receipt missing authorization receipt".into())
            })?
            .to_string(),
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

async fn financial_action_ids_in_transaction(
    conn: &mut diesel_async::AsyncPgConnection,
    workspace_id: &str,
    principal_id: &str,
    currency: &str,
) -> Result<Vec<Uuid>, StorageError> {
    financial_actions::table
        .filter(financial_actions::workspace_id.eq(workspace_id))
        .filter(financial_actions::principal_id.eq(principal_id))
        .filter(financial_actions::currency.eq(currency))
        .select(financial_actions::id)
        .load::<Uuid>(conn)
        .await
        .map_err(|error| StorageError::Internal(format!("financial spend actions: {error}")))
}

async fn net_spend_minor_in_transaction(
    conn: &mut diesel_async::AsyncPgConnection,
    workspace_id: &str,
    action_ids: &[Uuid],
    currency: &str,
    start: DateTime<Utc>,
    end: DateTime<Utc>,
) -> Result<i64, StorageError> {
    if action_ids.is_empty() {
        return Ok(0);
    }
    let rows = financial_ledger_entries::table
        .filter(financial_ledger_entries::workspace_id.eq(workspace_id))
        .filter(financial_ledger_entries::action_id.eq_any(action_ids))
        .filter(financial_ledger_entries::currency.eq(currency))
        .filter(financial_ledger_entries::effective_at.ge(start))
        .filter(financial_ledger_entries::effective_at.lt(end))
        .select(FinancialLedgerEntryRecord::as_select())
        .load::<FinancialLedgerEntryRecord>(conn)
        .await
        .map_err(|error| StorageError::Internal(format!("financial spend ledger: {error}")))?;

    rows.into_iter().try_fold(0_i64, |total, row| {
        let kind = ledger_kind_from_text(&row.entry_kind)?;
        Ok(total.saturating_add(kind.signed_amount(row.amount_minor)))
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

impl From<StoredFinancialReceipt> for FinancialReceipt {
    fn from(row: StoredFinancialReceipt) -> Self {
        Self {
            id: row.id,
            action_id: row.action_id,
            authorization_receipt_id: row.authorization_receipt_id,
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

fn parse_uuid(value: &str) -> Result<Uuid, StorageError> {
    Uuid::parse_str(value)
        .map_err(|e| StorageError::Internal(format!("invalid financial action uuid: {e}")))
}

fn parse_rfc3339(name: &str, value: &str) -> Result<DateTime<Utc>, StorageError> {
    DateTime::parse_from_rfc3339(value)
        .map(|value| value.with_timezone(&Utc))
        .map_err(|e| StorageError::Internal(format!("{name}: {e}")))
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

fn execution_status_from_text(value: &str) -> Result<FinancialExecutionStatus, StorageError> {
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
