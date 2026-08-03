//! OpenRouter client. The wire shape is OpenAI-compatible — we change
//! only the base URL and add an `HTTP-Referer` header so the request is
//! attributable to Featherlane AI in the OpenRouter dashboard.

use std::time::Duration;

use async_trait::async_trait;
use serde_json::json;

use crate::client::{JsonSchema, LlmClient, LlmCompletionOptions, LlmError, LlmOutput};
use crate::wire::{call_chat_completions, RequestParts};

const DEFAULT_BASE_URL: &str = "https://openrouter.ai/api";
const DEFAULT_REFERER: &str = "https://github.com/anthropics/featherlane-ai";

pub struct OpenRouterClient {
    http: reqwest::Client,
    base_url: String,
    api_key: String,
    referer: String,
}

impl OpenRouterClient {
    /// Construct a client using `OPENROUTER_API_KEY` from the environment.
    pub fn from_env() -> Result<Self, LlmError> {
        let key = std::env::var("OPENROUTER_API_KEY")
            .map_err(|_| LlmError::Http("OPENROUTER_API_KEY not set".into()))?;
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
            referer: DEFAULT_REFERER.into(),
        })
    }

    pub fn with_base_url(mut self, base_url: impl Into<String>) -> Self {
        self.base_url = base_url.into();
        self
    }

    pub fn with_referer(mut self, referer: impl Into<String>) -> Self {
        self.referer = referer.into();
        self
    }
}

#[async_trait]
impl LlmClient for OpenRouterClient {
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
        let mut body = json!({
            "model": model,
            "messages": [{ "role": "user", "content": prompt }],
            "response_format": {
                "type": "json_schema",
                "json_schema": {
                    "name": schema.name,
                    "strict": true,
                    "schema": schema.schema,
                }
            }
        });
        if let Some(effort) = options.reasoning_effort {
            body["reasoning_effort"] = json!(effort.as_str());
        }
        call_chat_completions(RequestParts {
            http: &self.http,
            url: format!("{}/v1/chat/completions", self.base_url),
            api_key: &self.api_key,
            extra_headers: &[("HTTP-Referer", &self.referer)],
            body,
            deadline,
        })
        .await
    }
}
