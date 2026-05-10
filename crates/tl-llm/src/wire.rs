//! Shared transport helper. Both `OpenAiClient` and `OpenRouterClient`
//! speak the same OpenAI v1 chat-completions wire shape; this module
//! holds the request build + response parse + timeout logic so the
//! provider modules stay tiny.

use std::time::Duration;

use serde_json::Value;
use tokio::time::timeout;

use crate::client::{LlmError, LlmOutput};

pub(crate) struct RequestParts<'a> {
    pub http: &'a reqwest::Client,
    pub url: String,
    pub api_key: &'a str,
    pub extra_headers: &'a [(&'a str, &'a str)],
    pub body: Value,
    pub deadline: Duration,
}

pub(crate) async fn call_chat_completions(parts: RequestParts<'_>) -> Result<LlmOutput, LlmError> {
    let mut builder = parts
        .http
        .post(&parts.url)
        .bearer_auth(parts.api_key)
        .json(&parts.body);
    for (k, v) in parts.extra_headers {
        builder = builder.header(*k, *v);
    }

    let send = builder.send();
    let resp = timeout(parts.deadline, send)
        .await
        .map_err(|_| LlmError::Timeout(parts.deadline))?
        .map_err(|e| LlmError::Http(e.to_string()))?;

    let status = resp.status();
    if !status.is_success() {
        let body = resp.text().await.unwrap_or_default();
        return Err(LlmError::Status(status.as_u16(), body));
    }

    let raw: Value = resp.json().await.map_err(|e| LlmError::Parse(e.to_string()))?;
    parse_chat_response(&raw)
}

/// Extract `LlmOutput` from a raw chat-completions response body. The
/// model returns its JSON output as a *string* in `choices[0].message.content`
/// — we parse that string into a `Value` so the caller gets structured JSON.
pub(crate) fn parse_chat_response(raw: &Value) -> Result<LlmOutput, LlmError> {
    let content = raw
        .pointer("/choices/0/message/content")
        .and_then(Value::as_str)
        .ok_or(LlmError::MissingField("choices[0].message.content"))?;

    let json: Value = serde_json::from_str(content).map_err(|e| LlmError::Parse(e.to_string()))?;

    let usage = raw.pointer("/usage").cloned().unwrap_or(Value::Null);
    let prompt_tokens = usage
        .pointer("/prompt_tokens")
        .and_then(Value::as_u64)
        .unwrap_or(0) as u32;
    let completion_tokens = usage
        .pointer("/completion_tokens")
        .and_then(Value::as_u64)
        .unwrap_or(0) as u32;

    Ok(LlmOutput {
        json,
        prompt_tokens,
        completion_tokens,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn parses_well_formed_response() {
        let raw = json!({
            "choices": [{
                "message": {
                    "content": "{\"verdict\":\"allow\",\"score\":0.92}"
                }
            }],
            "usage": { "prompt_tokens": 12, "completion_tokens": 7 }
        });
        let out = parse_chat_response(&raw).unwrap();
        assert_eq!(out.prompt_tokens, 12);
        assert_eq!(out.completion_tokens, 7);
        assert_eq!(out.json["verdict"], "allow");
        assert!((out.json["score"].as_f64().unwrap() - 0.92).abs() < 1e-6);
    }

    #[test]
    fn missing_content_yields_missing_field() {
        let raw = json!({ "choices": [{ "message": {} }], "usage": {} });
        let err = parse_chat_response(&raw).unwrap_err();
        assert!(matches!(err, LlmError::MissingField(_)));
    }

    #[test]
    fn malformed_inner_json_yields_parse_error() {
        let raw = json!({
            "choices": [{ "message": { "content": "not json" } }]
        });
        let err = parse_chat_response(&raw).unwrap_err();
        assert!(matches!(err, LlmError::Parse(_)));
    }

    #[test]
    fn missing_usage_defaults_to_zero() {
        let raw = json!({
            "choices": [{ "message": { "content": "{\"ok\":true}" } }]
        });
        let out = parse_chat_response(&raw).unwrap();
        assert_eq!(out.prompt_tokens, 0);
        assert_eq!(out.completion_tokens, 0);
    }
}
