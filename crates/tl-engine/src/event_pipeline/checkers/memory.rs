//! Memory write-time checker: v1 memory control.
//!
//! Blocks or escalates untrusted content becoming authority-bearing memory
//! at write time. Retrieval-time, cross-session memory analysis is
//! deliberately out of scope for v1.

use tl_core::{AuthorizationEffect, EventKind, GuardEvent, SideEffectClass};

use super::{contributing_sources, finding, has_unknown_trust, has_untrusted, FindingSpec};
use crate::event_pipeline::{Checker, CheckerFinding};

pub const MEMORY_CHECKER_ID: &str = "memory";

const RULE_MEMORY_WRITE_UNTRUSTED: &str = "memory-write-untrusted";
const RULE_MEMORY_WRITE_UNVERIFIED: &str = "memory-write-unverified";

pub struct MemoryChecker;

impl Checker for MemoryChecker {
    fn id(&self) -> &'static str {
        MEMORY_CHECKER_ID
    }

    fn check(&self, event: &GuardEvent) -> Vec<CheckerFinding> {
        let is_memory_write = event.kind == EventKind::MemoryWriteProposed
            || event.action.side_effect == Some(SideEffectClass::MemoryWrite);
        if !is_memory_write {
            return vec![];
        }

        let contributing = contributing_sources(event);
        let mut findings = Vec::new();

        let untrusted_indices = has_untrusted(&contributing.sources);
        if !untrusted_indices.is_empty() {
            let untrusted: Vec<_> = untrusted_indices
                .iter()
                .map(|&index| contributing.sources[index])
                .collect();
            findings.push(finding(FindingSpec {
                checker_id: MEMORY_CHECKER_ID,
                effect: AuthorizationEffect::Deny,
                rule: RULE_MEMORY_WRITE_UNTRUSTED,
                reason: "untrusted content would become authority-bearing memory".into(),
                offending: &untrusted,
                risk_code: "memory_poisoning",
                harm_class: "integrity",
            }));
        }

        let unverifiable = event.provenance.is_empty()
            || contributing.has_unattributed_paths
            || !contributing.dangling.is_empty()
            || has_unknown_trust(&contributing.sources);
        if unverifiable {
            let mut spec_finding = finding(FindingSpec {
                checker_id: MEMORY_CHECKER_ID,
                effect: AuthorizationEffect::Defer,
                rule: RULE_MEMORY_WRITE_UNVERIFIED,
                reason: "memory write has unverifiable content provenance".into(),
                offending: &[],
                risk_code: "unverified_memory_write",
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
    use tl_core::{Confidentiality, Integrity, Labels, Origin, ProvenanceMap, Trust};

    fn trusted() -> Labels {
        Labels {
            trust: Trust::Trusted,
            confidentiality: Confidentiality::Public,
            integrity: Integrity::High,
        }
    }

    fn untrusted() -> Labels {
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
    fn ignores_non_memory_events() {
        let event = event(
            EventKind::ToolCallProposed,
            Some(SideEffectClass::NetworkCall),
            vec![source("src.web", Origin::Web, untrusted())],
            provenance("url", &["src.web"]),
        );

        assert!(MemoryChecker.check(&event).is_empty());
    }

    #[test]
    fn applies_on_memory_write_side_effect_even_for_other_kinds() {
        let event = event(
            EventKind::ToolCallProposed,
            Some(SideEffectClass::MemoryWrite),
            vec![source("src.web", Origin::Web, untrusted())],
            provenance("note", &["src.web"]),
        );

        let findings = MemoryChecker.check(&event);
        assert_eq!(findings.len(), 1);
        assert_eq!(
            findings[0].violated_rule.as_deref(),
            Some("memory-write-untrusted")
        );
    }

    #[test]
    fn blocks_untrusted_memory_write() {
        let event = event(
            EventKind::MemoryWriteProposed,
            None,
            vec![
                source("src.web", Origin::Web, untrusted()),
                source("src.user", Origin::User, trusted()),
            ],
            provenance("note", &["src.web", "src.user"]),
        );

        let findings = MemoryChecker.check(&event);
        assert_eq!(findings.len(), 1);
        let finding = &findings[0];
        assert_eq!(finding.effect, Some(AuthorizationEffect::Deny));
        assert_eq!(finding.source_chain, vec!["src.web"]);
        assert_eq!(finding.risk_source.as_deref(), Some("web"));
        assert_eq!(finding.risk_code.as_deref(), Some("memory_poisoning"));
    }

    #[test]
    fn defers_unverified_memory_write() {
        let event = event(
            EventKind::MemoryWriteProposed,
            None,
            vec![],
            ProvenanceMap::default(),
        );

        let findings = MemoryChecker.check(&event);
        assert_eq!(findings.len(), 1);
        assert_eq!(
            findings[0].violated_rule.as_deref(),
            Some("memory-write-unverified")
        );
        assert_eq!(findings[0].effect, Some(AuthorizationEffect::Defer));
    }

    #[test]
    fn defers_unattributed_provenance_paths() {
        let event = event(
            EventKind::MemoryWriteProposed,
            None,
            vec![source("src.user", Origin::User, trusted())],
            provenance("note", &[]),
        );

        let findings = MemoryChecker.check(&event);
        assert_eq!(findings.len(), 1);
        assert_eq!(
            findings[0].violated_rule.as_deref(),
            Some("memory-write-unverified")
        );
    }

    #[test]
    fn trusted_provenance_passes_clean() {
        let event = event(
            EventKind::MemoryWriteProposed,
            None,
            vec![source("src.user", Origin::User, trusted())],
            provenance("note", &["src.user"]),
        );

        assert!(MemoryChecker.check(&event).is_empty());
    }

    #[test]
    fn untrusted_and_unknown_sources_emit_both_rules() {
        let event = event(
            EventKind::MemoryWriteProposed,
            None,
            vec![
                source("src.web", Origin::Web, untrusted()),
                source("src.mystery", Origin::Unknown, Labels::default()),
            ],
            provenance("note", &["src.web", "src.mystery"]),
        );

        let rules: Vec<String> = MemoryChecker
            .check(&event)
            .into_iter()
            .filter_map(|finding| finding.violated_rule)
            .collect();
        assert_eq!(
            rules,
            vec!["memory-write-untrusted", "memory-write-unverified"]
        );
    }
}
