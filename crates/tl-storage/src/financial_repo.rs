use chrono::{DateTime, Utc};
use diesel::prelude::*;
use diesel_async::RunQueryDsl;
use serde::de::DeserializeOwned;
use serde::Serialize;
use tl_core::{
    CreateFinancialActionRequest, EvidenceRef, FinancialAction, FinancialActionKind,
    FinancialActionStatus, FinancialApprovalRequestStatus, FinancialRail, MoneyAmount,
};
use uuid::Uuid;

use crate::models::{
    ApprovalRequestRecord, FinancialActionEventRecord, FinancialActionRecord,
    FinancialLedgerEntryRecord, NewApprovalRequest, NewFinancialAction, NewFinancialActionEvent,
    NewFinancialLedgerEntry,
};
use crate::postgres::{DbConnection, DbPool};
use crate::schema::{
    approval_requests, financial_action_events, financial_actions, financial_ledger_entries,
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
    pub idempotency_key: String,
    pub status: FinancialActionStatus,
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
            idempotency_key: idempotency_key.clone(),
            principal_id: input.action.principal_id.trim().to_string(),
            action_kind: enum_text(input.action.kind)?,
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
        action_from_record(record)
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
        rows.into_iter().map(action_from_record).collect()
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
                .filter(financial_actions::id.eq(action_uuid)),
        )
        .set((
            financial_actions::status.eq(enum_text(next_status)?),
            financial_actions::updated_at.eq(Utc::now()),
        ))
        .execute(&mut conn)
        .await
        .map_err(|e| StorageError::Internal(format!("financial action transition update: {e}")))?;

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

    pub async fn record_ledger_entry(
        &self,
        workspace_id: &str,
        action_id: &str,
        kind: FinancialLedgerEntryKind,
        amount_minor: i64,
        currency: &str,
        idempotency_key: &str,
        metadata: serde_json::Value,
    ) -> Result<(), StorageError> {
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
            currency: clean_currency,
            idempotency_key: clean_idempotency_key,
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
        Ok(())
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
            reason: None,
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
        idempotency_key: record.idempotency_key,
        status,
        action: FinancialAction {
            id: Some(record.id.to_string()),
            kind,
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

fn parse_uuid(value: &str) -> Result<Uuid, StorageError> {
    Uuid::parse_str(value)
        .map_err(|e| StorageError::Internal(format!("invalid financial action uuid: {e}")))
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
