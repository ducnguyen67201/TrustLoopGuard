//! Checker enforcement rollout types.
//!
//! Checkers roll out per workspace through an explicit
//! `off -> shadow -> enforce` progression. The mode is workspace
//! configuration data: the same checker code runs in every mode, and only
//! the decision effect changes. Checker evaluation evidence persists on the
//! event (and therefore in traces) so shadow mode shows exactly what
//! enforce mode would have decided.

use serde::{Deserialize, Serialize};

use crate::{AuthorizationEffect, Severity};

#[cfg(feature = "schema")]
use schemars::JsonSchema;
#[cfg(feature = "ts-export")]
use ts_rs::TS;
#[cfg(feature = "openapi")]
use utoipa::ToSchema;

/// Rollout mode for one checker in one workspace.
///
/// - `Off`: the checker is not evaluated; no evidence is recorded.
/// - `Shadow`: the checker is evaluated and full hypothetical evidence is
///   recorded, but the decision is never changed.
/// - `Enforce`: the checker is evaluated and its findings can change the
///   decision (the strongest authorization effect wins).
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[cfg_attr(feature = "schema", derive(JsonSchema))]
#[cfg_attr(feature = "openapi", derive(ToSchema))]
#[cfg_attr(feature = "ts-export", derive(TS))]
#[cfg_attr(feature = "ts-export", ts(export))]
pub enum EnforcementMode {
    #[default]
    Off,
    Shadow,
    Enforce,
}

/// Evidence for one checker evaluation, attached by the event pipeline.
///
/// Recorded whenever a checker runs (shadow or enforce). A checker in
/// `off` mode records nothing.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(JsonSchema))]
#[cfg_attr(feature = "openapi", derive(ToSchema))]
#[cfg_attr(feature = "ts-export", derive(TS))]
#[cfg_attr(feature = "ts-export", ts(export))]
pub struct CheckerRun {
    pub checker_id: String,
    /// Mode the checker ran under at evaluation time.
    pub mode: EnforcementMode,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    #[cfg_attr(
        feature = "ts-export",
        ts(as = "Option<Vec<CheckerFindingEvidence>>", optional)
    )]
    pub findings: Vec<CheckerFindingEvidence>,
}

/// One rule violation observed by a checker.
///
/// `recommended_effect` is what enforce mode applies; in shadow mode it is
/// the hypothetical outcome and the decision is left untouched. It is
/// absent for effect-free evidence findings (e.g. "tool not registered,
/// parameters unverifiable") that never affect the decision in any mode.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(JsonSchema))]
#[cfg_attr(feature = "openapi", derive(ToSchema))]
#[cfg_attr(feature = "ts-export", derive(TS))]
#[cfg_attr(feature = "ts-export", ts(export))]
pub struct CheckerFindingEvidence {
    pub rule: String,
    pub reason: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts-export", ts(optional))]
    pub recommended_effect: Option<AuthorizationEffect>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    #[cfg_attr(feature = "ts-export", ts(as = "Option<Vec<String>>", optional))]
    pub source_chain: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts-export", ts(optional))]
    pub risk_source: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts-export", ts(optional))]
    pub risk_code: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts-export", ts(optional))]
    pub harm_class: Option<String>,
}

/// Advisory evidence from one LLM/classifier signal provider, attached
/// by the event pipeline.
///
/// Signals never decide authorization effects: the composer ignores them for
/// effect composition, and this evidence exists so traces and review
/// show what advisory layers observed.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(JsonSchema))]
#[cfg_attr(feature = "openapi", derive(ToSchema))]
#[cfg_attr(feature = "ts-export", derive(TS))]
#[cfg_attr(feature = "ts-export", ts(export))]
pub struct SignalEvidence {
    pub provider_id: String,
    pub message: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts-export", ts(optional))]
    pub severity: Option<Severity>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn enforcement_mode_serializes_snake_case_and_defaults_off() {
        assert_eq!(EnforcementMode::default(), EnforcementMode::Off);
        assert_eq!(
            serde_json::to_string(&EnforcementMode::Off).unwrap(),
            r#""off""#
        );
        assert_eq!(
            serde_json::to_string(&EnforcementMode::Shadow).unwrap(),
            r#""shadow""#
        );
        assert_eq!(
            serde_json::to_string(&EnforcementMode::Enforce).unwrap(),
            r#""enforce""#
        );
        let parsed: EnforcementMode = serde_json::from_str(r#""shadow""#).unwrap();
        assert_eq!(parsed, EnforcementMode::Shadow);
    }

    #[test]
    fn checker_run_omits_empty_findings_and_optional_fields() {
        let run = CheckerRun {
            checker_id: "information_flow".into(),
            mode: EnforcementMode::Shadow,
            findings: vec![],
        };
        let serialized = serde_json::to_string(&run).unwrap();
        assert_eq!(
            serialized,
            r#"{"checker_id":"information_flow","mode":"shadow"}"#
        );
    }

    #[test]
    fn signal_evidence_round_trips_and_omits_missing_severity() {
        let signal = SignalEvidence {
            provider_id: "llm".into(),
            message: "looks dangerous".into(),
            severity: None,
        };
        let value = serde_json::to_value(&signal).unwrap();
        assert!(value.get("severity").is_none());
        let parsed: SignalEvidence = serde_json::from_value(value).unwrap();
        assert_eq!(parsed, signal);

        let with_severity = SignalEvidence {
            severity: Some(Severity::Critical),
            ..signal
        };
        let value = serde_json::to_value(&with_severity).unwrap();
        assert_eq!(value["severity"], "critical");
    }

    #[test]
    fn checker_finding_evidence_round_trips() {
        let finding = CheckerFindingEvidence {
            rule: "destination-permission".into(),
            reason: "private data flows to external sink".into(),
            recommended_effect: Some(AuthorizationEffect::Deny),
            source_chain: vec!["src.web".into()],
            risk_source: Some("web".into()),
            risk_code: Some("data_exfiltration".into()),
            harm_class: None,
        };
        let value = serde_json::to_value(&finding).unwrap();
        assert_eq!(value["recommended_effect"], "deny");
        assert_eq!(value["source_chain"][0], "src.web");
        assert!(value.get("harm_class").is_none());
        let parsed: CheckerFindingEvidence = serde_json::from_value(value).unwrap();
        assert_eq!(parsed, finding);
    }
}
