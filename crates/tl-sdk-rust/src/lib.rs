//! TrustLoopGuard Rust SDK.
//!
//! Async client over reqwest with typed errors, exponential-backoff
//! retries (honoring `Retry-After`), bearer-token auth, and `tracing`
//! spans on every call. The retry policy lives in [`RetryConfig`] —
//! callers can swap in their own (voice-channel callers should usually
//! disable retries with `max_attempts = 1`).

mod error;
mod events;
mod financial;
mod guardrails;
mod http;
mod policies;
mod retry;
mod runs;
#[cfg(test)]
mod tests;

pub use error::SdkError;
pub use financial::{
    financial_execution_attestation_message, financial_provider_proof_sha256,
    sign_financial_execution_attestation, FinancialOperation,
};
pub use retry::RetryConfig;
pub use runs::RunClient;

// Re-export the wire types so callers don't reach into `tl_core`
// directly. Doing so would violate the SDK-driven discipline (rule 2 in
// docs/SDK_DRIVEN.md) and break example apps that lint against internal
// imports.
pub use tl_core::{
    Action, AgenticPaymentAuthorizationResponse, AgenticPaymentAuthorizeRequest,
    AgenticPaymentCommitRequest, AgenticPaymentDecision, AgenticPaymentMandateScope,
    AgenticPaymentRecord, AgenticPaymentReservation, AgenticPaymentReservationStatus,
    AgenticPaymentRollbackRequest, AllowedSource, ApiError, ApiErrorCode, ApprovalRequirement,
    ApprovalRule, Channel, CommitFinancialActionRequest, CommitFinancialActionResponse,
    Confidentiality, CounterpartyRef, CreateFinancialActionRequest,
    CreateFinancialExecutionConnectorRequest, CreateFinancialExecutionConnectorResponse,
    CreateFinancialMandateRequest, CreateFinancialObservationReviewRequest,
    CreateFinancialPolicyRequest, CreateRunEventRequest, CreateRunRequest, Decision, EventKind,
    EvidenceRef, FinancialAction, FinancialActionDecision, FinancialActionDecisionReceipt,
    FinancialActionKind, FinancialActionListResponse, FinancialActionOutcome,
    FinancialActionOutcomeStatus, FinancialActionPrecondition, FinancialActionRecord,
    FinancialActionStatus, FinancialApprovalRequest, FinancialApprovalRequestListResponse,
    FinancialApprovalRequestStatus, FinancialAuthorizationScopeProof, FinancialDecision,
    FinancialDecisionRisk, FinancialDecisionRiskCode, FinancialEligibilityCheck,
    FinancialEligibilityResult, FinancialEligibilityStatus, FinancialEvidenceProof,
    FinancialExecutionConnector, FinancialExecutionConnectorListResponse, FinancialExecutionGrant,
    FinancialExecutionProof, FinancialExecutionProofStatus, FinancialMandate,
    FinancialMandateListResponse, FinancialMandateStatus, FinancialObservationReview,
    FinancialObservationReviewListResponse, FinancialObservationSummaryResponse,
    FinancialOutcomeListResponse, FinancialPolicyListResponse, FinancialPolicyRecord,
    FinancialPolicySelector, FinancialRail, FinancialReceipt, GuardEvent,
    GuardrailGenerateResponse, GuardrailListResponse, Integrity, Labels, MandateRef, MoneyAmount,
    Origin, ParamRole, ParamSpec, PolicyAction, PolicyDocument, PolicyFamily, PolicyListResponse,
    PolicySummary, Principal, ProvenanceMap, RecoveryStatus, ReversalCapability, RunDetail,
    RunEventKind, RunEventListResponse, RunEventSummary, RunKind, RunListResponse, RunStatus,
    RunSummary, Severity, SideEffectClass, Source, SpendMeter, ToolMetadata, TraceListResponse,
    TriggeredPolicy, Trust, UpdateRunRequest, Verdict, X402NormalizedPaymentRequirement,
    X402PaymentRequirement, X402SettlementProof,
};

// GuardEvent context and parameters use `serde_json::Value` on the wire.
// Re-export the crate so example apps don't take a separate dependency.
pub use serde_json;

#[derive(Debug, Clone)]
pub struct Client {
    base_url: String,
    api_key: Option<String>,
    http: reqwest::Client,
    retry: RetryConfig,
    session_id: Option<String>,
}

impl Client {
    pub fn new(base_url: impl Into<String>) -> Self {
        Self {
            base_url: base_url.into(),
            api_key: None,
            http: reqwest::Client::new(),
            retry: RetryConfig::default(),
            session_id: None,
        }
    }

    pub fn with_api_key(mut self, key: impl Into<String>) -> Self {
        self.api_key = Some(key.into());
        self
    }

    /// Enable monitoring: generates a session id (`sess_<uuid-v7>`)
    /// attached to the principal of every outgoing event that does not
    /// already carry one. Off by default; caller-explicit session ids
    /// always win. The id is opaque to the server and only groups this
    /// client's traces for session-scoped queries
    /// (`GET /v1/traces?session_id=...`).
    pub fn with_monitoring(mut self) -> Self {
        self.session_id = Some(format!("sess_{}", uuid::Uuid::now_v7()));
        self
    }

    /// The monitoring session id, if monitoring is enabled.
    pub fn session_id(&self) -> Option<&str> {
        self.session_id.as_deref()
    }

    /// Override the retry policy. Voice callers typically pass
    /// `RetryConfig { max_attempts: 1, ..Default::default() }` to opt out.
    pub fn with_retry(mut self, retry: RetryConfig) -> Self {
        self.retry = retry;
        self
    }

    /// Override the underlying reqwest client (for custom timeouts,
    /// proxies, or test fixtures).
    pub fn with_http_client(mut self, http: reqwest::Client) -> Self {
        self.http = http;
        self
    }
}

/// Synthesize an `ApiError` from a raw status when the server didn't
/// return our canonical body. Crate-private; the error module needs it.
pub(crate) fn synthesize_api_error(status: u16, body: &str) -> tl_core::ApiError {
    let code = tl_core::ApiErrorCode::from_http_status(status);
    tl_core::ApiError {
        code,
        message: if body.is_empty() {
            format!("server returned status {status}")
        } else {
            body.to_string()
        },
        retriable: code.default_retriable(),
        details: serde_json::Value::Null,
    }
}
