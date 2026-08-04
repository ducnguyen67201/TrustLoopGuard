//! OpenAI client. Speaks the v1 chat-completions API with a
//! `response_format: { type: "json_schema", ... }` constraint so the
//! model is forced to emit a JSON object matching the requested shape.

use std::time::Duration;

use async_trait::async_trait;

use crate::client::{JsonSchema, LlmClient, LlmCompletionOptions, LlmError, LlmOutput};
use crate::wire::{call_chat_completions, chat_completion_body, ReasoningWireFormat, RequestParts};

const DEFAULT_BASE_URL: &str = "https://api.openai.com";

pub struct OpenAiClient {
    http: reqwest::Client,
    base_url: String,
    api_key: String,
}

impl OpenAiClient {
    /// Construct a client using `OPENAI_API_KEY` from the environment.
    pub fn from_env() -> Result<Self, LlmError> {
        let key = std::env::var("OPENAI_API_KEY")
            .map_err(|_| LlmError::Http("OPENAI_API_KEY not set".into()))?;
        Self::new(key)
    }

    pub fn new(api_key: impl Into<String>) -> Result<Self, LlmError> {
        let http = reqwest::Client::builder()
            .build()
            .map_err(|e| LlmError::Http(e.to_string()))?;
        Ok(Self {
            http,
            base_url: DEFAULT_BASE_URL.into(),
            api_key: api_key.into(),
        })
    }

    /// Override the base URL — used by tests to point at wiremock and
    /// by future deployments behind a self-hosted reverse proxy.
    pub fn with_base_url(mut self, base_url: impl Into<String>) -> Self {
        self.base_url = base_url.into();
        self
    }
}

#[async_trait]
impl LlmClient for OpenAiClient {
    async fn complete(
        &self,
        model: &str,
        prompt: &str,
        schema: &JsonSchema,
        deadline: Duration,
    ) -> Result<LlmOutput, LlmError> {
        self.complete_with_options(
            model,
            prompt,
            schema,
            deadline,
            &LlmCompletionOptions::default(),
        )
        .await
    }

    async fn complete_with_options(
        &self,
        model: &str,
        prompt: &str,
        schema: &JsonSchema,
        deadline: Duration,
        options: &LlmCompletionOptions,
    ) -> Result<LlmOutput, LlmError> {
        call_chat_completions(RequestParts {
            http: &self.http,
            url: format!("{}/v1/chat/completions", self.base_url),
            api_key: &self.api_key,
            extra_headers: &[],
            body: chat_completion_body(model, prompt, schema, options, ReasoningWireFormat::OpenAi),
            deadline,
        })
        .await
    }
}
