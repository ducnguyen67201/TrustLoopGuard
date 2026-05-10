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

pub mod client;
pub mod openai;
pub mod openrouter;
mod wire;

pub use client::{JsonSchema, LlmClient, LlmError, LlmOutput};
pub use openai::OpenAiClient;
pub use openrouter::OpenRouterClient;
