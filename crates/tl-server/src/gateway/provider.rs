mod anthropic;
mod openai;
mod payment;

use async_trait::async_trait;
use serde_json::{json, Value};
use tl_core::GatewayProviderConnection;

pub(super) use anthropic::AnthropicGatewayProvider;
pub(super) use openai::OpenAiCompatibleGatewayProvider;
pub(crate) use payment::forward_payment;

pub(super) const BLOCKED_MESSAGE: &str = "Blocked by Featherlane AI.";

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
    ) -> Result<Value, String>;
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

pub(super) async fn provider_json_response(response: reqwest::Response) -> Result<Value, String> {
    let status = response.status();
    if !status.is_success() {
        let body = response.text().await.unwrap_or_default();
        tracing::warn!(status = status.as_u16(), body = %body, "upstream provider returned error");
        return Err(format!("provider returned status {}", status.as_u16()));
    }
    response
        .json::<Value>()
        .await
        .map_err(|e| format!("provider response must be JSON: {e}"))
}
