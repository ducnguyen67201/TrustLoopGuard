//! `LlmClient` trait and shared types.
//!
//! Every provider (OpenAI, OpenRouter, future BYOK targets) implements
//! this trait. The router in PR 8 dispatches across providers without
//! caring which concrete impl is behind each one.

use std::time::Duration;

use async_trait::async_trait;
use serde::{Deserialize, Serialize};

use crate::config::ReasoningEffort;

/// JSON Schema description sent in the `response_format` field of the
/// OpenAI-compatible chat completions API. Providers translate this
/// into their wire-level shape; the trait surface stays neutral.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JsonSchema {
    /// A short identifier shown to the model (e.g. "HallucinationVerdict").
    pub name: String,
    /// The JSON schema document itself.
    pub schema: serde_json::Value,
}

/// One judge call's structured output plus token accounting.
#[derive(Debug, Clone)]
pub struct LlmOutput {
    pub json: serde_json::Value,
    pub prompt_tokens: u32,
    pub completion_tokens: u32,
}

#[derive(Debug, Clone, Default)]
pub struct LlmCompletionOptions {
    pub reasoning_effort: Option<ReasoningEffort>,
}

#[derive(Debug, thiserror::Error)]
pub enum LlmError {
    #[error("http error: {0}")]
    Http(String),
    #[error("provider returned status {0}: {1}")]
    Status(u16, String),
    #[error("response parse: {0}")]
    Parse(String),
    #[error("provider response missing field: {0}")]
    MissingField(&'static str),
    #[error("timeout after {0:?}")]
    Timeout(Duration),
    #[error("budget exceeded")]
    BudgetExceeded,
}

#[async_trait]
pub trait LlmClient: Send + Sync {
    /// Run a single chat completion against the given model. The provider
    /// is expected to enforce `deadline` as a hard timeout — exceeding it
    /// must return `LlmError::Timeout`, not block the caller indefinitely.
    async fn complete(
        &self,
        model: &str,
        prompt: &str,
        schema: &JsonSchema,
        deadline: Duration,
    ) -> Result<LlmOutput, LlmError>;

    /// Run a completion with provider-neutral optional controls. The default
    /// preserves source compatibility for existing clients and test doubles.
    async fn complete_with_options(
        &self,
        model: &str,
        prompt: &str,
        schema: &JsonSchema,
        deadline: Duration,
        _options: &LlmCompletionOptions,
    ) -> Result<LlmOutput, LlmError> {
        self.complete(model, prompt, schema, deadline).await
    }
}
