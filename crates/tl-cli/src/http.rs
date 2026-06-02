use anyhow::{anyhow, Context};
use tl_core::ApiError;

pub(super) fn server_url(url: Option<String>) -> String {
    url.or_else(|| std::env::var("TL_SERVER_URL").ok())
        .unwrap_or_else(|| "http://localhost:8080".to_string())
}

pub(super) fn resolve_api_key(api_key: Option<String>) -> Option<String> {
    api_key
        .or_else(|| std::env::var("TL_API_KEY").ok())
        .filter(|value| !value.trim().is_empty())
}

/// Minimal hand-rolled path-segment encoder. The CLI doesn't pull in a
/// URL crate just for this; agent ids are kebab-case in practice, and
/// this still keeps slashes/spaces safe.
pub(super) fn urlencode_id(id: &str) -> String {
    let mut out = String::with_capacity(id.len());
    for b in id.bytes() {
        match b {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                out.push(b as char);
            }
            _ => out.push_str(&format!("%{b:02X}")),
        }
    }
    out
}

pub(super) async fn decode_typed_response<T: serde::de::DeserializeOwned>(
    response: reqwest::Response,
    label: &str,
) -> anyhow::Result<T> {
    let status = response.status();
    let body = response.text().await.context("read response body")?;
    if status.is_success() {
        return serde_json::from_str(&body).with_context(|| format!("decode {label}"));
    }
    if let Ok(api_error) = serde_json::from_str::<ApiError>(&body) {
        return Err(anyhow!(
            "server returned {} ({:?}): {}",
            status,
            api_error.code,
            api_error.message
        ));
    }
    Err(anyhow!("server returned {status}: {body}"))
}
