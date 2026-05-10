//! Core types for TrustLoopGuard. Stable across all other crates.

use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Channel an agent is operating on. Drives latency budget and matcher selection.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Channel {
    Voice,
    Chat,
    Email,
    Other(String),
}

/// What TrustLoopGuard tells the caller to do with the proposed output.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Verdict {
    Allow,
    Block,
    Rewrite,
    Escalate,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Severity {
    Low,
    Medium,
    High,
    Critical,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CheckRequest {
    pub agent_id: String,
    pub channel: Channel,
    pub input: String,
    pub proposed_output: String,
    #[serde(default)]
    pub policies: Vec<String>,
    #[serde(default)]
    pub context: serde_json::Value,
    #[serde(default)]
    pub trace_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TriggeredPolicy {
    pub id: String,
    pub severity: Severity,
    pub reason: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Decision {
    pub trace_id: String,
    pub verdict: Verdict,
    pub reason: String,
    pub triggered_policies: Vec<TriggeredPolicy>,
    pub safe_output: Option<String>,
    pub latency_ms: u64,
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
        }
    }
}

pub fn new_trace_id() -> String {
    Uuid::new_v4().to_string()
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
    }
}
