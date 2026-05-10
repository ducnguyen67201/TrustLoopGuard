//! TrustLoopGuard Rust SDK. Thin async client over reqwest.
//!
//! Errors map server responses into typed variants (see [`SdkError`]) so
//! callers can branch on failure modes without inspecting status codes.
//! Retries, auth wiring, and tracing land in subsequent PRs in the
//! SDK-driven stack — this PR is error taxonomy only.

use std::time::Duration;

use tl_core::{ApiError, ApiErrorCode, CheckRequest, Decision};

mod error;

pub use error::SdkError;

#[derive(Debug, Clone)]
pub struct Client {
    base_url: String,
    api_key: Option<String>,
    http: reqwest::Client,
}

impl Client {
    pub fn new(base_url: impl Into<String>) -> Self {
        Self {
            base_url: base_url.into(),
            api_key: None,
            http: reqwest::Client::new(),
        }
    }

    pub fn with_api_key(mut self, key: impl Into<String>) -> Self {
        self.api_key = Some(key.into());
        self
    }

    pub async fn check(&self, req: &CheckRequest) -> Result<Decision, SdkError> {
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

/// Internal: synthesize an `ApiError` from a raw status when the server
/// did not return our canonical error body. Public to the crate so the
/// error module can reuse it; not part of the published surface.
pub(crate) fn synthesize_api_error(status: u16, body: &str) -> ApiError {
    let code = ApiErrorCode::from_http_status(status);
    ApiError {
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
