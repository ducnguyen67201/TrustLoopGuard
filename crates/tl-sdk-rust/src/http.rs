use std::time::{Duration, Instant};

use tracing::{warn, Span};

use crate::{Client, SdkError};

impl Client {
    /// Shared retry harness for endpoints other than `check()`. Keeps
    /// retry semantics identical without dragging endpoint-specific
    /// request types into the helper signature.
    pub(crate) async fn retry_loop<T, F, Fut>(
        &self,
        _path: &str,
        mut send: F,
    ) -> Result<T, SdkError>
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

    pub(crate) async fn send_post_empty<T: serde::de::DeserializeOwned>(
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

    pub(crate) async fn send_post_json<T, B>(&self, path: &str, body: &B) -> Result<T, SdkError>
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

    pub(crate) async fn send_patch_json<T, B>(&self, path: &str, body: &B) -> Result<T, SdkError>
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

    pub(crate) async fn send_get<T: serde::de::DeserializeOwned>(
        &self,
        path: &str,
    ) -> Result<T, SdkError> {
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
}

fn parse_retry_after(headers: &reqwest::header::HeaderMap) -> Option<Duration> {
    headers
        .get(reqwest::header::RETRY_AFTER)
        .and_then(|v| v.to_str().ok())
        .and_then(|raw| raw.trim().parse::<u64>().ok())
        .map(Duration::from_secs)
}
