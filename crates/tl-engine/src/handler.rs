//! Cross-cutting context passed into the orchestrator and each tier.
//!
//! `HandlerCtx` aggregates the four collaborators tier 2 / tier 3 need:
//! profile resolution, embeddings, LLM judges, and the decision cache.
//! Each lives behind a trait so concrete impls can land in their own
//! crates over later PRs (`tl-fuzzy`, `tl-llm`, `tl-cache`, `tl-storage`)
//! without churning the engine.
//!
//! For PR 3, the stub tiers don't actually consult the ctx — but the
//! shape is locked here so tier wiring lands additively.

use async_trait::async_trait;
use std::sync::Arc;
use tl_core::{AgentProfile, Decision};

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

/// Text embedder for tier 2 fuzzy similarity. Real impl in `tl-fuzzy` (PR 5)
/// using `fastembed-rs` + `BGEBaseEnSmall`.
#[async_trait]
pub trait Embedder: Send + Sync {
    async fn embed(&self, texts: &[&str]) -> Vec<Vec<f32>>;
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
    pub embedder: Arc<dyn Embedder>,
    pub llm: Arc<dyn LlmJudge>,
}

// ---- NoOp impls — useful in tests, in `Engine::empty()` startup, and in
// any environment where a real backend hasn't been wired yet. They never
// produce reasons, so tier 2/3 with these are equivalent to "skipped". ----

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

pub struct NoOpEmbedder;
#[async_trait]
impl Embedder for NoOpEmbedder {
    async fn embed(&self, texts: &[&str]) -> Vec<Vec<f32>> {
        texts.iter().map(|_| vec![0.0; 8]).collect()
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
            embedder: Arc::new(NoOpEmbedder),
            llm: Arc::new(NoOpJudge),
        }
    }
}

impl Default for HandlerCtx {
    fn default() -> Self {
        Self::no_op()
    }
}
