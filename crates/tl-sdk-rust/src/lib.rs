//! TrustLoopGuard Rust SDK. Thin async client over reqwest.

use tl_core::{CheckRequest, Decision};

#[derive(Debug, thiserror::Error)]
pub enum SdkError {
    #[error("http: {0}")]
    Http(#[from] reqwest::Error),
    #[error("server returned status {0}")]
    Status(u16),
}

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
        if !resp.status().is_success() {
            return Err(SdkError::Status(resp.status().as_u16()));
        }
        Ok(resp.json::<Decision>().await?)
    }
}
