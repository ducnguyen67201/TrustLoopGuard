//! TrustLoopGuard Rust SDK.
//!
//! Async client over reqwest with typed errors, exponential-backoff
//! retries (honoring `Retry-After`), bearer-token auth, and `tracing`
//! spans on every call. The retry policy lives in [`RetryConfig`] —
//! callers can swap in their own (voice-channel callers should usually
//! disable retries with `max_attempts = 1`).

use std::time::Instant;

use tracing::{debug, instrument, warn, Span};

mod error;
mod guardrails;
mod http;
mod retry;
mod runs;
#[cfg(test)]
mod tests;

pub use error::SdkError;
pub use retry::RetryConfig;

// Re-export the wire types so callers don't reach into `tl_core`
// directly. Doing so would violate the SDK-driven discipline (rule 2 in
// docs/SDK_DRIVEN.md) and break example apps that lint against internal
// imports.
pub use tl_core::{
    ApiError, ApiErrorCode, Channel, CheckRequest, CreateRunEventRequest, CreateRunRequest,
    Decision, GuardrailGenerateResponse, GuardrailListResponse, RunDetail, RunEventKind,
    RunEventListResponse, RunEventSummary, RunKind, RunListResponse, RunStatus, RunSummary,
    Severity, TraceListResponse, TriggeredPolicy, UpdateRunRequest, Verdict,
};

// `CheckRequest::context` is typed as `serde_json::Value` on the wire,
// so callers building requests need access to that type. Re-export the
// crate so example apps don't take a separate dependency.
pub use serde_json;

#[derive(Debug, Clone)]
pub struct Client {
    base_url: String,
    api_key: Option<String>,
    http: reqwest::Client,
    retry: RetryConfig,
}

impl Client {
    pub fn new(base_url: impl Into<String>) -> Self {
        Self {
            base_url: base_url.into(),
            api_key: None,
            http: reqwest::Client::new(),
            retry: RetryConfig::default(),
        }
    }

    pub fn with_api_key(mut self, key: impl Into<String>) -> Self {
        self.api_key = Some(key.into());
        self
    }

    /// Override the retry policy. Voice callers typically pass
    /// `RetryConfig { max_attempts: 1, ..Default::default() }` to opt out.
    pub fn with_retry(mut self, retry: RetryConfig) -> Self {
        self.retry = retry;
        self
    }

    /// Override the underlying reqwest client (for custom timeouts,
    /// proxies, or test fixtures).
    pub fn with_http_client(mut self, http: reqwest::Client) -> Self {
        self.http = http;
        self
    }

    /// Send a `CheckRequest`. Retries transient errors per the configured
    /// policy. Spans are emitted under `target = "tl_sdk_rust::check"`.
    #[instrument(
        name = "tl_sdk_rust::check",
        skip_all,
        fields(
            agent_id = %req.agent_id,
            channel = ?req.channel,
            attempt = tracing::field::Empty,
        )
    )]
    pub async fn check(&self, req: &CheckRequest) -> Result<Decision, SdkError> {
        let start = Instant::now();
        let mut attempt: u32 = 0;
        loop {
            attempt += 1;
            Span::current().record("attempt", attempt);
            match self.send_once(req).await {
                Ok(decision) => {
                    debug!(latency_ms = decision.latency_ms, "check ok");
                    return Ok(decision);
                }
                Err(err) => {
                    let elapsed = start.elapsed();
                    let jitter = rand::random::<f64>();
                    match self.retry.next_delay(attempt, elapsed, &err, jitter) {
                        Some(delay) => {
                            warn!(
                                ?delay,
                                attempt,
                                error = %err,
                                "retrying after transient SDK error",
                            );
                            tokio::time::sleep(delay).await;
                        }
                        None => return Err(err),
                    }
                }
            }
        }
    }
}

/// Synthesize an `ApiError` from a raw status when the server didn't
/// return our canonical body. Crate-private; the error module needs it.
pub(crate) fn synthesize_api_error(status: u16, body: &str) -> tl_core::ApiError {
    let code = tl_core::ApiErrorCode::from_http_status(status);
    tl_core::ApiError {
        code,
        message: if body.is_empty() {
            format!("server returned status {status}")
        } else {
            body.to_string()
        },
        retriable: code.default_retriable(),
        details: serde_json::Value::Null,
    }
}
