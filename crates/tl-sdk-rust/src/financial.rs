use tracing::instrument;

use crate::{
    AgenticPaymentAuthorizationResponse, AgenticPaymentAuthorizeRequest,
    AgenticPaymentCommitRequest, AgenticPaymentRecord, AgenticPaymentRollbackRequest,
    AuthorizationClaim, Client, CounterpartyRef, CreateFinancialActionRequest,
    CreateFinancialPolicyRequest, EvidenceRef, ExecuteFinancialActionRequest, FinancialAction,
    FinancialActionKind, FinancialActionListResponse, FinancialActionOutcome,
    FinancialActionRecord, FinancialOutcomeListResponse, FinancialPolicyListResponse,
    FinancialPolicyRecord, FinancialRail, FinancialReceipt, MoneyAmount, SdkError,
};

#[derive(Debug, Clone)]
pub struct FinancialOperation {
    operation: String,
    kind: FinancialActionKind,
    principal_id: String,
    rail: FinancialRail,
    authorization: Option<AuthorizationClaim>,
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
            authorization: None,
        }
    }

    pub fn with_authorization(mut self, authorization: AuthorizationClaim) -> Self {
        self.authorization = Some(authorization);
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
            authorization: self.authorization.clone(),
            action: FinancialAction {
                id: None,
                kind: self.kind,
                operation: self.operation.clone(),
                principal_id: self.principal_id.clone(),
                amount,
                counterparty,
                rail: self.rail,
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

    /// Execute an authorized financial action.
    #[instrument(
        name = "tl_sdk_rust::execute_action",
        skip_all,
        fields(action_id = %action_id, attempt = tracing::field::Empty),
    )]
    pub async fn execute_action(
        &self,
        action_id: &str,
        request: &ExecuteFinancialActionRequest,
    ) -> Result<FinancialActionRecord, SdkError> {
        let path = format!(
            "/v1/financial/actions/{}/execute",
            urlencoding::encode(action_id)
        );
        self.send_post_json(&path, request).await
    }
}
