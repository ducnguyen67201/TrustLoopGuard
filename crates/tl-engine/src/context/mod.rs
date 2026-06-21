//! Cross-cutting context passed into the orchestrator and each tier.
//!
//! `HandlerCtx` aggregates the collaborators tier 2 / tier 3 need:
//! profile resolution, decision cache, fuzzy similarity check, managed
//! knowledge retrieval, and the
//! LLM router. Each lives behind a trait (or, for the LLM router, a
//! concrete struct from `tl-llm`) so concrete impls land in their own
//! crates over later PRs (`tl-cache`, `tl-storage`) without churning
//! the engine.

use async_trait::async_trait;
use std::sync::Arc;
use tl_cache::MokaCache;
use tl_core::{AgentProfile, Severity};
use tl_llm::LlmRouter;
use tl_policy::Action;

/// Resolves an `agent_id` to its parsed profile. The real implementation
/// will live in `tl-server` (PR 14/15) backed by `AgentRepo` + Postgres,
/// with an in-process LRU cache.
#[async_trait]
pub trait ProfileResolver: Send + Sync {
    async fn resolve(&self, workspace_id: &str, agent_id: &str) -> Option<Arc<AgentProfile>>;
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

#[derive(Debug, Clone)]
pub struct KnowledgeRetrievalRequest {
    pub workspace_id: String,
    pub agent_id: String,
    pub source_ids: Vec<String>,
    pub input: String,
    pub proposed_output: String,
}

#[derive(Debug, Clone, PartialEq)]
pub struct KnowledgeSnippet {
    pub source_id: String,
    pub chunk_id: String,
    pub score: f32,
    pub text: String,
}

#[async_trait]
pub trait KnowledgeRetriever: Send + Sync {
    async fn retrieve(&self, request: KnowledgeRetrievalRequest) -> Vec<KnowledgeSnippet>;
}

/// Bundle of collaborators. Cloning is cheap (`Arc`).
#[derive(Clone)]
pub struct HandlerCtx {
    pub profile_resolver: Arc<dyn ProfileResolver>,
    /// Decision cache. Use `MokaCache::disabled()` to bypass caching
    /// (every request runs the full tier pipeline).
    pub cache: Arc<MokaCache>,
    pub fuzzy: Arc<dyn FuzzyChecker>,
    /// Retrieves small, trusted grounding snippets for Tier 3. Use
    /// `NoOpKnowledgeRetriever` to keep current per-request `context.docs`
    /// behaviour only.
    pub knowledge: Arc<dyn KnowledgeRetriever>,
    /// LLM router used by Tier 3. Use `LlmRouter::empty()` to disable
    /// Tier 3 entirely (judges that aren't routed report `Skipped`).
    pub llm: Arc<LlmRouter>,
}

// ---- NoOp impls — useful in tests and `Engine::empty()` startup. ----

pub struct NoOpProfileResolver;
#[async_trait]
impl ProfileResolver for NoOpProfileResolver {
    async fn resolve(&self, _workspace_id: &str, _agent_id: &str) -> Option<Arc<AgentProfile>> {
        None
    }
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

pub struct NoOpKnowledgeRetriever;
#[async_trait]
impl KnowledgeRetriever for NoOpKnowledgeRetriever {
    async fn retrieve(&self, _request: KnowledgeRetrievalRequest) -> Vec<KnowledgeSnippet> {
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
            cache: Arc::new(MokaCache::disabled()),
            fuzzy: Arc::new(NoOpFuzzyChecker),
            knowledge: Arc::new(NoOpKnowledgeRetriever),
            llm: Arc::new(LlmRouter::empty()),
        }
    }
}

impl Default for HandlerCtx {
    fn default() -> Self {
        Self::no_op()
    }
}
