use std::collections::HashMap;

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use tl_core::{
    ApprovalRequirement, CreateFinancialActionRequest, CreateFinancialMandateRequest,
    FinancialActionListResponse, FinancialActionOutcome, FinancialActionRecord,
    FinancialActionStatus, FinancialApprovalRequest, FinancialApprovalRequestListResponse,
    FinancialApprovalRequestStatus, FinancialMandate, FinancialMandateListResponse,
    FinancialMandateStatus, FinancialOutcomeListResponse, FinancialReceipt,
};
use tokio::sync::RwLock;

use super::{
    validation::{is_valid_transition, validate_create_action},
    FinancialLedgerEntryKind, FinancialStore, FinancialStoreError,
};

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

#[async_trait]
impl FinancialStore for MemoryFinancialStore {
    async fn create_action(
        &self,
        workspace_id: &str,
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
