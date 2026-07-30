//! Label resolution and provenance propagation (observe-only).
//!
//! Deterministic, in-process logic: built-in origin defaults, workspace
//! policy overrides read through a cached provider seam, and label
//! propagation over `ProvenanceMap`. Evidence only — no checker changes
//! an authorization effect because of labels.

use std::collections::BTreeMap;
use std::sync::Arc;

use async_trait::async_trait;
use tl_core::{
    Confidentiality, GuardEvent, Integrity, LabelBasis, LabelBasisSet, LabelPolicyStatus,
    LabelResolution, Labels, Origin, Source, SourceLabelEvidence, SourceLabelPolicy, Trust,
};

use super::{LabelResolver, ProvenanceResolver};

/// Marker error: the workspace label policy store could not be
/// consulted. Implementations log the details; resolution fails open —
/// built-in defaults apply and the evidence records
/// `policy_status: unavailable`, never a blocked decision.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LabelPolicyUnavailable;

/// Runtime lookup seam for workspace label policies. Async so
/// implementations can read through a cache with a storage fallback.
/// Returns enabled policies only; an empty vec means no overrides are
/// configured for the workspace.
#[async_trait]
pub trait LabelPolicyProvider: Send + Sync {
    async fn get(
        &self,
        workspace_id: &str,
    ) -> Result<Vec<SourceLabelPolicy>, LabelPolicyUnavailable>;
}

pub struct NoOpLabelPolicyProvider;

#[async_trait]
impl LabelPolicyProvider for NoOpLabelPolicyProvider {
    async fn get(
        &self,
        _workspace_id: &str,
    ) -> Result<Vec<SourceLabelPolicy>, LabelPolicyUnavailable> {
        Ok(vec![])
    }
}

/// Built-in label defaults per origin. Exhaustive on purpose: adding an
/// `Origin` variant must break compilation here, not silently default.
///
/// User and system channels are the only trusted ones; everything that
/// enters from outside the operator's control (tool output, memory,
/// files, web, email, external APIs) is untrusted with low integrity.
/// Unknown origin resolves to untrusted/unknown — conservative evidence
/// for later enforcement phases.
pub fn origin_default_labels(origin: Origin) -> Labels {
    let (trust, confidentiality, integrity) = match origin {
        Origin::User => (Trust::Trusted, Confidentiality::Private, Integrity::High),
        Origin::System => (Trust::Trusted, Confidentiality::Private, Integrity::High),
        Origin::Tool => (Trust::Untrusted, Confidentiality::Unknown, Integrity::Low),
        Origin::Memory => (Trust::Untrusted, Confidentiality::Private, Integrity::Low),
        Origin::File => (Trust::Untrusted, Confidentiality::Private, Integrity::Low),
        Origin::Web => (Trust::Untrusted, Confidentiality::Public, Integrity::Low),
        Origin::Email => (Trust::Untrusted, Confidentiality::Private, Integrity::Low),
        Origin::Api => (Trust::Untrusted, Confidentiality::Private, Integrity::Low),
        Origin::Unknown => (
            Trust::Untrusted,
            Confidentiality::Unknown,
            Integrity::Unknown,
        ),
    };
    Labels {
        trust,
        confidentiality,
        integrity,
    }
}

/// Resolve one source's labels. Runtime callers cannot promote labels on
/// externally controlled origins: those values come from an enabled workspace
/// override (`Some`) or the conservative built-in origin default. User/system
/// sources retain the existing producer-declared precedence because those
/// origins are the trusted input channels.
pub fn resolve_source_labels(
    source: &Source,
    policies: &[SourceLabelPolicy],
) -> (Labels, LabelBasisSet) {
    let defaults = origin_default_labels(source.origin);
    let policy = policies.iter().find(|p| p.origin == source.origin);
    let accepts_declared = matches!(source.origin, Origin::User | Origin::System);

    let (trust, trust_basis) = match (source.labels.trust, policy.and_then(|p| p.trust)) {
        (declared, _) if accepts_declared && declared != Trust::Unknown => {
            (declared, LabelBasis::Declared)
        }
        (_, Some(overridden)) => (overridden, LabelBasis::WorkspaceOverride),
        _ => (defaults.trust, LabelBasis::OriginDefault),
    };
    let (confidentiality, confidentiality_basis) = match (
        source.labels.confidentiality,
        policy.and_then(|p| p.confidentiality),
    ) {
        (declared, _) if accepts_declared && declared != Confidentiality::Unknown => {
            (declared, LabelBasis::Declared)
        }
        (_, Some(overridden)) => (overridden, LabelBasis::WorkspaceOverride),
        _ => (defaults.confidentiality, LabelBasis::OriginDefault),
    };
    let (integrity, integrity_basis) =
        match (source.labels.integrity, policy.and_then(|p| p.integrity)) {
            (declared, _) if accepts_declared && declared != Integrity::Unknown => {
                (declared, LabelBasis::Declared)
            }
            (_, Some(overridden)) => (overridden, LabelBasis::WorkspaceOverride),
            _ => (defaults.integrity, LabelBasis::OriginDefault),
        };

    (
        Labels {
            trust,
            confidentiality,
            integrity,
        },
        LabelBasisSet {
            trust: trust_basis,
            confidentiality: confidentiality_basis,
            integrity: integrity_basis,
        },
    )
}

/// Deterministic fold of contributing labels for one provenance path.
/// Empty contributors yield all-`Unknown` — missing provenance is
/// unknown, never clean.
///
/// - trust: any untrusted wins; else any unknown wins; else trusted.
/// - confidentiality: highest sensitivity wins. `Unknown` outranks
///   `Public` (a path touched by an unknown source can never be claimed
///   public) but known-sensitive claims dominate.
/// - integrity: weakest contributor wins; any unknown poisons the path
///   to unknown.
pub fn combine_labels(contributors: &[Labels]) -> Labels {
    if contributors.is_empty() {
        return Labels::default();
    }

    let trust = if contributors.iter().any(|l| l.trust == Trust::Untrusted) {
        Trust::Untrusted
    } else if contributors.iter().any(|l| l.trust == Trust::Unknown) {
        Trust::Unknown
    } else {
        Trust::Trusted
    };

    let confidentiality = contributors
        .iter()
        .map(|l| l.confidentiality)
        .max_by_key(|c| confidentiality_rank(*c))
        .expect("non-empty contributors");

    let integrity = contributors
        .iter()
        .map(|l| l.integrity)
        .min_by_key(|i| integrity_rank(*i))
        .expect("non-empty contributors");

    Labels {
        trust,
        confidentiality,
        integrity,
    }
}

fn confidentiality_rank(c: Confidentiality) -> u8 {
    match c {
        Confidentiality::Public => 0,
        Confidentiality::Unknown => 1,
        Confidentiality::Private => 2,
        Confidentiality::Secret => 3,
        Confidentiality::Identity => 4,
    }
}

fn integrity_rank(i: Integrity) -> u8 {
    match i {
        Integrity::Unknown => 0,
        Integrity::Low => 1,
        Integrity::Medium => 2,
        Integrity::High => 3,
    }
}

/// Live label-resolution stage: fetches the workspace's enabled label
/// policies through the cached provider seam, resolves every source's
/// labels in place (the resolved value is authoritative for later
/// stages, mirroring the registry side-effect overwrite), and attaches
/// per-source basis evidence.
pub struct PolicyLabelResolver {
    policy: Arc<dyn LabelPolicyProvider>,
}

impl PolicyLabelResolver {
    pub fn new(policy: Arc<dyn LabelPolicyProvider>) -> Self {
        Self { policy }
    }
}

#[async_trait]
impl LabelResolver for PolicyLabelResolver {
    async fn resolve(&self, event: &mut GuardEvent) {
        let (policies, policy_status) = match self.policy.get(&event.principal.workspace_id).await {
            Ok(policies) if policies.is_empty() => (policies, LabelPolicyStatus::NotConfigured),
            Ok(policies) => (policies, LabelPolicyStatus::Applied),
            Err(LabelPolicyUnavailable) => (vec![], LabelPolicyStatus::Unavailable),
        };

        let mut sources = Vec::with_capacity(event.sources.len());
        for source in &mut event.sources {
            let (labels, basis) = resolve_source_labels(source, &policies);
            source.labels = labels;
            sources.push(SourceLabelEvidence {
                source_id: source.id.clone(),
                labels,
                basis,
            });
        }

        event.label_resolution = Some(LabelResolution {
            policy_status,
            sources,
            derived: BTreeMap::new(),
        });
    }
}

/// Live propagation stage: pure and deterministic over `ProvenanceMap`.
/// For each parameter path, the derived labels are the fold of the
/// resolved labels of every referenced source; a source id with no
/// matching event source contributes all-`Unknown`. Events without
/// provenance get no derived entries — evidence is never invented.
///
/// Pair this stage with a live `LabelResolver`: it folds whatever labels
/// the sources carry at this point in the chain. Run standalone it folds
/// producer-declared values, and the `LabelResolution` container it
/// creates reports `policy_status: not_configured` even though label
/// resolution never ran.
pub struct ProvenancePropagator;

impl ProvenanceResolver for ProvenancePropagator {
    fn resolve(&self, event: &mut GuardEvent) {
        if event.provenance.is_empty() {
            return;
        }

        let by_id: BTreeMap<&str, Labels> = event
            .sources
            .iter()
            .map(|s| (s.id.as_str(), s.labels))
            .collect();

        let mut derived = BTreeMap::new();
        for (path, source_ids) in &event.provenance.0 {
            let contributors: Vec<Labels> = source_ids
                .iter()
                .map(|id| by_id.get(id.as_str()).copied().unwrap_or_default())
                .collect();
            derived.insert(path.clone(), combine_labels(&contributors));
        }

        event
            .label_resolution
            .get_or_insert_with(|| LabelResolution {
                policy_status: LabelPolicyStatus::NotConfigured,
                sources: vec![],
                derived: BTreeMap::new(),
            })
            .derived = derived;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn source(id: &str, origin: Origin, labels: Labels) -> Source {
        Source {
            id: id.into(),
            origin,
            labels,
            kind: None,
        }
    }

    fn web_policy(
        trust: Option<Trust>,
        confidentiality: Option<Confidentiality>,
        integrity: Option<Integrity>,
    ) -> SourceLabelPolicy {
        SourceLabelPolicy {
            origin: Origin::Web,
            trust,
            confidentiality,
            integrity,
        }
    }

    fn labels(trust: Trust, confidentiality: Confidentiality, integrity: Integrity) -> Labels {
        Labels {
            trust,
            confidentiality,
            integrity,
        }
    }

    #[test]
    fn origin_defaults_are_deterministic() {
        let cases = [
            (
                Origin::User,
                Trust::Trusted,
                Confidentiality::Private,
                Integrity::High,
            ),
            (
                Origin::System,
                Trust::Trusted,
                Confidentiality::Private,
                Integrity::High,
            ),
            (
                Origin::Tool,
                Trust::Untrusted,
                Confidentiality::Unknown,
                Integrity::Low,
            ),
            (
                Origin::Memory,
                Trust::Untrusted,
                Confidentiality::Private,
                Integrity::Low,
            ),
            (
                Origin::File,
                Trust::Untrusted,
                Confidentiality::Private,
                Integrity::Low,
            ),
            (
                Origin::Web,
                Trust::Untrusted,
                Confidentiality::Public,
                Integrity::Low,
            ),
            (
                Origin::Email,
                Trust::Untrusted,
                Confidentiality::Private,
                Integrity::Low,
            ),
            (
                Origin::Api,
                Trust::Untrusted,
                Confidentiality::Private,
                Integrity::Low,
            ),
        ];
        for (origin, trust, confidentiality, integrity) in cases {
            assert_eq!(
                origin_default_labels(origin),
                labels(trust, confidentiality, integrity),
                "{origin:?}"
            );
        }
    }

    #[test]
    fn unknown_origin_defaults_to_untrusted_unknown() {
        assert_eq!(
            origin_default_labels(Origin::Unknown),
            labels(
                Trust::Untrusted,
                Confidentiality::Unknown,
                Integrity::Unknown
            )
        );
    }

    #[test]
    fn workspace_override_wins_over_default() {
        let policies = [web_policy(Some(Trust::Trusted), None, None)];
        let src = source("src.web", Origin::Web, Labels::default());

        let (resolved, basis) = resolve_source_labels(&src, &policies);

        assert_eq!(resolved.trust, Trust::Trusted);
        assert_eq!(basis.trust, LabelBasis::WorkspaceOverride);
        // Untouched families keep origin defaults.
        assert_eq!(resolved.confidentiality, Confidentiality::Public);
        assert_eq!(basis.confidentiality, LabelBasis::OriginDefault);
        assert_eq!(resolved.integrity, Integrity::Low);
        assert_eq!(basis.integrity, LabelBasis::OriginDefault);
    }

    #[test]
    fn externally_controlled_origin_cannot_promote_declared_labels() {
        let src = source(
            "src.web",
            Origin::Web,
            labels(Trust::Trusted, Confidentiality::Public, Integrity::High),
        );

        let (resolved, basis) = resolve_source_labels(&src, &[]);

        assert_eq!(resolved, origin_default_labels(Origin::Web));
        assert_eq!(basis.trust, LabelBasis::OriginDefault);
        assert_eq!(basis.integrity, LabelBasis::OriginDefault);
    }

    #[test]
    fn trusted_channel_declared_label_keeps_precedence() {
        let src = source(
            "src.user",
            Origin::User,
            labels(Trust::Trusted, Confidentiality::Public, Integrity::High),
        );

        let (resolved, basis) = resolve_source_labels(&src, &[]);

        assert_eq!(resolved.confidentiality, Confidentiality::Public);
        assert_eq!(basis.confidentiality, LabelBasis::Declared);
    }

    #[test]
    fn declared_unknown_means_undeclared() {
        let src = source("src.web", Origin::Web, Labels::default());

        let (resolved, basis) = resolve_source_labels(&src, &[]);

        assert_eq!(resolved, origin_default_labels(Origin::Web));
        assert_eq!(basis.trust, LabelBasis::OriginDefault);
        assert_eq!(basis.confidentiality, LabelBasis::OriginDefault);
        assert_eq!(basis.integrity, LabelBasis::OriginDefault);
    }

    #[test]
    fn partial_override_only_touches_set_families() {
        let policies = [web_policy(None, Some(Confidentiality::Private), None)];
        let src = source("src.web", Origin::Web, Labels::default());

        let (resolved, basis) = resolve_source_labels(&src, &policies);

        assert_eq!(resolved.confidentiality, Confidentiality::Private);
        assert_eq!(basis.confidentiality, LabelBasis::WorkspaceOverride);
        assert_eq!(resolved.trust, Trust::Untrusted);
        assert_eq!(basis.trust, LabelBasis::OriginDefault);
    }

    #[test]
    fn combine_any_untrusted_is_untrusted() {
        let combined = combine_labels(&[
            labels(Trust::Trusted, Confidentiality::Public, Integrity::High),
            labels(Trust::Untrusted, Confidentiality::Public, Integrity::High),
        ]);
        assert_eq!(combined.trust, Trust::Untrusted);
    }

    #[test]
    fn combine_unknown_without_untrusted_is_unknown() {
        let combined = combine_labels(&[
            labels(Trust::Trusted, Confidentiality::Public, Integrity::High),
            labels(Trust::Unknown, Confidentiality::Public, Integrity::High),
        ]);
        assert_eq!(combined.trust, Trust::Unknown);
    }

    #[test]
    fn combine_all_trusted_is_trusted() {
        let combined = combine_labels(&[
            labels(Trust::Trusted, Confidentiality::Public, Integrity::High),
            labels(Trust::Trusted, Confidentiality::Public, Integrity::High),
        ]);
        assert_eq!(combined.trust, Trust::Trusted);
    }

    #[test]
    fn combine_confidentiality_takes_max_rank() {
        let cases = [
            (
                Confidentiality::Public,
                Confidentiality::Private,
                Confidentiality::Private,
            ),
            (
                Confidentiality::Private,
                Confidentiality::Secret,
                Confidentiality::Secret,
            ),
            (
                Confidentiality::Public,
                Confidentiality::Identity,
                Confidentiality::Identity,
            ),
        ];
        for (a, b, expected) in cases {
            let combined = combine_labels(&[
                labels(Trust::Trusted, a, Integrity::High),
                labels(Trust::Trusted, b, Integrity::High),
            ]);
            assert_eq!(combined.confidentiality, expected, "{a:?} + {b:?}");
        }
    }

    #[test]
    fn combine_unknown_conf_outranks_public_only() {
        let with_public = combine_labels(&[
            labels(Trust::Trusted, Confidentiality::Public, Integrity::High),
            labels(Trust::Trusted, Confidentiality::Unknown, Integrity::High),
        ]);
        assert_eq!(with_public.confidentiality, Confidentiality::Unknown);

        let with_private = combine_labels(&[
            labels(Trust::Trusted, Confidentiality::Private, Integrity::High),
            labels(Trust::Trusted, Confidentiality::Unknown, Integrity::High),
        ]);
        assert_eq!(with_private.confidentiality, Confidentiality::Private);
    }

    #[test]
    fn combine_integrity_takes_min_rank() {
        let high_low = combine_labels(&[
            labels(Trust::Trusted, Confidentiality::Public, Integrity::High),
            labels(Trust::Trusted, Confidentiality::Public, Integrity::Low),
        ]);
        assert_eq!(high_low.integrity, Integrity::Low);

        let high_unknown = combine_labels(&[
            labels(Trust::Trusted, Confidentiality::Public, Integrity::High),
            labels(Trust::Trusted, Confidentiality::Public, Integrity::Unknown),
        ]);
        assert_eq!(high_unknown.integrity, Integrity::Unknown);
    }

    #[test]
    fn combine_empty_contributors_all_unknown() {
        assert_eq!(combine_labels(&[]), Labels::default());
    }
}
