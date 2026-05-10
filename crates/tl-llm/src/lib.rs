//! TrustLoopGuard LLM provider clients.
//!
//! v0 ships two providers behind a single trait:
//!
//! - [`OpenAiClient`] — direct OpenAI v1 chat-completions
//! - [`OpenRouterClient`] — OpenAI-compatible aggregator
//!
//! Both speak the `response_format: { type: "json_schema", strict: true }`
//! constraint so judge outputs are structurally guaranteed. The router
//! that picks between them — including failover, per-tenant budgets, and
//! per-judge model selection — lands in PR 8.

pub mod budget;
pub mod client;
pub mod config;
pub mod openai;
pub mod openrouter;
pub mod prompts;
pub mod router;
mod wire;

pub use budget::{BudgetExceeded, TokenBudget};
pub use client::{JsonSchema, LlmClient, LlmError, LlmOutput};
pub use config::{
    BudgetConfig, ConfigError, ProviderConfig, ProviderTarget, RouteConfig, RouterConfig,
};
pub use openai::OpenAiClient;
pub use openrouter::OpenRouterClient;
pub use router::{JudgeKind, LlmRouter, ResolvedRoute, RouterBuildError};
