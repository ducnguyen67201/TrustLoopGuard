//! TrustLoopGuard Rust SDK.
//!
//! Async client over reqwest with typed errors, exponential-backoff
//! retries (honoring `Retry-After`), bearer-token auth, and `tracing`
//! spans on every call. The retry policy lives in [`RetryConfig`] —
//! callers can swap in their own (voice-channel callers should usually
//! disable retries with `max_attempts = 1`).

use std::time::{Duration, Instant};

use tracing::{debug, instrument, warn, Span};

mod error;
mod retry;

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

    /// Derive a guardrail policy set from an agent's stored
    /// `system_prompt`. The server reads the prompt, calls its
    /// configured LLM, and persists each draft with `enabled=false`,
    /// returning what was saved.
    ///
    /// Callers must have previously registered the agent (including a
    /// non-empty `system_prompt`) via `POST /v1/agents`. The endpoint
    /// returns `404` if the agent is unknown, `422` if `system_prompt`
    /// is absent, and `503` if the deployment has no LLM configured.
    #[instrument(
        name = "tl_sdk_rust::generate_guardrails",
        skip_all,
        fields(agent_id = %agent_id, attempt = tracing::field::Empty),
    )]
    pub async fn generate_guardrails(
        &self,
        agent_id: &str,
    ) -> Result<GuardrailGenerateResponse, SdkError> {
        let path = format!(
            "/v1/agents/{}/guardrails/generate",
            urlencoding::encode(agent_id)
        );
        self.retry_loop(&path, || self.send_post_empty(&path)).await
    }

    /// List guardrail policies owned by an agent. Empty when the agent
    /// has no generated policies or doesn't exist.
    #[instrument(
        name = "tl_sdk_rust::list_guardrails",
        skip_all,
        fields(agent_id = %agent_id, attempt = tracing::field::Empty),
    )]
    pub async fn list_guardrails(&self, agent_id: &str) -> Result<GuardrailListResponse, SdkError> {
        let path = format!("/v1/agents/{}/guardrails", urlencoding::encode(agent_id));
        self.retry_loop(&path, || self.send_get(&path)).await
    }

    /// Create a run that groups related guardrail checks.
    #[instrument(
        name = "tl_sdk_rust::start_run",
        skip_all,
        fields(agent_id = %req.agent_id, attempt = tracing::field::Empty),
    )]
    pub async fn start_run(&self, req: CreateRunRequest) -> Result<RunSummary, SdkError> {
        self.retry_loop("/v1/runs", || self.send_post_json("/v1/runs", &req))
            .await
    }

    /// Fetch a run with recent events and traces.
    #[instrument(
        name = "tl_sdk_rust::get_run",
        skip_all,
        fields(run_id = %run_id, attempt = tracing::field::Empty),
    )]
    pub async fn get_run(&self, run_id: &str) -> Result<RunDetail, SdkError> {
        let path = format!("/v1/runs/{}", urlencoding::encode(run_id));
        self.retry_loop(&path, || self.send_get(&path)).await
    }

    /// Update run status or metadata.
    #[instrument(
        name = "tl_sdk_rust::update_run",
        skip_all,
        fields(run_id = %run_id, attempt = tracing::field::Empty),
    )]
    pub async fn update_run(
        &self,
        run_id: &str,
        req: UpdateRunRequest,
    ) -> Result<RunSummary, SdkError> {
        let path = format!("/v1/runs/{}", urlencoding::encode(run_id));
        self.retry_loop(&path, || self.send_patch_json(&path, &req))
            .await
    }

    /// Mark a run completed.
    #[instrument(
        name = "tl_sdk_rust::finish_run",
        skip_all,
        fields(run_id = %run_id, attempt = tracing::field::Empty),
    )]
    pub async fn finish_run(&self, run_id: &str) -> Result<RunSummary, SdkError> {
        self.update_run(
            run_id,
            UpdateRunRequest {
                status: Some(RunStatus::Completed),
                metadata: None,
                ended_at: None,
            },
        )
        .await
    }

    /// Append an event to a run timeline.
    #[instrument(
        name = "tl_sdk_rust::create_run_event",
        skip_all,
        fields(run_id = %run_id, attempt = tracing::field::Empty),
    )]
    pub async fn create_run_event(
        &self,
        run_id: &str,
        req: CreateRunEventRequest,
    ) -> Result<RunEventSummary, SdkError> {
        let path = format!("/v1/runs/{}/events", urlencoding::encode(run_id));
        self.retry_loop(&path, || self.send_post_json(&path, &req))
            .await
    }

    /// List run timeline events.
    #[instrument(
        name = "tl_sdk_rust::list_run_events",
        skip_all,
        fields(run_id = %run_id, attempt = tracing::field::Empty),
    )]
    pub async fn list_run_events(&self, run_id: &str) -> Result<RunEventListResponse, SdkError> {
        let path = format!("/v1/runs/{}/events", urlencoding::encode(run_id));
        self.retry_loop(&path, || self.send_get(&path)).await
    }

    /// List traces grouped under a run.
    #[instrument(
        name = "tl_sdk_rust::list_run_traces",
        skip_all,
        fields(run_id = %run_id, attempt = tracing::field::Empty),
    )]
    pub async fn list_run_traces(&self, run_id: &str) -> Result<TraceListResponse, SdkError> {
        let path = format!("/v1/runs/{}/traces", urlencoding::encode(run_id));
        self.retry_loop(&path, || self.send_get(&path)).await
    }

    /// Shared retry harness for the new agent-bound endpoints. Keeps the
    /// retry semantics identical to `check()` without dragging that
    /// method's `CheckRequest` type into the helper signature.
    async fn retry_loop<T, F, Fut>(&self, _path: &str, mut send: F) -> Result<T, SdkError>
    where
        F: FnMut() -> Fut,
        Fut: std::future::Future<Output = Result<T, SdkError>>,
    {
        let start = Instant::now();
        let mut attempt: u32 = 0;
        loop {
            attempt += 1;
            Span::current().record("attempt", attempt);
            match send().await {
                Ok(out) => return Ok(out),
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

    async fn send_post_empty<T: serde::de::DeserializeOwned>(
        &self,
        path: &str,
    ) -> Result<T, SdkError> {
        let url = format!("{}{}", self.base_url.trim_end_matches('/'), path);
        let mut builder = self.http.post(&url);
        if let Some(k) = &self.api_key {
            builder = builder.bearer_auth(k);
        }
        let resp = builder.send().await?;
        let status = resp.status().as_u16();
        if (200..300).contains(&status) {
            return Ok(resp.json::<T>().await?);
        }
        let retry_after = parse_retry_after(resp.headers());
        let body = resp.text().await.unwrap_or_default();
        Err(SdkError::from_response(status, &body, retry_after))
    }

    async fn send_post_json<T, B>(&self, path: &str, body: &B) -> Result<T, SdkError>
    where
        T: serde::de::DeserializeOwned,
        B: serde::Serialize + ?Sized,
    {
        let url = format!("{}{}", self.base_url.trim_end_matches('/'), path);
        let mut builder = self.http.post(&url).json(body);
        if let Some(k) = &self.api_key {
            builder = builder.bearer_auth(k);
        }
        let resp = builder.send().await?;
        let status = resp.status().as_u16();
        if (200..300).contains(&status) {
            return Ok(resp.json::<T>().await?);
        }
        let retry_after = parse_retry_after(resp.headers());
        let body = resp.text().await.unwrap_or_default();
        Err(SdkError::from_response(status, &body, retry_after))
    }

    async fn send_patch_json<T, B>(&self, path: &str, body: &B) -> Result<T, SdkError>
    where
        T: serde::de::DeserializeOwned,
        B: serde::Serialize + ?Sized,
    {
        let url = format!("{}{}", self.base_url.trim_end_matches('/'), path);
        let mut builder = self.http.patch(&url).json(body);
        if let Some(k) = &self.api_key {
            builder = builder.bearer_auth(k);
        }
        let resp = builder.send().await?;
        let status = resp.status().as_u16();
        if (200..300).contains(&status) {
            return Ok(resp.json::<T>().await?);
        }
        let retry_after = parse_retry_after(resp.headers());
        let body = resp.text().await.unwrap_or_default();
        Err(SdkError::from_response(status, &body, retry_after))
    }

    async fn send_get<T: serde::de::DeserializeOwned>(&self, path: &str) -> Result<T, SdkError> {
        let url = format!("{}{}", self.base_url.trim_end_matches('/'), path);
        let mut builder = self.http.get(&url);
        if let Some(k) = &self.api_key {
            builder = builder.bearer_auth(k);
        }
        let resp = builder.send().await?;
        let status = resp.status().as_u16();
        if (200..300).contains(&status) {
            return Ok(resp.json::<T>().await?);
        }
        let retry_after = parse_retry_after(resp.headers());
        let body = resp.text().await.unwrap_or_default();
        Err(SdkError::from_response(status, &body, retry_after))
    }

    async fn send_once(&self, req: &CheckRequest) -> Result<Decision, SdkError> {
        let url = format!("{}/v1/check", self.base_url.trim_end_matches('/'));
        let mut builder = self.http.post(&url).json(req);
        if let Some(k) = &self.api_key {
            builder = builder.bearer_auth(k);
        }
        let resp = builder.send().await?;
        let status = resp.status().as_u16();
        if (200..300).contains(&status) {
            return Ok(resp.json::<Decision>().await?);
        }
        let retry_after = parse_retry_after(resp.headers());
        let body = resp.text().await.unwrap_or_default();
        Err(SdkError::from_response(status, &body, retry_after))
    }
}

fn parse_retry_after(headers: &reqwest::header::HeaderMap) -> Option<Duration> {
    headers
        .get(reqwest::header::RETRY_AFTER)
        .and_then(|v| v.to_str().ok())
        .and_then(|raw| raw.trim().parse::<u64>().ok())
        .map(Duration::from_secs)
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

#[cfg(test)]
mod tests {
    use super::*;
    use tl_core::ApiErrorCode;

    #[test]
    fn synthesize_unknown_body_uses_status_fallback() {
        let err = synthesize_api_error(503, "");
        assert_eq!(err.code, ApiErrorCode::Unavailable);
        assert!(err.retriable);
    }

    #[test]
    fn synthesize_400_is_not_retriable() {
        let err = synthesize_api_error(400, "bad input");
        assert_eq!(err.code, ApiErrorCode::Invalid);
        assert!(!err.retriable);
        assert_eq!(err.message, "bad input");
    }
}
