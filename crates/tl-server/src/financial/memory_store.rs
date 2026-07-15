use std::collections::HashMap;

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use tl_core::{
    AgenticPaymentReservation, AgenticPaymentReservationStatus, AuthorizationEffect,
    AuthorizationIntentStatus, CreateFinancialActionRequest, FinancialActionListResponse,
    FinancialActionOutcome, FinancialActionRecord, FinancialExecutionStatus,
    FinancialOutcomeListResponse, FinancialReceipt, MoneyAmount,
};
use tokio::sync::{Mutex, RwLock};

use super::{
    validation::{is_valid_execution_transition, validate_create_action},
    AgenticPaymentBudgetReservationRequest, FinancialBudgetReservationOutcome,
    FinancialBudgetReservationRequest, FinancialBudgetViolation, FinancialBudgetWindow,
    FinancialLedgerEntryKind, FinancialStore, FinancialStoreError,
};

#[derive(Debug, Default)]
pub struct MemoryFinancialStore {
    actions: RwLock<HashMap<String, FinancialActionRecord>>,
    idempotency: RwLock<HashMap<String, String>>,
    receipts: RwLock<HashMap<String, FinancialReceipt>>,
    outcomes: RwLock<HashMap<String, Vec<FinancialActionOutcome>>>,
    ledger_entries: RwLock<HashMap<String, MemoryLedgerEntry>>,
    ledger_idempotency: RwLock<HashMap<String, String>>,
    action_budget_lock: Mutex<()>,
    agentic_payments: RwLock<MemoryAgenticPayments>,
}

impl MemoryFinancialStore {
    pub fn new() -> Self {
        Self::default()
    }

    async fn find_action(
        &self,
        workspace_id: &str,
        action_id: &str,
    ) -> Result<FinancialActionRecord, FinancialStoreError> {
        self.actions
            .read()
            .await
            .values()
            .find(|action| action.workspace_id == workspace_id && action.id == action_id)
            .cloned()
            .ok_or(FinancialStoreError::NotFound)
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
        let environment_id = clean_required("environment_id", environment_id)?;
        let idempotency_key = format!(
            "{workspace_id}:{environment_id}:{}",
            input.idempotency_key.trim()
        );
        let mut idempotency = self.idempotency.write().await;
        if let Some(action_id) = idempotency.get(&idempotency_key).cloned() {
            drop(idempotency);
            return self
                .get_action(workspace_id, &environment_id, &action_id)
                .await;
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
            environment_id: environment_id.clone(),
            authorization_intent_id: None,
            authorization_receipt_id: None,
            authorization_effect: AuthorizationEffect::Defer,
            authorization_status: AuthorizationIntentStatus::Evaluating,
            authorization: None,
            execution_status: FinancialExecutionStatus::NotStarted,
            status_reason: None,
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
        let action_key = action_key(workspace_id, &environment_id, &id);
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
        environment_id: &str,
        action_id: &str,
    ) -> Result<FinancialActionRecord, FinancialStoreError> {
        self.actions
            .read()
            .await
            .get(&action_key(workspace_id, environment_id, action_id))
            .cloned()
            .ok_or(FinancialStoreError::NotFound)
    }

    async fn list_actions(
        &self,
        workspace_id: &str,
        environment_id: Option<&str>,
    ) -> Result<FinancialActionListResponse, FinancialStoreError> {
        let mut actions = self
            .actions
            .read()
            .await
            .values()
            .filter(|action| {
                action.workspace_id == workspace_id
                    && environment_id.map_or(true, |id| action.environment_id == id)
            })
            .cloned()
            .collect::<Vec<_>>();
        actions.sort_by(|a, b| {
            b.created_at
                .cmp(&a.created_at)
                .then_with(|| b.id.cmp(&a.id))
        });
        Ok(FinancialActionListResponse { actions })
    }

    async fn update_authorization(
        &self,
        workspace_id: &str,
        environment_id: &str,
        action_id: &str,
        intent_id: Option<&str>,
        receipt_id: Option<&str>,
        effect: AuthorizationEffect,
        status: AuthorizationIntentStatus,
    ) -> Result<FinancialActionRecord, FinancialStoreError> {
        let mut actions = self.actions.write().await;
        let record = actions
            .get_mut(&action_key(workspace_id, environment_id, action_id))
            .ok_or(FinancialStoreError::NotFound)?;
        record.authorization_intent_id = intent_id.map(str::to_string);
        record.authorization_receipt_id = receipt_id.map(str::to_string);
        record.authorization_effect = effect;
        record.authorization_status = status;
        record.updated_at = Utc::now().to_rfc3339();
        Ok(record.clone())
    }

    async fn transition_execution(
        &self,
        workspace_id: &str,
        environment_id: &str,
        action_id: &str,
        status: FinancialExecutionStatus,
        reason: Option<&str>,
    ) -> Result<FinancialActionRecord, FinancialStoreError> {
        let mut actions = self.actions.write().await;
        let record = actions
            .get_mut(&action_key(workspace_id, environment_id, action_id))
            .ok_or(FinancialStoreError::NotFound)?;
        if record.execution_status != status
            && !is_valid_execution_transition(record.execution_status, status)
        {
            return Err(FinancialStoreError::Conflict);
        }
        record.execution_status = status;
        record.status_reason = reason.map(str::to_string);
        record.updated_at = Utc::now().to_rfc3339();
        Ok(record.clone())
    }

    async fn create_receipt(
        &self,
        workspace_id: &str,
        action_id: &str,
        authorization_receipt_id: &str,
        trace_id: Option<&str>,
        ledger_event_ids: Vec<String>,
        proof: serde_json::Value,
    ) -> Result<FinancialReceipt, FinancialStoreError> {
        self.find_action(workspace_id, action_id).await?;
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
            authorization_receipt_id: clean_required(
                "authorization_receipt_id",
                authorization_receipt_id,
            )?,
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
        self.find_action(workspace_id, action_id).await?;
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
        self.find_action(workspace_id, action_id).await?;
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

        let action = self.find_action(workspace_id, action_id).await?;
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

    async fn try_reserve_action_budget(
        &self,
        request: FinancialBudgetReservationRequest,
    ) -> Result<FinancialBudgetReservationOutcome, FinancialStoreError> {
        let _guard = self.action_budget_lock.lock().await;
        let FinancialBudgetReservationRequest {
            workspace_id,
            action_id,
            principal_id,
            amount,
            idempotency_key,
            day_start,
            week_start,
            month_start,
            now,
            constraints,
            metadata,
        } = request;
        let existing_key = key(&workspace_id, idempotency_key.trim());
        if let Some(entry_id) = self
            .ledger_idempotency
            .read()
            .await
            .get(&existing_key)
            .cloned()
        {
            return Ok(FinancialBudgetReservationOutcome::Reserved {
                ledger_entry_id: entry_id,
                violations: vec![],
            });
        }
        let action = self.find_action(&workspace_id, &action_id).await?;
        let currency = clean_required("currency", &amount.currency)?.to_uppercase();
        if action.action.principal_id != principal_id
            || action.action.amount.amount_minor != amount.amount_minor
            || !action
                .action
                .amount
                .currency
                .eq_ignore_ascii_case(&currency)
        {
            return Err(FinancialStoreError::Conflict);
        }

        let entries = self.ledger_entries.read().await;
        let mut violations = Vec::new();
        for constraint in constraints {
            let start = match constraint.window {
                FinancialBudgetWindow::Day => day_start,
                FinancialBudgetWindow::Week => week_start,
                FinancialBudgetWindow::Month => month_start,
            };
            let committed_minor = entries
                .values()
                .filter(|entry| {
                    entry.workspace_id == workspace_id
                        && entry.principal_id == principal_id
                        && entry.currency == currency
                        && entry.effective_at >= start
                        && entry.effective_at < now
                })
                .map(|entry| signed_ledger_amount(entry.kind, entry.amount_minor))
                .sum::<i64>();
            if committed_minor.saturating_add(amount.amount_minor) > constraint.cap_minor {
                violations.push(FinancialBudgetViolation {
                    policy_id: constraint.policy_id,
                    window: constraint.window,
                    cap_minor: constraint.cap_minor,
                    committed_minor,
                    requested_minor: amount.amount_minor,
                    block_on_breach: constraint.block_on_breach,
                });
            }
        }
        drop(entries);
        if violations.iter().any(|violation| violation.block_on_breach) {
            return Ok(FinancialBudgetReservationOutcome::Denied { violations });
        }

        let ledger_entry_id = self
            .record_ledger_entry(
                &workspace_id,
                &action_id,
                FinancialLedgerEntryKind::Reserved,
                amount.amount_minor,
                &currency,
                &idempotency_key,
                metadata,
            )
            .await?;
        Ok(FinancialBudgetReservationOutcome::Reserved {
            ledger_entry_id,
            violations,
        })
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

fn action_key(workspace_id: &str, environment_id: &str, action_id: &str) -> String {
    format!("{workspace_id}:{environment_id}:{action_id}")
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
