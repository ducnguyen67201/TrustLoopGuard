//! Information-flow checker: action-integrity and destination-permission.

use tl_core::{Confidentiality, GuardEvent, Verdict};

use super::{
    contributing_sources, derived_labels, finding, has_unknown_trust, has_untrusted, FindingSpec,
    EXTERNAL_SINKS, HIGH_IMPACT,
};
use crate::event_pipeline::{Checker, CheckerFinding};

pub const INFORMATION_FLOW_CHECKER_ID: &str = "information_flow";

const RULE_DESTINATION_PERMISSION: &str = "destination-permission";
const RULE_ACTION_INTEGRITY: &str = "action-integrity";
const RULE_MISSING_PROVENANCE: &str = "missing-provenance";

fn is_sensitive(confidentiality: Confidentiality) -> bool {
    matches!(
        confidentiality,
        Confidentiality::Private | Confidentiality::Secret | Confidentiality::Identity
    )
}

/// Deterministic information-flow control over resolved labels and
/// provenance:
///
/// - **destination-permission**: sensitive data may flow to external sinks
///   only when policy labels allow it (v1: never).
/// - **action-integrity**: high-impact actions must be controlled by
///   trusted context; untrusted control blocks, unverifiable control
///   escalates conservatively.
pub struct InformationFlowChecker;

impl Checker for InformationFlowChecker {
    fn id(&self) -> &'static str {
        INFORMATION_FLOW_CHECKER_ID
    }

    fn check(&self, event: &GuardEvent) -> Vec<CheckerFinding> {
        let side_effect = match event.action.side_effect {
            Some(side_effect) if HIGH_IMPACT.contains(&side_effect) => side_effect,
            _ => return vec![],
        };

        let contributing = contributing_sources(event);
        let mut findings = Vec::new();

        if EXTERNAL_SINKS.contains(&side_effect) {
            let sensitive: Vec<_> = contributing
                .sources
                .iter()
                .copied()
                .filter(|source| is_sensitive(source.labels.confidentiality))
                .collect();
            let derived_sensitive = derived_labels(event)
                .iter()
                .any(|labels| is_sensitive(labels.confidentiality));
            if !sensitive.is_empty() || derived_sensitive {
                findings.push(finding(FindingSpec {
                    checker_id: INFORMATION_FLOW_CHECKER_ID,
                    verdict: Verdict::Block,
                    rule: RULE_DESTINATION_PERMISSION,
                    reason: "sensitive data flows to an external sink".into(),
                    offending: &sensitive,
                    failure_mode: "data_exfiltration",
                    harm_class: "confidentiality",
                }));
            }
        }

        let untrusted_indices = has_untrusted(&contributing.sources);
        if !untrusted_indices.is_empty() {
            let untrusted: Vec<_> = untrusted_indices
                .iter()
                .map(|&index| contributing.sources[index])
                .collect();
            findings.push(finding(FindingSpec {
                checker_id: INFORMATION_FLOW_CHECKER_ID,
                verdict: Verdict::Block,
                rule: RULE_ACTION_INTEGRITY,
                reason: "high-impact action is controlled by untrusted context".into(),
                offending: &untrusted,
                failure_mode: "untrusted_control",
                harm_class: "integrity",
            }));
        }

        let unverifiable = event.provenance.is_empty()
            || contributing.has_unattributed_paths
            || !contributing.dangling.is_empty()
            || has_unknown_trust(&contributing.sources);
        if unverifiable {
            let mut spec_finding = finding(FindingSpec {
                checker_id: INFORMATION_FLOW_CHECKER_ID,
                verdict: Verdict::Escalate,
                rule: RULE_MISSING_PROVENANCE,
                reason: "high-impact action has unverifiable control provenance".into(),
                offending: &[],
                failure_mode: "unverified_control",
                harm_class: "integrity",
            });
            spec_finding.source_chain = contributing
                .dangling
                .iter()
                .map(|id| id.to_string())
                .collect();
            findings.push(spec_finding);
        }

        findings
    }
}

#[cfg(test)]
mod tests {
    use super::super::test_support::{event, source};
    use super::*;
    use tl_core::{
        EventKind, Integrity, LabelPolicyStatus, LabelResolution, Labels, Origin, ProvenanceMap,
        SideEffectClass, Trust,
    };

    fn trusted_public() -> Labels {
        Labels {
            trust: Trust::Trusted,
            confidentiality: Confidentiality::Public,
            integrity: Integrity::High,
        }
    }

    fn trusted_private() -> Labels {
        Labels {
            trust: Trust::Trusted,
            confidentiality: Confidentiality::Private,
            integrity: Integrity::High,
        }
    }

    fn untrusted_public() -> Labels {
        Labels {
            trust: Trust::Untrusted,
            confidentiality: Confidentiality::Public,
            integrity: Integrity::Low,
        }
    }

    fn provenance(path: &str, ids: &[&str]) -> ProvenanceMap {
        let mut map = ProvenanceMap::default();
        map.insert(path, ids.iter().map(|id| id.to_string()).collect());
        map
    }

    #[test]
    fn skips_low_impact_actions() {
        let checker = InformationFlowChecker;
        for side_effect in [
            None,
            Some(SideEffectClass::Read),
            Some(SideEffectClass::None),
        ] {
            let event = event(
                EventKind::ToolCallProposed,
                side_effect,
                vec![source("src.web", Origin::Web, untrusted_public())],
                provenance("recipient", &["src.web"]),
            );
            assert!(checker.check(&event).is_empty());
        }
    }

    #[test]
    fn blocks_private_source_flowing_to_external_sink() {
        let event = event(
            EventKind::ToolCallProposed,
            Some(SideEffectClass::ExternalCommunication),
            vec![source("src.crm", Origin::Api, trusted_private())],
            provenance("body", &["src.crm"]),
        );

        let findings = InformationFlowChecker.check(&event);
        assert_eq!(findings.len(), 1);
        let finding = &findings[0];
        assert_eq!(
            finding.violated_rule.as_deref(),
            Some("destination-permission")
        );
        assert_eq!(finding.verdict, Some(Verdict::Block));
        assert_eq!(finding.source_chain, vec!["src.crm"]);
        assert_eq!(finding.risk_source.as_deref(), Some("api"));
        assert_eq!(finding.failure_mode.as_deref(), Some("data_exfiltration"));
    }

    #[test]
    fn blocks_untrusted_controlled_high_impact_action() {
        let event = event(
            EventKind::DatabaseMutationProposed,
            Some(SideEffectClass::DbMutation),
            vec![source("src.web", Origin::Web, untrusted_public())],
            provenance("query", &["src.web"]),
        );

        let findings = InformationFlowChecker.check(&event);
        assert_eq!(findings.len(), 1);
        assert_eq!(
            findings[0].violated_rule.as_deref(),
            Some("action-integrity")
        );
        assert_eq!(findings[0].verdict, Some(Verdict::Block));
        assert_eq!(findings[0].risk_source.as_deref(), Some("web"));
    }

    #[test]
    fn allows_trusted_public_flow_to_external_sink() {
        let event = event(
            EventKind::ToolCallProposed,
            Some(SideEffectClass::ExternalCommunication),
            vec![source("src.user", Origin::User, trusted_public())],
            provenance("recipient", &["src.user"]),
        );

        assert!(InformationFlowChecker.check(&event).is_empty());
    }

    #[test]
    fn escalates_missing_provenance_on_high_impact_action() {
        let event = event(
            EventKind::ShellActionProposed,
            Some(SideEffectClass::ShellExec),
            vec![],
            ProvenanceMap::default(),
        );

        let findings = InformationFlowChecker.check(&event);
        assert_eq!(findings.len(), 1);
        assert_eq!(
            findings[0].violated_rule.as_deref(),
            Some("missing-provenance")
        );
        assert_eq!(findings[0].verdict, Some(Verdict::Escalate));
        assert_eq!(
            findings[0].failure_mode.as_deref(),
            Some("unverified_control")
        );
    }

    #[test]
    fn escalates_unattributed_provenance_paths() {
        // A provenance entry with an empty source-id list claims coverage
        // while attributing nothing; it must not read as clean.
        let event = event(
            EventKind::ToolCallProposed,
            Some(SideEffectClass::ExternalCommunication),
            vec![source("src.user", Origin::User, trusted_public())],
            provenance("recipient", &[]),
        );

        let findings = InformationFlowChecker.check(&event);
        assert_eq!(findings.len(), 1);
        assert_eq!(
            findings[0].violated_rule.as_deref(),
            Some("missing-provenance")
        );
        assert_eq!(findings[0].verdict, Some(Verdict::Escalate));
    }

    #[test]
    fn escalates_dangling_provenance_source_ids() {
        let event = event(
            EventKind::ToolCallProposed,
            Some(SideEffectClass::FileWrite),
            vec![source("src.user", Origin::User, trusted_public())],
            provenance("path", &["src.user", "src.ghost"]),
        );

        let findings = InformationFlowChecker.check(&event);
        assert_eq!(findings.len(), 1);
        assert_eq!(
            findings[0].violated_rule.as_deref(),
            Some("missing-provenance")
        );
        assert_eq!(findings[0].source_chain, vec!["src.ghost"]);
    }

    #[test]
    fn escalates_unknown_trust_control_on_high_impact_action() {
        let event = event(
            EventKind::ToolCallProposed,
            Some(SideEffectClass::ApiMutation),
            vec![source("src.mystery", Origin::Unknown, Labels::default())],
            provenance("target", &["src.mystery"]),
        );

        let findings = InformationFlowChecker.check(&event);
        assert_eq!(findings.len(), 1);
        assert_eq!(
            findings[0].violated_rule.as_deref(),
            Some("missing-provenance")
        );
        assert_eq!(findings[0].verdict, Some(Verdict::Escalate));
    }

    #[test]
    fn emits_both_rules_when_both_violated() {
        let mut labels = untrusted_public();
        labels.confidentiality = Confidentiality::Secret;
        let event = event(
            EventKind::ToolCallProposed,
            Some(SideEffectClass::ExternalCommunication),
            vec![source("src.web", Origin::Web, labels)],
            provenance("body", &["src.web"]),
        );

        let findings = InformationFlowChecker.check(&event);
        let rules: Vec<&str> = findings
            .iter()
            .filter_map(|finding| finding.violated_rule.as_deref())
            .collect();
        assert_eq!(rules, vec!["destination-permission", "action-integrity"]);
    }

    #[test]
    fn flags_sensitive_derived_labels_even_when_sources_look_clean() {
        let mut derived = std::collections::BTreeMap::new();
        derived.insert(
            "body".to_string(),
            Labels {
                trust: Trust::Trusted,
                confidentiality: Confidentiality::Private,
                integrity: Integrity::High,
            },
        );
        let mut guard_event = event(
            EventKind::ToolCallProposed,
            Some(SideEffectClass::ExternalCommunication),
            vec![source("src.user", Origin::User, trusted_public())],
            provenance("body", &["src.user"]),
        );
        guard_event.label_resolution = Some(LabelResolution {
            policy_status: LabelPolicyStatus::Applied,
            sources: vec![],
            derived,
        });

        let findings = InformationFlowChecker.check(&guard_event);
        assert_eq!(findings.len(), 1);
        assert_eq!(
            findings[0].violated_rule.as_deref(),
            Some("destination-permission")
        );
        // No single offending source — the derivation carries the evidence.
        assert!(findings[0].source_chain.is_empty());
    }
}
