//! Phase-4 deterministic checkers and their shared evidence helpers.
//!
//! Checkers evaluate the *resolved* event: labels come from the label
//! resolver, the side effect from registry resolution. They never resolve
//! anything themselves and never perform I/O.

mod approval;
mod information_flow;
mod memory;
mod param_auth;

pub use approval::{ApprovalChecker, APPROVAL_CHECKER_ID};
pub use information_flow::{InformationFlowChecker, INFORMATION_FLOW_CHECKER_ID};
pub use memory::{MemoryChecker, MEMORY_CHECKER_ID};
pub use param_auth::{ParameterAuthChecker, PARAMETER_AUTH_CHECKER_ID};

use tl_core::{GuardEvent, Labels, Origin, SideEffectClass, Source, Trust};

use super::CheckerFinding;

/// Side effects that send data outside the workspace boundary.
pub(crate) const EXTERNAL_SINKS: &[SideEffectClass] = &[
    SideEffectClass::ExternalCommunication,
    SideEffectClass::Publish,
    SideEffectClass::NetworkCall,
];

/// Side effects whose actions can cause real-world or persistent harm.
pub(crate) const HIGH_IMPACT: &[SideEffectClass] = &[
    SideEffectClass::ExternalCommunication,
    SideEffectClass::Publish,
    SideEffectClass::NetworkCall,
    SideEffectClass::FileWrite,
    SideEffectClass::ShellExec,
    SideEffectClass::DbMutation,
    SideEffectClass::ApiMutation,
];

/// The sources whose values flow into the action, per the provenance map.
///
/// `dangling` collects provenance source ids with no matching entry in
/// `event.sources`; their trust cannot be verified, so checkers treat them
/// as unverified control. `has_unattributed_paths` flags provenance
/// entries with an empty source-id list — a path that claims provenance
/// but attributes nothing proves nothing, so checkers treat it like
/// missing provenance rather than a clean evaluation.
pub(crate) struct ContributingSources<'a> {
    pub sources: Vec<&'a Source>,
    pub dangling: Vec<&'a str>,
    pub has_unattributed_paths: bool,
}

pub(crate) fn contributing_sources(event: &GuardEvent) -> ContributingSources<'_> {
    let mut sources: Vec<&Source> = Vec::new();
    let mut dangling: Vec<&str> = Vec::new();
    let mut has_unattributed_paths = false;
    for ids in event.provenance.0.values() {
        if ids.is_empty() {
            has_unattributed_paths = true;
        }
        for id in ids {
            match event.sources.iter().find(|source| &source.id == id) {
                Some(source) => {
                    if !sources.iter().any(|seen| seen.id == source.id) {
                        sources.push(source);
                    }
                }
                None => {
                    if !dangling.iter().any(|seen| seen == id) {
                        dangling.push(id);
                    }
                }
            }
        }
    }
    ContributingSources {
        sources,
        dangling,
        has_unattributed_paths,
    }
}

/// Per-path labels derived by provenance propagation, when the label
/// resolver ran.
pub(crate) fn derived_labels(event: &GuardEvent) -> Vec<&Labels> {
    event
        .label_resolution
        .as_ref()
        .map(|resolution| resolution.derived.values().collect())
        .unwrap_or_default()
}

pub(crate) fn origin_str(origin: Origin) -> &'static str {
    match origin {
        Origin::User => "user",
        Origin::System => "system",
        Origin::Tool => "tool",
        Origin::Memory => "memory",
        Origin::File => "file",
        Origin::Web => "web",
        Origin::Email => "email",
        Origin::Api => "api",
        Origin::Unknown => "unknown",
    }
}

/// One finding per violated rule, with the evidence fields the decision
/// composer copies onto an enforced decision.
pub(crate) struct FindingSpec<'a> {
    pub checker_id: &'static str,
    pub verdict: tl_core::Verdict,
    pub rule: &'static str,
    pub reason: String,
    pub offending: &'a [&'a Source],
    pub failure_mode: &'static str,
    pub harm_class: &'static str,
}

pub(crate) fn finding(spec: FindingSpec<'_>) -> CheckerFinding {
    CheckerFinding {
        checker_id: spec.checker_id.to_string(),
        verdict: Some(spec.verdict),
        reason: spec.reason,
        violated_rule: Some(spec.rule.to_string()),
        remediation: None,
        source_chain: spec
            .offending
            .iter()
            .map(|source| source.id.clone())
            .collect(),
        risk_source: spec
            .offending
            .first()
            .map(|source| origin_str(source.origin).to_string()),
        failure_mode: Some(spec.failure_mode.to_string()),
        harm_class: Some(spec.harm_class.to_string()),
    }
}

pub(crate) fn has_untrusted(sources: &[&Source]) -> Vec<usize> {
    sources
        .iter()
        .enumerate()
        .filter(|(_, source)| source.labels.trust == Trust::Untrusted)
        .map(|(index, _)| index)
        .collect()
}

pub(crate) fn has_unknown_trust(sources: &[&Source]) -> bool {
    sources
        .iter()
        .any(|source| source.labels.trust == Trust::Unknown)
}

#[cfg(test)]
pub(crate) mod test_support {
    use tl_core::{
        Action, EventKind, GuardEvent, Labels, Origin, Principal, ProvenanceMap, SideEffectClass,
        Source,
    };

    pub(crate) fn source(id: &str, origin: Origin, labels: Labels) -> Source {
        Source {
            id: id.into(),
            origin,
            labels,
            kind: None,
        }
    }

    pub(crate) fn event(
        kind: EventKind,
        side_effect: Option<SideEffectClass>,
        sources: Vec<Source>,
        provenance: ProvenanceMap,
    ) -> GuardEvent {
        GuardEvent {
            kind,
            principal: Principal {
                workspace_id: "ws_1".into(),
                environment_id: "production".into(),
                agent_id: "agent-1".into(),
                user_id: None,
                session_id: None,
                task_id: None,
                run_id: None,
                run_event_id: None,
            },
            action: Action {
                operation: "op".into(),
                parameters: serde_json::Value::Null,
                side_effect,
            },
            sources,
            provenance,
            resolution: None,
            label_resolution: None,
            checks: vec![],
            signals: vec![],
            context: serde_json::Value::Null,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::test_support::{event, source};
    use super::*;
    use tl_core::{EventKind, ProvenanceMap};

    #[test]
    fn contributing_sources_unions_paths_and_collects_dangling_ids() {
        let mut provenance = ProvenanceMap::default();
        provenance.insert("recipient", vec!["src.web".into(), "src.user".into()]);
        provenance.insert("body", vec!["src.web".into(), "src.ghost".into()]);
        let event = event(
            EventKind::ToolCallProposed,
            None,
            vec![
                source("src.web", Origin::Web, Labels::default()),
                source("src.user", Origin::User, Labels::default()),
            ],
            provenance,
        );

        let contributing = contributing_sources(&event);
        let ids: Vec<&str> = contributing
            .sources
            .iter()
            .map(|source| source.id.as_str())
            .collect();
        assert_eq!(ids, vec!["src.web", "src.user"]);
        assert_eq!(contributing.dangling, vec!["src.ghost"]);
        assert!(!contributing.has_unattributed_paths);
    }

    #[test]
    fn contributing_sources_flags_paths_with_no_source_ids() {
        let mut provenance = ProvenanceMap::default();
        provenance.insert("recipient", vec![]);
        let event = event(
            EventKind::ToolCallProposed,
            None,
            vec![source("src.user", Origin::User, Labels::default())],
            provenance,
        );

        let contributing = contributing_sources(&event);
        assert!(contributing.sources.is_empty());
        assert!(contributing.dangling.is_empty());
        assert!(contributing.has_unattributed_paths);
    }

    #[test]
    fn contributing_sources_is_empty_for_empty_provenance() {
        let event = event(
            EventKind::ToolCallProposed,
            None,
            vec![source("src.user", Origin::User, Labels::default())],
            ProvenanceMap::default(),
        );

        let contributing = contributing_sources(&event);
        assert!(contributing.sources.is_empty());
        assert!(contributing.dangling.is_empty());
    }
}
