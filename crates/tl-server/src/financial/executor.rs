use std::sync::Arc;

use async_trait::async_trait;
use serde_json::Value;
use tl_core::{FinancialActionRecord, GatewayProviderKind, RecoveryStatus, ReversalCapability};

use crate::gateway::{forward_payment, unseal_provider_key, GatewayStore};

#[derive(Debug, Clone, PartialEq)]
pub struct FinancialExecutionResult {
    pub provider_status: Option<String>,
    pub provider_reference: Option<String>,
    pub provider_response: Value,
    pub reversal_capability: ReversalCapability,
    pub recovery_status: RecoveryStatus,
}

#[derive(Debug, thiserror::Error)]
pub enum FinancialExecutionError {
    #[error("no payment_http provider connection configured")]
    NoProvider,
    #[error("{0}")]
    Failed(String),
}

#[async_trait]
pub trait FinancialExecutor: Send + Sync {
    async fn execute(
        &self,
        workspace_id: &str,
        action: &FinancialActionRecord,
        idempotency_key: &str,
    ) -> Result<FinancialExecutionResult, FinancialExecutionError>;
}

#[derive(Clone)]
pub struct PaymentHttpFinancialExecutor {
    gateway_store: Arc<dyn GatewayStore>,
    seal_key: [u8; 32],
    http: reqwest::Client,
}

impl PaymentHttpFinancialExecutor {
    pub fn new(
        gateway_store: Arc<dyn GatewayStore>,
        seal_key: [u8; 32],
        http: reqwest::Client,
    ) -> Self {
        Self {
            gateway_store,
            seal_key,
            http,
        }
    }
}

#[async_trait]
impl FinancialExecutor for PaymentHttpFinancialExecutor {
    async fn execute(
        &self,
        workspace_id: &str,
        action: &FinancialActionRecord,
        idempotency_key: &str,
    ) -> Result<FinancialExecutionResult, FinancialExecutionError> {
        let connections = self
            .gateway_store
            .list_provider_connections(workspace_id)
            .await
            .map_err(|e| FinancialExecutionError::Failed(format!("provider lookup failed: {e}")))?;
        let Some(connection) = connections
            .into_iter()
            .find(|connection| connection.kind == GatewayProviderKind::PaymentHttp)
        else {
            return Err(FinancialExecutionError::NoProvider);
        };
        let secret = self
            .gateway_store
            .get_provider_connection_secret(workspace_id, &connection.id)
            .await
            .map_err(|e| {
                FinancialExecutionError::Failed(format!("provider credential lookup failed: {e}"))
            })?;
        let api_key = unseal_provider_key(&secret.encrypted_api_key, &self.seal_key)
            .map_err(FinancialExecutionError::Failed)?;
        let body = provider_body(action);
        let provider_response =
            forward_payment(&self.http, &connection, &api_key, idempotency_key, &body)
                .await
                .map_err(FinancialExecutionError::Failed)?;

        Ok(FinancialExecutionResult {
            provider_status: string_field(&provider_response, &["status", "provider_status"]),
            provider_reference: string_field(
                &provider_response,
                &["provider_reference", "reference", "id"],
            ),
            reversal_capability: reversal_capability(&provider_response),
            recovery_status: recovery_status(&provider_response),
            provider_response,
        })
    }
}

fn provider_body(action: &FinancialActionRecord) -> Value {
    let counterparty = action.action.counterparty.as_ref();
    serde_json::json!({
        "action_id": action.id,
        "kind": action.action.kind,
        "amount": action.action.amount.amount_minor,
        "amount_minor": action.action.amount.amount_minor,
        "currency": action.action.amount.currency.clone(),
        "counterparty": action.action.counterparty.clone(),
        "merchant": counterparty
            .and_then(|value| value.display_name.clone())
            .or_else(|| counterparty.map(|value| value.id.clone())),
        "memo": action.action.memo.clone(),
        "metadata": action.action.metadata.clone(),
    })
}

fn string_field(value: &Value, names: &[&str]) -> Option<String> {
    names
        .iter()
        .find_map(|name| value.get(name).and_then(Value::as_str))
        .map(str::to_string)
}

fn reversal_capability(value: &Value) -> ReversalCapability {
    match string_field(value, &["reversal_capability"]).as_deref() {
        Some("cancel_before_capture") => ReversalCapability::CancelBeforeCapture,
        Some("cancel_pending_refund") => ReversalCapability::CancelPendingRefund,
        Some("provider_reversal") => ReversalCapability::ProviderReversal,
        Some("compensating_charge") => ReversalCapability::CompensatingCharge,
        Some("internal_balance_adjustment") => ReversalCapability::InternalBalanceAdjustment,
        Some("manual_recovery") => ReversalCapability::ManualRecovery,
        _ => ReversalCapability::None,
    }
}

fn recovery_status(value: &Value) -> RecoveryStatus {
    match string_field(value, &["recovery_status"]).as_deref() {
        Some("not_available") => RecoveryStatus::NotAvailable,
        Some("available") => RecoveryStatus::Available,
        Some("started") => RecoveryStatus::Started,
        Some("recovered") => RecoveryStatus::Recovered,
        Some("failed") => RecoveryStatus::Failed,
        Some("manual_required") => RecoveryStatus::ManualRequired,
        _ => RecoveryStatus::NotNeeded,
    }
}
