//! Core types for TrustLoopGuard. Stable across all other crates.
//!
//! # Versioning
//!
//! These types are the **wire format**. Compatibility is enforced at the
//! HTTP layer via the URL path (`/v1/...`, `/v2/...`), not via a body
//! discriminator. When the wire shape needs to break, copy this module
//! into `crates/tl-core/src/v2.rs` and let both compile in parallel.
//!
//! # Codegen
//!
//! `tl-codegen` reads these types and emits:
//! - `docs/openapi.yaml` (via `utoipa`)
//! - `policies/schema.json` (via `schemars`)
//! - `sdks/typescript/src/types.ts` (via `ts-rs`)
//!
//! CI fails if the committed artifacts diverge from what the derives produce.
//! Do not hand-edit those files.

use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[cfg(feature = "schema")]
use schemars::JsonSchema;
#[cfg(feature = "ts-export")]
use ts_rs::TS;
#[cfg(feature = "openapi")]
use utoipa::ToSchema;

pub mod agent;
pub mod policy;
pub mod tier;

pub use agent::{AgentAuthority, AgentProfile, AgentScope, AgentTone, KnowledgeSource};
pub use policy::{PolicyValidateResponse, PolicyValidationIssue};
pub use tier::{Tier, TierResult, TierStatus};

/// Channel an agent is operating on. Drives latency budget and matcher selection.
///
/// Flat enum on the wire so SDK type generation stays clean across languages.
/// New channels are added as variants here; we don't carry a free-form
/// `Other(String)` because it pollutes the Pydantic / TS surface.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
#[cfg_attr(feature = "schema", derive(JsonSchema))]
#[cfg_attr(feature = "openapi", derive(ToSchema))]
#[cfg_attr(feature = "ts-export", derive(TS))]
#[cfg_attr(feature = "ts-export", ts(export))]
pub enum Channel {
    Voice,
    Chat,
    Email,
}

/// What TrustLoopGuard tells the caller to do with the proposed output.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
#[cfg_attr(feature = "schema", derive(JsonSchema))]
#[cfg_attr(feature = "openapi", derive(ToSchema))]
#[cfg_attr(feature = "ts-export", derive(TS))]
#[cfg_attr(feature = "ts-export", ts(export))]
pub enum Verdict {
    Allow,
    Block,
    Rewrite,
    Escalate,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
#[cfg_attr(feature = "schema", derive(JsonSchema))]
#[cfg_attr(feature = "openapi", derive(ToSchema))]
#[cfg_attr(feature = "ts-export", derive(TS))]
#[cfg_attr(feature = "ts-export", ts(export))]
pub enum Severity {
    Low,
    Medium,
    High,
    Critical,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(JsonSchema))]
#[cfg_attr(feature = "openapi", derive(ToSchema))]
#[cfg_attr(feature = "ts-export", derive(TS))]
#[cfg_attr(feature = "ts-export", ts(export))]
pub struct CheckRequest {
    pub agent_id: String,
    pub channel: Channel,
    pub input: String,
    pub proposed_output: String,
    /// Optional domain selector for the dispatcher. Defaults to
    /// `customer_support` server-side when absent. Reserved for future
    /// `voice_agent` / `coding_agent` handlers.
    #[serde(default)]
    pub domain: Option<String>,
    #[serde(default)]
    pub policies: Vec<String>,
    #[serde(default)]
    #[cfg_attr(feature = "ts-export", ts(type = "Record<string, unknown>"))]
    pub context: serde_json::Value,
    #[serde(default)]
    pub trace_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(JsonSchema))]
#[cfg_attr(feature = "openapi", derive(ToSchema))]
#[cfg_attr(feature = "ts-export", derive(TS))]
#[cfg_attr(feature = "ts-export", ts(export))]
pub struct TriggeredPolicy {
    pub id: String,
    pub severity: Severity,
    pub reason: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(JsonSchema))]
#[cfg_attr(feature = "openapi", derive(ToSchema))]
#[cfg_attr(feature = "ts-export", derive(TS))]
#[cfg_attr(feature = "ts-export", ts(export))]
pub struct Decision {
    pub trace_id: String,
    pub verdict: Verdict,
    pub reason: String,
    pub triggered_policies: Vec<TriggeredPolicy>,
    pub safe_output: Option<String>,
    pub latency_ms: u64,
    /// Per-tier breakdown produced by the parallel-cancel orchestrator.
    /// Empty for callers that only ran the synchronous `Engine::check`
    /// path; populated when `Engine::check_async` is used.
    #[serde(default)]
    pub tier_results: Vec<TierResult>,
}

impl Decision {
    pub fn allow(trace_id: impl Into<String>) -> Self {
        Self {
            trace_id: trace_id.into(),
            verdict: Verdict::Allow,
            reason: "no policies triggered".into(),
            triggered_policies: vec![],
            safe_output: None,
            latency_ms: 0,
            tier_results: vec![],
        }
    }
}

/// Generate a fresh trace id. UUIDv7 is time-ordered so callers (and
/// the storage layer's daily-partitioned tables) get cheap chronological
/// scans without a separate sequence.
pub fn new_trace_id() -> String {
    Uuid::now_v7().to_string()
}

#[derive(Debug, thiserror::Error)]
pub enum TlError {
    #[error("policy compile error: {0}")]
    PolicyCompile(String),
    #[error("invalid request: {0}")]
    InvalidRequest(String),
    #[error("internal: {0}")]
    Internal(String),
}

/// Canonical error envelope returned on non-2xx responses from every
/// TrustLoopGuard endpoint. SDKs deserialize this body to produce typed
/// errors; integrators don't have to inspect status codes by hand.
///
/// The shape is intentionally minimal — `code` drives SDK-side fan-out,
/// `message` is for logs, `retriable` tells callers whether the same
/// request may be retried, and `details` is opaque so the server can
/// add validation field paths without breaking SDKs.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(JsonSchema))]
#[cfg_attr(feature = "openapi", derive(ToSchema))]
#[cfg_attr(feature = "ts-export", derive(TS))]
#[cfg_attr(feature = "ts-export", ts(export))]
pub struct ApiError {
    pub code: ApiErrorCode,
    pub message: String,
    /// Whether the caller may retry the same request without modification.
    /// SDKs honor `Retry-After` when present in addition to this flag.
    pub retriable: bool,
    /// Opaque structured details (e.g. validation field path).
    /// Defaults to `null`; servers may add fields without breaking SDKs.
    #[serde(default, skip_serializing_if = "serde_json::Value::is_null")]
    #[cfg_attr(feature = "ts-export", ts(type = "Record<string, unknown> | null"))]
    pub details: serde_json::Value,
}

impl std::fmt::Display for ApiError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?}: {}", self.code, self.message)
    }
}

/// Stable error code dictionary. Add variants here when introducing new
/// failure modes; never repurpose an existing variant — SDK callers may
/// be branching on it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[cfg_attr(feature = "schema", derive(JsonSchema))]
#[cfg_attr(feature = "openapi", derive(ToSchema))]
#[cfg_attr(feature = "ts-export", derive(TS))]
#[cfg_attr(feature = "ts-export", ts(export))]
pub enum ApiErrorCode {
    /// 400 — request malformed or failed validation.
    Invalid,
    /// 401 — missing or invalid credentials.
    Unauthorized,
    /// 403 — credentials valid but caller lacks permission.
    Forbidden,
    /// 404 — referenced resource not found.
    NotFound,
    /// 410 — API version retired; caller must upgrade.
    Gone,
    /// 422 — well-formed but semantically rejected.
    Unprocessable,
    /// 429 — rate limited. Honor `Retry-After` header when present.
    RateLimited,
    /// 500 — server-side bug; not retriable without server fix.
    Internal,
    /// 502 / 503 / 504 — transient infra issue; retriable with backoff.
    Unavailable,
}

impl ApiErrorCode {
    /// Map an HTTP status code to the canonical error code. Used by SDKs
    /// when the server returned a body that doesn't match `ApiError` —
    /// gives us a fallback that's still useful to integrators.
    pub fn from_http_status(status: u16) -> Self {
        match status {
            400 => Self::Invalid,
            401 => Self::Unauthorized,
            403 => Self::Forbidden,
            404 => Self::NotFound,
            410 => Self::Gone,
            422 => Self::Unprocessable,
            429 => Self::RateLimited,
            500..=501 => Self::Internal,
            502..=504 => Self::Unavailable,
            _ if (500..600).contains(&status) => Self::Internal,
            _ => Self::Invalid,
        }
    }

    /// Default retriable flag for this code. The server may override via
    /// `ApiError.retriable`; this is only used when synthesizing an
    /// `ApiError` from a raw HTTP status.
    pub fn default_retriable(self) -> bool {
        matches!(self, Self::RateLimited | Self::Unavailable)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn allow_helper_sets_verdict() {
        let d = Decision::allow("t-1");
        assert_eq!(d.verdict, Verdict::Allow);
        assert_eq!(d.trace_id, "t-1");
        assert!(d.tier_results.is_empty());
    }

    #[test]
    fn pre_v0_check_request_still_deserializes() {
        // Pre-PR-1 wire shape: no `domain` field. Must still parse so
        // existing SDKs and replay fixtures don't break.
        let json = r#"{
            "agent_id": "a",
            "channel": "chat",
            "input": "hi",
            "proposed_output": "hello"
        }"#;
        let req: CheckRequest = serde_json::from_str(json).unwrap();
        assert_eq!(req.agent_id, "a");
        assert!(req.domain.is_none());
        assert!(req.policies.is_empty());
    }

    #[test]
    fn api_error_round_trip() {
        let body = r#"{"code":"rate_limited","message":"too many requests","retriable":true}"#;
        let parsed: ApiError = serde_json::from_str(body).unwrap();
        assert_eq!(parsed.code, ApiErrorCode::RateLimited);
        assert!(parsed.retriable);
        assert!(parsed.details.is_null());
        let serialized = serde_json::to_string(&parsed).unwrap();
        assert!(serialized.contains("\"code\":\"rate_limited\""));
    }

    #[test]
    fn api_error_code_status_fallback() {
        assert_eq!(
            ApiErrorCode::from_http_status(429),
            ApiErrorCode::RateLimited
        );
        assert_eq!(
            ApiErrorCode::from_http_status(503),
            ApiErrorCode::Unavailable
        );
        assert_eq!(
            ApiErrorCode::from_http_status(401),
            ApiErrorCode::Unauthorized
        );
        assert_eq!(ApiErrorCode::from_http_status(599), ApiErrorCode::Internal);
        assert!(ApiErrorCode::RateLimited.default_retriable());
        assert!(ApiErrorCode::Unavailable.default_retriable());
        assert!(!ApiErrorCode::Invalid.default_retriable());
        assert!(!ApiErrorCode::Internal.default_retriable());
    }

    #[test]
    fn pre_v0_decision_still_deserializes() {
        let json = r#"{
            "trace_id": "t-1",
            "verdict": "allow",
            "reason": "ok",
            "triggered_policies": [],
            "safe_output": null,
            "latency_ms": 1
        }"#;
        let d: Decision = serde_json::from_str(json).unwrap();
        assert_eq!(d.verdict, Verdict::Allow);
        assert!(d.tier_results.is_empty());
    }

    #[test]
    fn policy_validate_response_is_core_wire_contract() {
        let response = PolicyValidateResponse {
            valid: false,
            policy_id: Some("refund-guarantee".into()),
            errors: vec![PolicyValidationIssue {
                path: "match.regex".into(),
                message: "regex failed to compile".into(),
            }],
        };

        let body = serde_json::to_value(&response).unwrap();
        assert_eq!(body["valid"], false);
        assert_eq!(body["policy_id"], "refund-guarantee");
        assert_eq!(body["errors"][0]["path"], "match.regex");
    }
}
