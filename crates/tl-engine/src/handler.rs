//! Cross-cutting context passed into the orchestrator and each tier.
//!
//! `HandlerCtx` aggregates the four collaborators tier 2 / tier 3 need:
//! profile resolution, decision cache, fuzzy similarity check, and LLM
//! judge. Each lives behind a trait so concrete impls can land in their
//! own crates over later PRs (`tl-cache`, `tl-llm`, `tl-storage`)
//! without churning the engine.
//!
//! For PR 6, `FuzzyChecker` got its first real implementation
//! (`HnswFuzzyChecker` in `crate::fuzzy`). LLM judge and decision cache
//! remain `NoOp*` until PRs 7-9 and 10.

use async_trait::async_trait;
use std::sync::Arc;
use tl_core::{AgentProfile, Decision, Severity};
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

/// LLM judge for tier 3 grounded reasoning. Real impl in `tl-llm` (PR 7-9)
/// behind an `LlmRouter` that owns provider failover and per-tenant budgets.
#[async_trait]
pub trait LlmJudge: Send + Sync {
    async fn judge(&self, prompt: &str) -> Result<serde_json::Value, JudgeError>;
}

#[derive(Debug, thiserror::Error)]
pub enum JudgeError {
    #[error("provider error: {0}")]
    Provider(String),
    #[error("budget exceeded")]
    BudgetExceeded,
    #[error("timeout")]
    Timeout,
}

/// Bundle of collaborators. Cloning is cheap (`Arc`).
#[derive(Clone)]
pub struct HandlerCtx {
    pub profile_resolver: Arc<dyn ProfileResolver>,
    pub cache: Arc<dyn DecisionCache>,
    pub fuzzy: Arc<dyn FuzzyChecker>,
    pub llm: Arc<dyn LlmJudge>,
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

pub struct NoOpJudge;
#[async_trait]
impl LlmJudge for NoOpJudge {
    async fn judge(&self, _prompt: &str) -> Result<serde_json::Value, JudgeError> {
        Ok(serde_json::Value::Null)
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
            llm: Arc::new(NoOpJudge),
        }
    }
}

impl Default for HandlerCtx {
    fn default() -> Self {
        Self::no_op()
    }
}
