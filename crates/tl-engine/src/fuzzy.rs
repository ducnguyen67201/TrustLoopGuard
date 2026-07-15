//! Tier 2 fuzzy checker — wraps an `Embedder` + HNSW index for semantic
//! similarity, and a list of `(literal, policy_id)` pairs for Levenshtein
//! bypass detection.
//!
//! Built once at engine startup from a tenant's policy set and reused
//! across requests. Per-agent scope embedding (out_of_scope topics) is
//! deferred to a later PR — for v0 this checker is tenant-scoped only.

use std::sync::Arc;

use async_trait::async_trait;
use tl_fuzzy::{fuzzy_contains, Embedder, HnswIndex};
use tl_policy::{MatchClause, Matcher, Policy};

use crate::context::{FuzzyChecker, FuzzyHit};

/// Default cosine-similarity threshold for HNSW hits.
pub const DEFAULT_SEMANTIC_THRESHOLD: f32 = 0.85;
/// Default normalised-edit-distance threshold for Levenshtein bypass hits.
pub const DEFAULT_LEVENSHTEIN_THRESHOLD: f32 = 0.85;

/// Tenant-scoped fuzzy checker. Built async (the embedder calls into
/// model code that wants `await`), held behind an `Arc` afterwards.
pub struct HnswFuzzyChecker {
    embedder: Arc<dyn Embedder>,
    index: HnswIndex,
    /// Map from `label` (the policy id we inserted into the HNSW) to the
    /// originating policy. Lets us reconstruct severity / action / rewrite
    /// on hit without re-walking the policy set.
    semantic_lookup: std::collections::HashMap<String, Policy>,
    /// Literal patterns extracted from `Matcher::Literal(...)` plus their
    /// owning policy id. Scanned with Levenshtein on every check.
    levenshtein: Vec<(String, Policy)>,
    semantic_threshold: f32,
    levenshtein_threshold: f32,
}

impl HnswFuzzyChecker {
    /// Build the checker from a policy set. Semantic matchers go into
    /// the HNSW index; literal matchers go into the Levenshtein list.
    /// Regex matchers are tier 1 territory and get skipped here.
    pub async fn build(
        policies: &[Policy],
        embedder: Arc<dyn Embedder>,
    ) -> Result<Self, BuildError> {
        let dim = embedder.dimension();
        let mut semantic_texts: Vec<String> = vec![];
        let mut semantic_labels: Vec<String> = vec![];
        let mut semantic_lookup = std::collections::HashMap::new();
        let mut levenshtein: Vec<(String, Policy)> = vec![];

        for policy in policies {
            walk_clause(&policy.r#match, |m| match m {
                Matcher::Semantic(text) => {
                    semantic_texts.push(text.clone());
                    semantic_labels.push(policy.id.clone());
                    semantic_lookup.insert(policy.id.clone(), policy.clone());
                }
                Matcher::Literal(text) => {
                    levenshtein.push((text.clone(), policy.clone()));
                }
                Matcher::Regex(_) => { /* tier 1 */ }
            });
        }

        let mut index = HnswIndex::new(dim, semantic_texts.len().max(16));
        if !semantic_texts.is_empty() {
            let vectors = embedder
                .embed(&semantic_texts)
                .await
                .map_err(|e| BuildError::Embed(e.to_string()))?;
            for (label, vec) in semantic_labels.into_iter().zip(vectors) {
                index.insert(label, vec);
            }
        }

        Ok(Self {
            embedder,
            index,
            semantic_lookup,
            levenshtein,
            semantic_threshold: DEFAULT_SEMANTIC_THRESHOLD,
            levenshtein_threshold: DEFAULT_LEVENSHTEIN_THRESHOLD,
        })
    }

    pub fn with_semantic_threshold(mut self, t: f32) -> Self {
        self.semantic_threshold = t;
        self
    }

    pub fn with_levenshtein_threshold(mut self, t: f32) -> Self {
        self.levenshtein_threshold = t;
        self
    }

    /// Number of indexed semantic patterns. Useful for diagnostics.
    pub fn semantic_count(&self) -> usize {
        self.index.len()
    }

    pub fn levenshtein_count(&self) -> usize {
        self.levenshtein.len()
    }
}

#[derive(Debug, thiserror::Error)]
pub enum BuildError {
    #[error("embedder failed: {0}")]
    Embed(String),
}

#[async_trait]
impl FuzzyChecker for HnswFuzzyChecker {
    async fn check(&self, draft: &str) -> Vec<FuzzyHit> {
        let mut hits = vec![];
        let mut seen_ids = std::collections::HashSet::new();

        // -- Semantic (HNSW) --
        if !self.index.is_empty() {
            if let Ok(vectors) = self.embedder.embed(&[draft.to_string()]).await {
                if let Some(qvec) = vectors.into_iter().next() {
                    for hit in self
                        .index
                        .query(&qvec, 8, self.semantic_threshold)
                        .into_iter()
                    {
                        if let Some(policy) = self.semantic_lookup.get(&hit.label) {
                            if seen_ids.insert(policy.id.clone()) {
                                hits.push(FuzzyHit {
                                    policy_id: policy.id.clone(),
                                    severity: policy.severity,
                                    action: policy.action,
                                    message: format!(
                                        "semantic match (cosine={:.3}) on policy `{}`",
                                        hit.similarity, policy.id
                                    ),
                                    safe_output: policy.rewrite.clone(),
                                });
                            }
                        }
                    }
                }
            }
        }

        // -- Levenshtein bypass --
        for (literal, policy) in &self.levenshtein {
            if seen_ids.contains(&policy.id) {
                continue; // already caught semantically
            }
            if fuzzy_contains(draft, literal, self.levenshtein_threshold) {
                seen_ids.insert(policy.id.clone());
                hits.push(FuzzyHit {
                    policy_id: policy.id.clone(),
                    severity: policy.severity,
                    action: policy.action,
                    message: format!(
                        "fuzzy literal match on `{literal}` for policy `{}`",
                        policy.id
                    ),
                    safe_output: policy.rewrite.clone(),
                });
            }
        }

        hits
    }
}

fn walk_clause(clause: &MatchClause, mut f: impl FnMut(&Matcher)) {
    match clause {
        MatchClause::Single(m) => f(m),
        MatchClause::Any { any } => any.iter().for_each(&mut f),
        MatchClause::All { all } => all.iter().for_each(&mut f),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tl_core::{AuthorizationEffect, Severity};
    use tl_fuzzy::MockEmbedder;
    use tl_policy::load_str;

    fn semantic_policy(id: &str, text: &str, action: &str, rewrite: Option<&str>) -> Policy {
        let r = rewrite
            .map(|s| format!("\nrewrite: \"{s}\""))
            .unwrap_or_default();
        let yaml = format!(
            "id: {id}\nmatch:\n  semantic: \"{text}\"\naction: {action}\nseverity: high{r}"
        );
        load_str(&yaml).expect("policy")
    }

    fn literal_policy(id: &str, text: &str) -> Policy {
        let yaml = format!("id: {id}\nmatch:\n  literal: \"{text}\"\naction: deny\nseverity: high");
        load_str(&yaml).expect("policy")
    }

    #[tokio::test]
    async fn empty_policies_yields_no_hits() {
        let checker = HnswFuzzyChecker::build(&[], Arc::new(MockEmbedder::default()))
            .await
            .unwrap();
        assert!(checker.check("any draft").await.is_empty());
        assert_eq!(checker.semantic_count(), 0);
        assert_eq!(checker.levenshtein_count(), 0);
    }

    #[tokio::test]
    async fn semantic_match_on_paraphrase() {
        let policies = vec![semantic_policy(
            "no-refund-promises",
            "i promise full refund to the customer",
            "deny",
            Some("Let me connect you with a teammate."),
        )];
        // Lower the threshold a bit since MockEmbedder produces less
        // smooth vectors than a real model. Real embedders pass 0.85.
        let checker = HnswFuzzyChecker::build(&policies, Arc::new(MockEmbedder::new(128)))
            .await
            .unwrap()
            .with_semantic_threshold(0.4);

        let hits = checker
            .check("i promise complete refund right now to the customer")
            .await;
        assert_eq!(hits.len(), 1);
        assert_eq!(hits[0].policy_id, "no-refund-promises");
        assert_eq!(hits[0].action, AuthorizationEffect::Deny);
        assert_eq!(hits[0].severity, Severity::High);
        assert!(hits[0].safe_output.is_some());
        assert_eq!(checker.semantic_count(), 1);
    }

    #[tokio::test]
    async fn levenshtein_catches_typo_bypass() {
        let policies = vec![literal_policy("no-refund-word", "refund")];
        let checker = HnswFuzzyChecker::build(&policies, Arc::new(MockEmbedder::default()))
            .await
            .unwrap()
            .with_levenshtein_threshold(0.8);

        // Typo bypass — `refunddd` is edit-distance 2 from `refund`
        // which clears the 0.8 normalised similarity threshold.
        let hits = checker.check("you can refunddd it any time").await;
        assert_eq!(hits.len(), 1);
        assert_eq!(hits[0].policy_id, "no-refund-word");
        assert_eq!(checker.levenshtein_count(), 1);
    }

    #[tokio::test]
    async fn levenshtein_misses_unrelated_text() {
        let policies = vec![literal_policy("no-refund-word", "refund")];
        let checker = HnswFuzzyChecker::build(&policies, Arc::new(MockEmbedder::default()))
            .await
            .unwrap();
        assert!(checker
            .check("the weather in tokyo is sunny today")
            .await
            .is_empty());
    }

    #[tokio::test]
    async fn dedup_when_both_tiers_match_same_policy() {
        // A policy with both a semantic AND a literal matcher (Any) —
        // a draft that triggers both should produce only one FuzzyHit.
        let yaml = r#"
id: dual-match
match:
  any:
    - semantic: "promising refund"
    - literal: "refund"
action: deny
severity: high
"#;
        let policy = load_str(yaml).expect("policy");
        let checker = HnswFuzzyChecker::build(&[policy], Arc::new(MockEmbedder::new(128)))
            .await
            .unwrap()
            .with_semantic_threshold(0.4)
            .with_levenshtein_threshold(0.8);

        let hits = checker.check("we are promising a refund").await;
        let policy_ids: Vec<&str> = hits.iter().map(|h| h.policy_id.as_str()).collect();
        assert_eq!(policy_ids, vec!["dual-match"]);
    }
}
