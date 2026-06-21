//! Decision engine. Two entry points:
//!
//! - `Engine::check` — synchronous, deterministic-only (Tier 1). Retained
//!   for replay tooling and benchmarks while the public runtime path moves
//!   through `GuardEvent`.
//!
//! - `Engine::check_async` — full pipeline through the parallel-cancel
//!   orchestrator. Tiers 2 and 3 are stubs in PR 3 and get fleshed out
//!   in PR 5/6 (`tl-fuzzy`) and PR 7-9 (`tl-llm`).

pub mod context;
pub mod engine;
pub mod engine_match;
pub mod event_pipeline;
pub mod event_policy;
pub mod fuzzy;
pub mod pipeline;
pub mod tiers;

pub use context as handler;
pub use context::{
    FuzzyChecker, FuzzyHit, HandlerCtx, KnowledgeRetrievalRequest, KnowledgeRetriever,
    KnowledgeSnippet, NoOpFuzzyChecker, NoOpKnowledgeRetriever, NoOpProfileResolver,
    ProfileResolver,
};
pub use engine::Engine;
pub use event_pipeline::{
    ApprovalChecker, Checker, CheckerFinding, CheckerModes, DecisionComposer, EventPipelineCtx,
    InformationFlowChecker, LabelPolicyProvider, LabelPolicyUnavailable, LabelResolver,
    MemoryChecker, ModeAwareDecisionComposer, NoOpChecker, NoOpDecisionComposer,
    NoOpLabelPolicyProvider, NoOpLabelResolver, NoOpNormalizer, NoOpPrincipalResolver,
    NoOpProvenanceResolver, NoOpSignalProvider, NoOpToolMetadataProvider, NoOpTracePersister,
    Normalizer, ParameterAuthChecker, PolicyLabelResolver, PrincipalResolver, ProvenancePropagator,
    ProvenanceResolver, Signal, SignalProvider, ToolMetadataProvider, ToolMetadataUnavailable,
    TracePersister, DEFAULT_SIGNAL_BUDGET,
};
pub use event_policy::{
    evaluate_event_policies, EventPolicyEvalCtx, EventPolicyOutcome, SemanticPolicyJudge,
    SemanticPolicyJudgeInput, SemanticPolicyJudgeResult,
};
pub use fuzzy::{BuildError as FuzzyBuildError, HnswFuzzyChecker};
pub use pipeline::orchestrator as orchestrate;
pub use pipeline::{BlockSignal, DefaultTierRunner, OrchestrateConfig, TierOutput, TierRunner};
pub use tiers::deterministic as tier1;
pub use tiers::fuzzy as tier2;
pub use tiers::llm as tier3;
#[cfg(test)]
mod tests;
