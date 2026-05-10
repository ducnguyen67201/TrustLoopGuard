//! Cross-cutting context passed into the orchestrator and each tier.
//!
//! `HandlerCtx` aggregates the four collaborators tier 2 / tier 3 need:
//! profile resolution, decision cache, fuzzy similarity check, and the
//! LLM router. Each lives behind a trait (or, for the LLM router, a
//! concrete struct from `tl-llm`) so concrete impls land in their own
//! crates over later PRs (`tl-cache`, `tl-storage`) without churning
//! the engine.

use async_trait::async_trait;
use std::sync::Arc;
use tl_core::{AgentProfile, Decision, Severity};
use tl_llm::LlmRouter;
use tl_policy::Action;

/// Resolves an `agent_id` to its parsed profile. The real implementation
/// will live in `tl-server` (PR 14/15) backed by `AgentRepo` + Postgres,
/// with an in-process LRU cache.
#[async_trait]
pub trait ProfileResolver: Send + Sync {
    async fn resolve(&self, agent_id: &str) -> Option<Arc<AgentProfile>>;
}

/// Decision cache. Keys are `blake3(domain || agent_id || input || draft)`.
/// Real impl lands in `tl-cache` (PR 10) using `moka`.
#[async_trait]
pub trait DecisionCache: Send + Sync {
    async fn get(&self, key: &str) -> Option<Decision>;
    async fn put(&self, key: &str, decision: Decision);
}

/// Tier 2 fuzzy similarity check. Real impl lives in `crate::fuzzy`
/// (`HnswFuzzyChecker`). The trait is the seam; implementations are
/// free to use any embedder, index, or hybrid scheme.
#[async_trait]
pub trait FuzzyChecker: Send + Sync {
    /// Run all fuzzy checks against the agent's proposed output.
    /// Returns one `FuzzyHit` per matched pattern, in arbitrary order.
    async fn check(&self, draft: &str) -> Vec<FuzzyHit>;
}

/// One fuzzy hit, ready to be folded into a `TierResult`.
#[derive(Debug, Clone)]
pub struct FuzzyHit {
    pub policy_id: String,
    pub severity: Severity,
    pub action: Action,
    pub message: String,
    pub safe_output: Option<String>,
}

/// Bundle of collaborators. Cloning is cheap (`Arc`).
#[derive(Clone)]
pub struct HandlerCtx {
    pub profile_resolver: Arc<dyn ProfileResolver>,
    pub cache: Arc<dyn DecisionCache>,
    pub fuzzy: Arc<dyn FuzzyChecker>,
    /// LLM router used by Tier 3. Use `LlmRouter::empty()` to disable
    /// Tier 3 entirely (judges that aren't routed report `Skipped`).
    pub llm: Arc<LlmRouter>,
}

// ---- NoOp impls — useful in tests and `Engine::empty()` startup. ----

pub struct NoOpProfileResolver;
#[async_trait]
impl ProfileResolver for NoOpProfileResolver {
    async fn resolve(&self, _agent_id: &str) -> Option<Arc<AgentProfile>> {
        None
    }
}

pub struct NoOpCache;
#[async_trait]
impl DecisionCache for NoOpCache {
    async fn get(&self, _key: &str) -> Option<Decision> {
        None
    }
    async fn put(&self, _key: &str, _decision: Decision) {}
}

/// `FuzzyChecker` that always returns no hits. Tier 2 with this checker
/// reports `Skipped` status — equivalent to the PR 3 stub.
pub struct NoOpFuzzyChecker;
#[async_trait]
impl FuzzyChecker for NoOpFuzzyChecker {
    async fn check(&self, _draft: &str) -> Vec<FuzzyHit> {
        vec![]
    }
}

impl HandlerCtx {
    /// Build a ctx whose components do nothing. Useful for unit tests
    /// and as a placeholder when the server boots before real backends
    /// are connected.
    pub fn no_op() -> Self {
        Self {
            profile_resolver: Arc::new(NoOpProfileResolver),
            cache: Arc::new(NoOpCache),
            fuzzy: Arc::new(NoOpFuzzyChecker),
            llm: Arc::new(LlmRouter::empty()),
        }
    }
}

impl Default for HandlerCtx {
    fn default() -> Self {
        Self::no_op()
    }
}
