use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine as _};
use ring::hmac;
use sha2::{Digest, Sha256};
use tracing::instrument;

use crate::{
    AgenticPaymentAuthorizationResponse, AgenticPaymentAuthorizeRequest,
    AgenticPaymentCommitRequest, AgenticPaymentRecord, AgenticPaymentRollbackRequest, Client,
    CommitFinancialActionRequest, CommitFinancialActionResponse, CounterpartyRef,
    CreateFinancialActionRequest, CreateFinancialExecutionConnectorRequest,
    CreateFinancialExecutionConnectorResponse, CreateFinancialMandateRequest,
    CreateFinancialObservationReviewRequest, CreateFinancialPolicyRequest, EvidenceRef,
    FinancialAction, FinancialActionDecisionReceipt, FinancialActionKind,
    FinancialActionListResponse, FinancialActionOutcome, FinancialActionRecord,
    FinancialApprovalRequestListResponse, FinancialExecutionConnector,
    FinancialExecutionConnectorListResponse, FinancialMandate, FinancialMandateListResponse,
    FinancialObservationReview, FinancialObservationReviewListResponse,
    FinancialObservationSummaryResponse, FinancialOutcomeListResponse, FinancialPolicyListResponse,
    FinancialPolicyRecord, FinancialRail, FinancialReceipt, MandateRef, MoneyAmount, SdkError,
};

pub fn financial_provider_proof_sha256(proof: &str) -> String {
    let digest = Sha256::digest(proof.as_bytes());
    let hex = digest
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect::<String>();
    format!("sha256:{hex}")
}

pub fn financial_execution_attestation_message(
    action_id: &str,
    request: &CommitFinancialActionRequest,
) -> Vec<u8> {
    let fields = [
        request.connector_id.as_str(),
        action_id,
        request.grant_id.as_str(),
        request.action_hash.as_str(),
        request.provider.as_str(),
        request.provider_reference.as_str(),
        request.provider_status.as_str(),
        request.executed_at.as_str(),
        request.idempotency_key.as_str(),
        request.provider_proof_sha256.as_str(),
    ];
    let mut message = String::from("tlg-financial-execution-attestation.v1");
    for field in fields {
        message.push('\n');
        message.push_str(&field.len().to_string());
        message.push(':');
        message.push_str(field);
    }
    message.into_bytes()
}

pub fn sign_financial_execution_attestation(
    plaintext_secret: &str,
    action_id: &str,
    request: &CommitFinancialActionRequest,
) -> Result<String, String> {
    let secret = URL_SAFE_NO_PAD
        .decode(plaintext_secret)
        .map_err(|error| format!("connector secret is invalid: {error}"))?;
    let signature = hmac::sign(
        &hmac::Key::new(hmac::HMAC_SHA256, &secret),
        &financial_execution_attestation_message(action_id, request),
    );
    Ok(format!("v1={}", URL_SAFE_NO_PAD.encode(signature.as_ref())))
}

#[derive(Debug, Clone)]
pub struct FinancialOperation {
    operation: String,
    kind: FinancialActionKind,
    principal_id: String,
    rail: FinancialRail,
    mandate: Option<MandateRef>,
}

impl FinancialOperation {
    pub fn new(
        operation: impl Into<String>,
        kind: FinancialActionKind,
        principal_id: impl Into<String>,
        rail: FinancialRail,
    ) -> Self {
        Self {
            operation: operation.into(),
            kind,
            principal_id: principal_id.into(),
            rail,
            mandate: None,
        }
    }

    pub fn with_mandate(mut self, mandate: MandateRef) -> Self {
        self.mandate = Some(mandate);
        self
    }

    pub fn build_request(
        &self,
        idempotency_key: impl Into<String>,
        amount: MoneyAmount,
        counterparty: Option<CounterpartyRef>,
        memo: Option<String>,
        metadata: serde_json::Value,
        evidence: Vec<EvidenceRef>,
        execute: bool,
    ) -> CreateFinancialActionRequest {
        CreateFinancialActionRequest {
            idempotency_key: idempotency_key.into(),
            execute,
            action: FinancialAction {
                id: None,
                kind: self.kind,
                operation: self.operation.clone(),
                principal_id: self.principal_id.clone(),
                amount,
                counterparty,
                rail: self.rail,
                mandate: self.mandate.clone(),
                memo,
                metadata,
            },
            evidence,
        }
    }
}

impl Client {
    /// Submit a typed financial action for authorization.
    #[instrument(
        name = "tl_sdk_rust::verify_action",
        skip_all,
        fields(
            action_kind = ?req.action.kind,
            principal_id = %req.action.principal_id,
            attempt = tracing::field::Empty,
        ),
    )]
    pub async fn verify_action(
        &self,
        req: &CreateFinancialActionRequest,
    ) -> Result<FinancialActionRecord, SdkError> {
        self.retry_loop("/v1/financial/actions", || {
            self.send_post_json("/v1/financial/actions", req)
        })
        .await
    }

    /// Convenience alias for payment/refund callers.
    ///
    /// This is the same contract as [`Client::verify_action`]; it exists
    /// so payment-oriented integrations can keep product-language method
    /// names without a second wire shape.
    #[instrument(
        name = "tl_sdk_rust::guard_payment",
        skip_all,
        fields(
            action_kind = ?req.action.kind,
            principal_id = %req.action.principal_id,
            attempt = tracing::field::Empty,
        ),
    )]
    pub async fn guard_payment(
        &self,
        req: &CreateFinancialActionRequest,
    ) -> Result<FinancialActionRecord, SdkError> {
        self.verify_action(req).await
    }

    /// Fetch a financial action by id.
    #[instrument(
        name = "tl_sdk_rust::get_financial_action",
        skip_all,
        fields(action_id = %action_id, attempt = tracing::field::Empty),
    )]
    pub async fn get_financial_action(
        &self,
        action_id: &str,
    ) -> Result<FinancialActionRecord, SdkError> {
        let path = format!("/v1/financial/actions/{}", urlencoding::encode(action_id));
        self.retry_loop(&path, || self.send_get(&path)).await
    }

    /// List financial actions visible to the authenticated workspace.
    #[instrument(
        name = "tl_sdk_rust::list_financial_actions",
        skip_all,
        fields(attempt = tracing::field::Empty),
    )]
    pub async fn list_financial_actions(&self) -> Result<FinancialActionListResponse, SdkError> {
        self.retry_loop("/v1/financial/actions", || {
            self.send_get("/v1/financial/actions")
        })
        .await
    }

    /// Authorize and reserve an x402 agentic payment before the agent signs or pays.
    #[instrument(
        name = "tl_sdk_rust::authorize_agentic_payment",
        skip_all,
        fields(
            principal_id = %req.principal_id,
            session_id = %req.session_id,
            attempt = tracing::field::Empty,
        ),
    )]
    pub async fn authorize_agentic_payment(
        &self,
        req: &AgenticPaymentAuthorizeRequest,
    ) -> Result<AgenticPaymentAuthorizationResponse, SdkError> {
        self.retry_loop("/v1/financial/agentic-payments/authorize", || {
            self.send_post_json("/v1/financial/agentic-payments/authorize", req)
        })
        .await
    }

    /// Fetch an x402 agentic payment record by canonical financial action id.
    #[instrument(
        name = "tl_sdk_rust::get_agentic_payment",
        skip_all,
        fields(action_id = %action_id, attempt = tracing::field::Empty),
    )]
    pub async fn get_agentic_payment(
        &self,
        action_id: &str,
    ) -> Result<AgenticPaymentRecord, SdkError> {
        let path = format!(
            "/v1/financial/agentic-payments/{}",
            urlencoding::encode(action_id)
        );
        self.retry_loop(&path, || self.send_get(&path)).await
    }

    /// Commit an authorized x402 payment after settlement proof is available.
    #[instrument(
        name = "tl_sdk_rust::commit_agentic_payment",
        skip_all,
        fields(action_id = %action_id, attempt = tracing::field::Empty),
    )]
    pub async fn commit_agentic_payment(
        &self,
        action_id: &str,
        req: &AgenticPaymentCommitRequest,
    ) -> Result<AgenticPaymentRecord, SdkError> {
        let path = format!(
            "/v1/financial/agentic-payments/{}/commit",
            urlencoding::encode(action_id)
        );
        self.send_post_json(&path, req).await
    }

    /// Release an x402 payment reservation when the agent does not settle.
    #[instrument(
        name = "tl_sdk_rust::rollback_agentic_payment",
        skip_all,
        fields(action_id = %action_id, attempt = tracing::field::Empty),
    )]
    pub async fn rollback_agentic_payment(
        &self,
        action_id: &str,
        req: &AgenticPaymentRollbackRequest,
    ) -> Result<AgenticPaymentRecord, SdkError> {
        let path = format!(
            "/v1/financial/agentic-payments/{}/rollback",
            urlencoding::encode(action_id)
        );
        self.send_post_json(&path, req).await
    }

    /// Fetch the signed financial receipt for an x402 agentic payment.
    #[instrument(
        name = "tl_sdk_rust::get_agentic_payment_receipt",
        skip_all,
        fields(action_id = %action_id, attempt = tracing::field::Empty),
    )]
    pub async fn get_agentic_payment_receipt(
        &self,
        action_id: &str,
    ) -> Result<FinancialReceipt, SdkError> {
        let path = format!(
            "/v1/financial/agentic-payments/{}/receipt",
            urlencoding::encode(action_id)
        );
        self.retry_loop(&path, || self.send_get(&path)).await
    }

    pub async fn commit_financial_action(
        &self,
        action_id: &str,
        req: &CommitFinancialActionRequest,
    ) -> Result<CommitFinancialActionResponse, SdkError> {
        let path = format!(
            "/v1/financial/actions/{}/commit",
            urlencoding::encode(action_id)
        );
        self.send_post_json(&path, req).await
    }

    pub async fn create_financial_execution_connector(
        &self,
        req: &CreateFinancialExecutionConnectorRequest,
    ) -> Result<CreateFinancialExecutionConnectorResponse, SdkError> {
        self.send_post_json("/v1/financial/execution-connectors", req)
            .await
    }

    pub async fn list_financial_execution_connectors(
        &self,
    ) -> Result<FinancialExecutionConnectorListResponse, SdkError> {
        self.retry_loop("/v1/financial/execution-connectors", || {
            self.send_get("/v1/financial/execution-connectors")
        })
        .await
    }

    pub async fn revoke_financial_execution_connector(
        &self,
        connector_id: &str,
    ) -> Result<FinancialExecutionConnector, SdkError> {
        let path = format!(
            "/v1/financial/execution-connectors/{}/revoke",
            urlencoding::encode(connector_id)
        );
        self.send_post_json(&path, &serde_json::json!({})).await
    }

    pub async fn financial_observation_summary(
        &self,
        start: &str,
        end: &str,
    ) -> Result<FinancialObservationSummaryResponse, SdkError> {
        let path = format!(
            "/v1/financial/observations/summary?start={}&end={}",
            urlencoding::encode(start),
            urlencoding::encode(end)
        );
        self.retry_loop(&path, || self.send_get(&path)).await
    }

    pub async fn create_financial_observation_review(
        &self,
        action_id: &str,
        req: &CreateFinancialObservationReviewRequest,
    ) -> Result<FinancialObservationReview, SdkError> {
        let path = format!(
            "/v1/financial/actions/{}/observation-reviews",
            urlencoding::encode(action_id)
        );
        self.send_post_json(&path, req).await
    }

    pub async fn list_financial_observation_reviews(
        &self,
        action_id: &str,
    ) -> Result<FinancialObservationReviewListResponse, SdkError> {
        let path = format!(
            "/v1/financial/actions/{}/observation-reviews",
            urlencoding::encode(action_id)
        );
        self.retry_loop(&path, || self.send_get(&path)).await
    }

    /// Create or update a financial spending control.
    #[instrument(
        name = "tl_sdk_rust::create_financial_policy",
        skip_all,
        fields(policy_id = %req.id, attempt = tracing::field::Empty),
    )]
    pub async fn create_financial_policy(
        &self,
        req: &CreateFinancialPolicyRequest,
    ) -> Result<FinancialPolicyRecord, SdkError> {
        self.send_post_json("/v1/financial/policies", req).await
    }

    /// List financial spending controls visible to the authenticated workspace.
    #[instrument(
        name = "tl_sdk_rust::list_financial_policies",
        skip_all,
        fields(attempt = tracing::field::Empty),
    )]
    pub async fn list_financial_policies(&self) -> Result<FinancialPolicyListResponse, SdkError> {
        self.retry_loop("/v1/financial/policies", || {
            self.send_get("/v1/financial/policies")
        })
        .await
    }

    /// Create a durable financial mandate.
    #[instrument(
        name = "tl_sdk_rust::create_mandate",
        skip_all,
        fields(principal_id = %req.principal_id, attempt = tracing::field::Empty),
    )]
    pub async fn create_mandate(
        &self,
        req: &CreateFinancialMandateRequest,
    ) -> Result<FinancialMandate, SdkError> {
        self.send_post_json("/v1/financial/mandates", req).await
    }

    /// List durable financial mandates visible to the authenticated workspace.
    #[instrument(
        name = "tl_sdk_rust::list_mandates",
        skip_all,
        fields(attempt = tracing::field::Empty),
    )]
    pub async fn list_mandates(&self) -> Result<FinancialMandateListResponse, SdkError> {
        self.retry_loop("/v1/financial/mandates", || {
            self.send_get("/v1/financial/mandates")
        })
        .await
    }

    /// List pending and decided financial approval requests visible to the authenticated workspace.
    #[instrument(
        name = "tl_sdk_rust::list_approval_requests",
        skip_all,
        fields(attempt = tracing::field::Empty),
    )]
    pub async fn list_approval_requests(
        &self,
    ) -> Result<FinancialApprovalRequestListResponse, SdkError> {
        self.retry_loop("/v1/financial/approval-requests", || {
            self.send_get("/v1/financial/approval-requests")
        })
        .await
    }

    /// Revoke a financial mandate.
    #[instrument(
        name = "tl_sdk_rust::revoke_mandate",
        skip_all,
        fields(mandate_id = %mandate_id, attempt = tracing::field::Empty),
    )]
    pub async fn revoke_mandate(&self, mandate_id: &str) -> Result<FinancialMandate, SdkError> {
        let path = format!(
            "/v1/financial/mandates/{}/revoke",
            urlencoding::encode(mandate_id)
        );
        self.send_post_empty(&path).await
    }

    /// Fetch a financial receipt/proof by id.
    #[instrument(
        name = "tl_sdk_rust::get_receipt",
        skip_all,
        fields(receipt_id = %receipt_id, attempt = tracing::field::Empty),
    )]
    pub async fn get_receipt(&self, receipt_id: &str) -> Result<FinancialReceipt, SdkError> {
        let path = format!("/v1/financial/receipts/{}", urlencoding::encode(receipt_id));
        self.retry_loop(&path, || self.send_get(&path)).await
    }

    /// Fetch the per-action decision receipt before or after execution.
    #[instrument(
        name = "tl_sdk_rust::get_financial_decision_receipt",
        skip_all,
        fields(action_id = %action_id, attempt = tracing::field::Empty),
    )]
    pub async fn get_financial_decision_receipt(
        &self,
        action_id: &str,
    ) -> Result<FinancialActionDecisionReceipt, SdkError> {
        let path = format!(
            "/v1/financial/actions/{}/decision-receipt",
            urlencoding::encode(action_id)
        );
        self.retry_loop(&path, || self.send_get(&path)).await
    }

    /// Record provider outcome or recovery status for a financial action.
    #[instrument(
        name = "tl_sdk_rust::record_action_outcome",
        skip_all,
        fields(action_id = %action_id, attempt = tracing::field::Empty),
    )]
    pub async fn record_action_outcome(
        &self,
        action_id: &str,
        outcome: &FinancialActionOutcome,
    ) -> Result<FinancialActionOutcome, SdkError> {
        let path = format!(
            "/v1/financial/actions/{}/outcomes",
            urlencoding::encode(action_id)
        );
        self.send_post_json(&path, outcome).await
    }

    /// List provider outcomes and recovery history for a financial action.
    #[instrument(
        name = "tl_sdk_rust::list_action_outcomes",
        skip_all,
        fields(action_id = %action_id, attempt = tracing::field::Empty),
    )]
    pub async fn list_action_outcomes(
        &self,
        action_id: &str,
    ) -> Result<FinancialOutcomeListResponse, SdkError> {
        let path = format!(
            "/v1/financial/actions/{}/outcomes",
            urlencoding::encode(action_id)
        );
        self.retry_loop(&path, || self.send_get(&path)).await
    }

    /// Approve a held or proposed financial action.
    #[instrument(
        name = "tl_sdk_rust::approve_action",
        skip_all,
        fields(action_id = %action_id, attempt = tracing::field::Empty),
    )]
    pub async fn approve_action(&self, action_id: &str) -> Result<FinancialActionRecord, SdkError> {
        self.transition_financial_action(action_id, "approve").await
    }

    /// Deny a pending financial action.
    #[instrument(
        name = "tl_sdk_rust::deny_action",
        skip_all,
        fields(action_id = %action_id, attempt = tracing::field::Empty),
    )]
    pub async fn deny_action(&self, action_id: &str) -> Result<FinancialActionRecord, SdkError> {
        self.transition_financial_action(action_id, "deny").await
    }

    /// Execute an authorized financial action.
    #[instrument(
        name = "tl_sdk_rust::execute_action",
        skip_all,
        fields(action_id = %action_id, attempt = tracing::field::Empty),
    )]
    pub async fn execute_action(&self, action_id: &str) -> Result<FinancialActionRecord, SdkError> {
        self.transition_financial_action(action_id, "execute").await
    }

    async fn transition_financial_action(
        &self,
        action_id: &str,
        transition: &str,
    ) -> Result<FinancialActionRecord, SdkError> {
        let path = format!(
            "/v1/financial/actions/{}/{}",
            urlencoding::encode(action_id),
            transition
        );
        self.send_post_empty(&path).await
    }
}

#[cfg(test)]
mod attestation_tests {
    use super::*;

    #[test]
    fn signing_matches_shared_cross_language_vector() {
        let request = CommitFinancialActionRequest {
            connector_id: "connector-1".into(),
            grant_id: "grant-1".into(),
            action_hash: "sha256:action".into(),
            provider: "stripe".into(),
            provider_reference: "pi_123".into(),
            provider_status: "succeeded".into(),
            executed_at: "2026-07-09T00:00:00Z".into(),
            idempotency_key: "commit-1".into(),
            provider_proof: "provider receipt".into(),
            provider_proof_sha256: "sha256:proof".into(),
            signature: String::new(),
        };
        let signature = sign_financial_execution_attestation(
            "MDEyMzQ1Njc4OTAxMjM0NTY3ODkwMTIzNDU2Nzg5MDE",
            "action-1",
            &request,
        )
        .unwrap();
        assert_eq!(signature, "v1=FbjzlmAsFdGVBB5yKbLD6UZ6-CgtIXZCtByHv49nXpY");
    }
}
