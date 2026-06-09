//! Decision engine. Two entry points:
//!
//! - `Engine::check` — synchronous, deterministic-only (Tier 1). Used by
//!   the existing `/v1/check` handler, the replay tool, and benchmarks.
//!   Microsecond-scale.
//!
//! - `Engine::check_async` — full pipeline through the parallel-cancel
//!   orchestrator. Tiers 2 and 3 are stubs in PR 3 and get fleshed out
//!   in PR 5/6 (`tl-fuzzy`) and PR 7-9 (`tl-llm`).

pub mod context;
pub mod engine;
pub mod engine_match;
pub mod event_pipeline;
pub mod fuzzy;
pub mod pipeline;
pub mod tiers;

pub use context as handler;
pub use context::{
    FuzzyChecker, FuzzyHit, HandlerCtx, NoOpFuzzyChecker, NoOpProfileResolver, ProfileResolver,
};
pub use engine::Engine;
pub use event_pipeline::{
    Checker, CheckerFinding, DecisionComposer, EventPipelineCtx, LabelResolver,
    LegacyCheckNormalizer, NoOpChecker, NoOpDecisionComposer, NoOpLabelResolver,
    NoOpPrincipalResolver, NoOpProvenanceResolver, NoOpSignalProvider, NoOpToolMetadataProvider,
    NoOpTracePersister, Normalizer, PrincipalResolver, ProvenanceResolver, Signal, SignalProvider,
    ToolMetadataProvider, TracePersister,
};
pub use fuzzy::{BuildError as FuzzyBuildError, HnswFuzzyChecker};
pub use pipeline::orchestrator as orchestrate;
pub use pipeline::{BlockSignal, DefaultTierRunner, OrchestrateConfig, TierOutput, TierRunner};
pub use tiers::deterministic as tier1;
pub use tiers::fuzzy as tier2;
pub use tiers::llm as tier3;
#[cfg(test)]
mod tests;
