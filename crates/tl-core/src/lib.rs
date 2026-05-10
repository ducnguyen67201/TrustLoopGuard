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
pub mod tier;

pub use agent::{AgentAuthority, AgentProfile, AgentScope, AgentTone, KnowledgeSource};
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
}
