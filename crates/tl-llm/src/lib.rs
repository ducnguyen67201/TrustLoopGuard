//! Featherlane AI LLM provider clients.
//!
//! v0 ships two providers behind a single trait:
//!
//! - [`OpenAiClient`] — direct OpenAI v1 chat-completions
//! - [`OpenRouterClient`] — OpenAI-compatible aggregator
//!
//! Both speak the `response_format: { type: "json_schema", strict: true }`
//! constraint so structured outputs are guaranteed. The router is the
//! first-party chokepoint for failover, typed workload selection, optional
//! reasoning, and runtime-judge budgets.

pub mod budget;
pub mod client;
pub mod config;
pub mod openai;
pub mod openrouter;
pub mod prompts;
pub mod router;
mod wire;

pub use budget::{BudgetExceeded, TokenBudget};
pub use client::{JsonSchema, LlmClient, LlmCompletionOptions, LlmError, LlmOutput};
pub use config::{
    BudgetConfig, ConfigError, ProviderConfig, ProviderTarget, ReasoningEffort, RouteConfig,
    RouterConfig, ROUTER_CONFIG_SCHEMA_VERSION,
};
pub use openai::OpenAiClient;
pub use openrouter::OpenRouterClient;
pub use router::{
    AuditedLlmError, AuditedLlmOutput, JudgeKind, LlmCallAudit, LlmRouteKind, LlmRouter,
    ResolvedRoute, RouterBuildError,
};
