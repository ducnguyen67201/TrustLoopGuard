mod anthropic;
mod openai;
mod payment;

use async_trait::async_trait;
use serde_json::{json, Value};
use std::time::Duration;
use tl_core::GatewayProviderConnection;

pub(super) use anthropic::AnthropicGatewayProvider;
pub(super) use openai::OpenAiCompatibleGatewayProvider;
pub(crate) use payment::forward_payment;

pub(super) const BLOCKED_MESSAGE: &str = "Blocked by Featherlane AI.";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum ProviderErrorClass {
    Transport,
    Timeout,
    RateLimited,
    Server,
    Client,
    InvalidResponse,
}

#[derive(Debug, Clone)]
pub(super) struct ProviderError {
    pub(super) class: ProviderErrorClass,
    pub(super) status: Option<u16>,
    pub(super) retry_after: Option<Duration>,
    pub(super) code: &'static str,
}

impl ProviderError {
    pub(super) fn transport(error: &reqwest::Error) -> Self {
        Self {
            class: if error.is_timeout() {
                ProviderErrorClass::Timeout
            } else {
                ProviderErrorClass::Transport
            },
            status: None,
            retry_after: None,
            code: if error.is_timeout() {
                "provider_timeout"
            } else {
                "provider_transport"
            },
        }
    }

    pub(super) const fn is_retryable(&self) -> bool {
        matches!(
            self.class,
            ProviderErrorClass::Transport
                | ProviderErrorClass::Timeout
                | ProviderErrorClass::RateLimited
                | ProviderErrorClass::Server
        )
    }
}

#[async_trait]
pub(super) trait GatewayProvider: Send + Sync {
    fn is_streaming(&self, request: &Value) -> bool {
        request
            .get("stream")
            .and_then(Value::as_bool)
            .unwrap_or(false)
    }

    fn strip_streaming_fields(&self, request: &mut Value) {
        if let Some(obj) = request.as_object_mut() {
            obj.remove("stream");
            obj.remove("stream_options");
        }
    }

    fn extract_input(&self, request: &Value) -> String {
        latest_user_message_input_text(request)
    }

    fn extract_output(&self, response: &Value) -> String;
    fn streaming_sse_body(&self, response: &Value) -> String;

    fn apply_input_rewrite(&self, request: &mut Value, safe_input: &str) {
        if let Some(messages) = request.get_mut("messages").and_then(Value::as_array_mut) {
            if let Some(last) = messages.iter_mut().rev().find(|message| {
                message
                    .get("role")
                    .and_then(Value::as_str)
                    .map(|role| role == "user")
                    .unwrap_or(false)
            }) {
                last["content"] = json!(safe_input);
            }
        }
    }

    fn apply_output_rewrite(&self, response: Value, safe_output: &str) -> Value;
    fn blocked_response(&self, request: &Value) -> Value;
    async fn forward(
        &self,
        http: &reqwest::Client,
        connection: &GatewayProviderConnection,
        api_key: &str,
        request: Value,
    ) -> Result<Value, ProviderError>;
}

fn message_content_text(value: &Value) -> String {
    match value {
        Value::String(text) => text.clone(),
        Value::Array(items) => items
            .iter()
            .filter_map(|item| {
                item.get("text")
                    .and_then(Value::as_str)
                    .or_else(|| item.get("content").and_then(Value::as_str))
            })
            .collect::<Vec<_>>()
            .join("\n"),
        _ => String::new(),
    }
}

fn latest_user_message_input_text(request: &Value) -> String {
    latest_user_message_content(request)
        .map(|content| format!("user: {content}"))
        .unwrap_or_default()
}

pub(super) fn latest_user_message_content(request: &Value) -> Option<String> {
    request
        .get("messages")
        .and_then(Value::as_array)
        .and_then(|messages| {
            messages.iter().rev().find_map(|message| {
                let role = message.get("role").and_then(Value::as_str).unwrap_or("");
                if role != "user" {
                    return None;
                }
                let content = message_content_text(message.get("content")?);
                let content = content.trim();
                if content.is_empty() {
                    None
                } else {
                    Some(content.to_string())
                }
            })
        })
}

fn provider_url(connection: &GatewayProviderConnection, default_base: &str, path: &str) -> String {
    let base = connection
        .base_url
        .as_deref()
        .unwrap_or(default_base)
        .trim_end_matches('/');
    if base.ends_with(path.trim_start_matches('/')) {
        base.to_string()
    } else {
        format!("{base}{path}")
    }
}

pub(super) async fn provider_json_response(
    response: reqwest::Response,
) -> Result<Value, ProviderError> {
    let status = response.status();
    let retry_after = parse_retry_after(response.headers().get(reqwest::header::RETRY_AFTER));
    if !status.is_success() {
        let body_bytes = response.bytes().await.map_or(0, |body| body.len());
        tracing::warn!(
            status = status.as_u16(),
            body_bytes,
            "upstream provider returned error"
        );
        let class = if status.as_u16() == 429 {
            ProviderErrorClass::RateLimited
        } else if status.is_server_error() || status.as_u16() == 408 {
            ProviderErrorClass::Server
        } else {
            ProviderErrorClass::Client
        };
        return Err(ProviderError {
            class,
            status: Some(status.as_u16()),
            retry_after,
            code: match class {
                ProviderErrorClass::RateLimited => "provider_rate_limited",
                ProviderErrorClass::Server => "provider_unavailable",
                _ => "provider_rejected_request",
            },
        });
    }
    response.json::<Value>().await.map_err(|_| ProviderError {
        class: ProviderErrorClass::InvalidResponse,
        status: Some(status.as_u16()),
        retry_after: None,
        code: "provider_invalid_json",
    })
}

fn parse_retry_after(value: Option<&reqwest::header::HeaderValue>) -> Option<Duration> {
    let value = value?.to_str().ok()?.trim();
    let delay = if let Ok(seconds) = value.parse::<u64>() {
        Duration::from_secs(seconds)
    } else {
        let at = chrono::DateTime::parse_from_rfc2822(value)
            .ok()?
            .with_timezone(&chrono::Utc);
        let milliseconds = (at - chrono::Utc::now()).num_milliseconds().max(0) as u64;
        Duration::from_millis(milliseconds)
    };
    Some(delay.min(Duration::from_secs(30)))
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use reqwest::header::HeaderValue;

    use super::{parse_retry_after, ProviderError, ProviderErrorClass};

    #[test]
    fn retry_classifier_is_bounded_to_temporary_failures() {
        for class in [
            ProviderErrorClass::Transport,
            ProviderErrorClass::Timeout,
            ProviderErrorClass::RateLimited,
            ProviderErrorClass::Server,
        ] {
            assert!(ProviderError {
                class,
                status: None,
                retry_after: None,
                code: "temporary",
            }
            .is_retryable());
        }
        assert!(!ProviderError {
            class: ProviderErrorClass::Client,
            status: Some(401),
            retry_after: None,
            code: "credential",
        }
        .is_retryable());
        assert!(!ProviderError {
            class: ProviderErrorClass::InvalidResponse,
            status: Some(200),
            retry_after: None,
            code: "invalid",
        }
        .is_retryable());
    }

    #[test]
    fn retry_after_seconds_and_http_dates_are_capped() {
        assert_eq!(
            parse_retry_after(Some(&HeaderValue::from_static("2"))),
            Some(Duration::from_secs(2))
        );
        assert_eq!(
            parse_retry_after(Some(&HeaderValue::from_static("120"))),
            Some(Duration::from_secs(30))
        );
        assert_eq!(
            parse_retry_after(Some(&HeaderValue::from_static(
                "Wed, 21 Oct 2037 07:28:00 GMT",
            ))),
            Some(Duration::from_secs(30))
        );
        assert_eq!(
            parse_retry_after(Some(&HeaderValue::from_static("invalid"))),
            None
        );
    }
}
