use tracing::instrument;

use crate::{
    Client, CreateFinancialActionRequest, CreateFinancialMandateRequest,
    FinancialActionListResponse, FinancialActionRecord, FinancialMandate,
    FinancialMandateListResponse, FinancialReceipt, SdkError,
};

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
        self.retry_loop("/v1/financial/mandates", || {
            self.send_post_json("/v1/financial/mandates", req)
        })
        .await
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
        self.retry_loop(&path, || self.send_post_empty(&path)).await
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
        self.retry_loop(&path, || self.send_post_empty(&path)).await
    }
}
