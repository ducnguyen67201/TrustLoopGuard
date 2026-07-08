use serde::{Deserialize, Serialize};

use crate::{label::Origin, CreateRunEventRequest, TierResult};

#[cfg(feature = "schema")]
use schemars::JsonSchema;
#[cfg(feature = "ts-export")]
use ts_rs::TS;
#[cfg(feature = "openapi")]
use utoipa::ToSchema;

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

impl Verdict {
    /// Severity rank for worst-verdict-wins composition:
    /// `Block > Escalate > Rewrite > Allow`.
    fn severity_rank(self) -> u8 {
        match self {
            Verdict::Allow => 0,
            Verdict::Rewrite => 1,
            Verdict::Escalate => 2,
            Verdict::Block => 3,
        }
    }

    /// The more severe of the two verdicts. Never downgrades.
    pub fn worst_with(self, other: Verdict) -> Verdict {
        if other.severity_rank() > self.severity_rank() {
            other
        } else {
            self
        }
    }
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
pub struct CheckRequest {
    #[serde(default)]
    pub workspace_id: Option<String>,
    #[serde(default)]
    pub run_id: Option<String>,
    #[serde(default)]
    pub run_event_id: Option<String>,
    #[serde(default)]
    pub run_event: Option<CreateRunEventRequest>,
    /// Caller-supplied monitoring session id. Opaque to the server;
    /// promoted to a trace column for session-scoped trace queries.
    /// Skipped when absent so untagged requests keep their pre-session
    /// wire shape byte-identical.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub session_id: Option<String>,
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
    pub context: serde_json::Value,
    #[serde(default)]
    pub trace_id: Option<String>,
    #[serde(default)]
    pub redaction: Option<RedactionInfo>,
}

impl Default for CheckRequest {
    fn default() -> Self {
        Self {
            workspace_id: None,
            run_id: None,
            run_event_id: None,
            run_event: None,
            session_id: None,
            agent_id: String::new(),
            channel: Channel::Chat,
            input: String::new(),
            proposed_output: String::new(),
            domain: None,
            policies: Vec::new(),
            context: serde_json::Value::Null,
            trace_id: None,
            redaction: None,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[cfg_attr(feature = "schema", derive(JsonSchema))]
#[cfg_attr(feature = "openapi", derive(ToSchema))]
#[cfg_attr(feature = "ts-export", derive(TS))]
#[cfg_attr(feature = "ts-export", ts(export))]
pub enum RedactionMode {
    SdkLocal,
    CustomerService,
    Server,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[cfg_attr(feature = "schema", derive(JsonSchema))]
#[cfg_attr(feature = "openapi", derive(ToSchema))]
#[cfg_attr(feature = "ts-export", derive(TS))]
#[cfg_attr(feature = "ts-export", ts(export))]
pub enum RedactionStatus {
    NotRequested,
    Applied,
    Failed,
    RejectedRawSensitiveData,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(JsonSchema))]
#[cfg_attr(feature = "openapi", derive(ToSchema))]
#[cfg_attr(feature = "ts-export", derive(TS))]
#[cfg_attr(feature = "ts-export", ts(export))]
pub struct RedactedEntity {
    pub entity_type: String,
    pub token: String,
    pub count: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(JsonSchema))]
#[cfg_attr(feature = "openapi", derive(ToSchema))]
#[cfg_attr(feature = "ts-export", derive(TS))]
#[cfg_attr(feature = "ts-export", ts(export))]
pub struct RedactionInfo {
    pub mode: RedactionMode,
    pub status: RedactionStatus,
    pub entities: Vec<RedactedEntity>,
    pub input_redacted: bool,
    pub proposed_output_redacted: bool,
    pub context_redacted: bool,
}

impl RedactionInfo {
    /// Reject states that the wire shape allows but the contract forbids.
    /// Only `Applied` may carry redacted entities or claim that a field was
    /// touched; any other status describing entity output is a misreport.
    pub fn validate(&self) -> Result<(), &'static str> {
        let claims_effect = !self.entities.is_empty()
            || self.input_redacted
            || self.proposed_output_redacted
            || self.context_redacted;
        match self.status {
            RedactionStatus::Applied => Ok(()),
            RedactionStatus::NotRequested
            | RedactionStatus::Failed
            | RedactionStatus::RejectedRawSensitiveData
                if claims_effect =>
            {
                Err("redaction status does not permit entities or redacted fields")
            }
            _ => Ok(()),
        }
    }
}

/// Normalized repair category for action/tool-call feedback.
///
/// This is advisory control-plane guidance for the caller. It never changes
/// the verdict by itself; the checker/policy decision has already happened.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[cfg_attr(feature = "schema", derive(JsonSchema))]
#[cfg_attr(feature = "openapi", derive(ToSchema))]
#[cfg_attr(feature = "ts-export", derive(TS))]
#[cfg_attr(feature = "ts-export", ts(export))]
pub enum ActionFeedbackKind {
    ApprovalRequired,
    TrustedSourceRequired,
    ProvenanceRequired,
    ToolMetadataRequired,
    ValueLimitExceeded,
}

/// Programmatic next-step guidance for a rejected or escalated action.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(JsonSchema))]
#[cfg_attr(feature = "openapi", derive(ToSchema))]
#[cfg_attr(feature = "ts-export", derive(TS))]
#[cfg_attr(feature = "ts-export", ts(export))]
pub struct ActionFeedback {
    pub kind: ActionFeedbackKind,
    pub message: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts-export", ts(optional))]
    pub tool: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts-export", ts(optional))]
    pub parameter: Option<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    #[cfg_attr(feature = "ts-export", ts(as = "Option<Vec<Origin>>", optional))]
    pub required_origins: Vec<Origin>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    #[cfg_attr(feature = "ts-export", ts(as = "Option<Vec<String>>", optional))]
    pub approver_roles: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    #[cfg_attr(feature = "ts-export", ts(as = "Option<Vec<String>>", optional))]
    pub source_chain: Vec<String>,
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
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts-export", ts(optional))]
    pub checked_input_excerpt: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts-export", ts(optional))]
    pub checked_output_excerpt: Option<String>,
    pub latency_ms: u64,
    /// Per-tier breakdown produced by the parallel-cancel orchestrator.
    /// Empty for callers that only ran the synchronous `Engine::check`
    /// path; populated when `Engine::check_async` is used.
    #[serde(default)]
    pub tier_results: Vec<TierResult>,
    #[serde(default)]
    pub redaction: Option<RedactionInfo>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts-export", ts(optional))]
    pub violated_rule: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts-export", ts(optional))]
    pub remediation: Option<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    #[cfg_attr(
        feature = "ts-export",
        ts(as = "Option<Vec<ActionFeedback>>", optional)
    )]
    pub action_feedback: Vec<ActionFeedback>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts-export", ts(optional))]
    pub source_chain: Option<Vec<String>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts-export", ts(optional))]
    pub risk_source: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts-export", ts(optional))]
    pub failure_mode: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts-export", ts(optional))]
    pub harm_class: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts-export", ts(optional))]
    #[cfg_attr(feature = "ts-export", ts(type = "Record<string, unknown> | null"))]
    pub constraints: Option<serde_json::Value>,
}

impl Decision {
    pub fn allow(trace_id: impl Into<String>) -> Self {
        Self {
            trace_id: trace_id.into(),
            verdict: Verdict::Allow,
            reason: "no policies triggered".into(),
            triggered_policies: vec![],
            safe_output: None,
            checked_input_excerpt: None,
            checked_output_excerpt: None,
            latency_ms: 0,
            tier_results: vec![],
            redaction: None,
            violated_rule: None,
            remediation: None,
            action_feedback: vec![],
            source_chain: None,
            risk_source: None,
            failure_mode: None,
            harm_class: None,
            constraints: None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn check_request_without_session_id_still_deserializes() {
        let json = r#"{
            "agent_id": "agent-1",
            "channel": "chat",
            "input": "hi",
            "proposed_output": "hello"
        }"#;
        let req: CheckRequest = serde_json::from_str(json).expect("legacy JSON deserializes");
        assert_eq!(req.session_id, None);
    }

    #[test]
    fn check_request_omits_absent_session_id_on_serialize() {
        let req = CheckRequest {
            agent_id: "agent-1".into(),
            ..CheckRequest::default()
        };
        let value = serde_json::to_value(&req).expect("serialize");
        assert!(
            value.get("session_id").is_none(),
            "session_id must be absent (not null) when None, so untagged \
             requests stay byte-identical to the pre-session wire shape"
        );

        let tagged = CheckRequest {
            session_id: Some("sess_x".into()),
            ..req
        };
        let value = serde_json::to_value(&tagged).expect("serialize");
        assert_eq!(value["session_id"], "sess_x");
    }
}
