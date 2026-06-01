use async_trait::async_trait;
use reqwest::header;
use serde_json::{json, Value};
use tl_core::{EnforcementProfile, GatewayProviderConnection};
use uuid::Uuid;

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

    fn inject_feedback(&self, request: &mut Value, reason: &str) {
        if let Some(messages) = request.get_mut("messages").and_then(Value::as_array_mut) {
            messages.push(json!({
                "role": "system",
                "content": format!(
                    "Your previous response violated policy: {reason}. Please revise to comply."
                )
            }));
        }
    }

    fn apply_output_rewrite(&self, response: Value, safe_output: &str) -> Value;
    fn safe_response(&self, request: &Value, profile: &EnforcementProfile) -> Value;
    async fn forward(
        &self,
        http: &reqwest::Client,
        connection: &GatewayProviderConnection,
        api_key: &str,
        request: Value,
    ) -> Result<Value, String>;
}

pub(super) struct OpenAiCompatibleGatewayProvider;

#[async_trait]
impl GatewayProvider for OpenAiCompatibleGatewayProvider {
    fn extract_output(&self, response: &Value) -> String {
        response
            .pointer("/choices/0/message/content")
            .and_then(Value::as_str)
            .unwrap_or("")
            .to_string()
    }

    fn apply_output_rewrite(&self, mut response: Value, safe_output: &str) -> Value {
        if let Some(content) = response.pointer_mut("/choices/0/message/content") {
            *content = json!(safe_output);
        }
        response
    }

    fn streaming_sse_body(&self, response: &Value) -> String {
        let id = response
            .get("id")
            .and_then(Value::as_str)
            .unwrap_or("chatcmpl_tlg_stream");
        let model = response
            .get("model")
            .cloned()
            .unwrap_or(json!("trustloopguard-gateway"));
        let created = response
            .get("created")
            .and_then(Value::as_i64)
            .unwrap_or_else(|| chrono::Utc::now().timestamp());
        let content = self.extract_output(response);
        let finish_reason = response
            .pointer("/choices/0/finish_reason")
            .and_then(Value::as_str)
            .unwrap_or("stop");

        let head = json!({
            "id": id,
            "object": "chat.completion.chunk",
            "created": created,
            "model": model,
            "choices": [{
                "index": 0,
                "delta": { "role": "assistant", "content": content },
                "finish_reason": Value::Null,
            }],
        });
        let tail = json!({
            "id": id,
            "object": "chat.completion.chunk",
            "created": created,
            "model": model,
            "choices": [{
                "index": 0,
                "delta": {},
                "finish_reason": finish_reason,
            }],
        });
        format!("data: {head}\n\ndata: {tail}\n\ndata: [DONE]\n\n")
    }

    fn safe_response(&self, request: &Value, profile: &EnforcementProfile) -> Value {
        json!({
            "id": format!("chatcmpl_tlg_{}", Uuid::now_v7()),
            "object": "chat.completion",
            "created": chrono::Utc::now().timestamp(),
            "model": request.get("model").cloned().unwrap_or_else(|| json!("trustloopguard-gateway")),
            "choices": [{
                "index": 0,
                "message": {
                    "role": "assistant",
                    "content": profile.fallback_message,
                },
                "finish_reason": "content_filter",
            }],
            "usage": { "prompt_tokens": 0, "completion_tokens": 0, "total_tokens": 0 },
        })
    }

    async fn forward(
        &self,
        http: &reqwest::Client,
        connection: &GatewayProviderConnection,
        api_key: &str,
        mut request: Value,
    ) -> Result<Value, String> {
        if request.get("model").is_none() {
            request["model"] = json!(connection.default_model);
        }
        let url = provider_url(connection, "https://api.openai.com", "/v1/chat/completions");
        let response = http
            .post(url)
            .bearer_auth(api_key)
            .json(&request)
            .send()
            .await
            .map_err(|e| format!("provider request failed: {e}"))?;
        provider_json_response(response).await
    }
}

pub(super) struct AnthropicGatewayProvider;

#[async_trait]
impl GatewayProvider for AnthropicGatewayProvider {
    fn extract_input(&self, request: &Value) -> String {
        let mut parts = Vec::new();
        if let Some(system) = request.get("system") {
            let system = message_content_text(system);
            if !system.is_empty() {
                parts.push(format!("system: {system}"));
            }
        }
        let message = latest_user_message_input_text(request);
        if !message.is_empty() {
            parts.push(message);
        }
        parts.join("\n")
    }

    fn extract_output(&self, response: &Value) -> String {
        response
            .get("content")
            .and_then(Value::as_array)
            .map(|content| {
                content
                    .iter()
                    .filter_map(|item| item.get("text").and_then(Value::as_str))
                    .collect::<Vec<_>>()
                    .join("\n")
            })
            .unwrap_or_default()
    }

    fn inject_feedback(&self, request: &mut Value, reason: &str) {
        if let Some(messages) = request.get_mut("messages").and_then(Value::as_array_mut) {
            messages.push(json!({
                "role": "user",
                "content": format!(
                    "Your previous response violated policy: {reason}. Please revise to comply."
                )
            }));
        }
    }

    fn apply_output_rewrite(&self, mut response: Value, safe_output: &str) -> Value {
        if let Some(text) = response.pointer_mut("/content/0/text") {
            *text = json!(safe_output);
        }
        response
    }

    fn streaming_sse_body(&self, response: &Value) -> String {
        let id = response
            .get("id")
            .and_then(Value::as_str)
            .unwrap_or("msg_tlg_stream");
        let model = response
            .get("model")
            .cloned()
            .unwrap_or(json!("trustloopguard-gateway"));
        let text = self.extract_output(response);
        let stop_reason = response
            .get("stop_reason")
            .cloned()
            .unwrap_or(json!("end_turn"));

        let events = [
            (
                "message_start",
                json!({
                    "type": "message_start",
                    "message": {
                        "id": id,
                        "type": "message",
                        "role": "assistant",
                        "model": model,
                        "content": [],
                        "stop_reason": Value::Null,
                        "stop_sequence": Value::Null,
                        "usage": { "input_tokens": 0, "output_tokens": 0 },
                    },
                }),
            ),
            (
                "content_block_start",
                json!({
                    "type": "content_block_start",
                    "index": 0,
                    "content_block": { "type": "text", "text": "" },
                }),
            ),
            (
                "content_block_delta",
                json!({
                    "type": "content_block_delta",
                    "index": 0,
                    "delta": { "type": "text_delta", "text": text },
                }),
            ),
            (
                "content_block_stop",
                json!({ "type": "content_block_stop", "index": 0 }),
            ),
            (
                "message_delta",
                json!({
                    "type": "message_delta",
                    "delta": { "stop_reason": stop_reason, "stop_sequence": Value::Null },
                    "usage": { "output_tokens": 0 },
                }),
            ),
            ("message_stop", json!({ "type": "message_stop" })),
        ];

        events
            .iter()
            .map(|(event, data)| format!("event: {event}\ndata: {data}\n\n"))
            .collect()
    }

    fn safe_response(&self, request: &Value, profile: &EnforcementProfile) -> Value {
        json!({
            "id": format!("msg_tlg_{}", Uuid::now_v7()),
            "type": "message",
            "role": "assistant",
            "model": request.get("model").cloned().unwrap_or_else(|| json!("trustloopguard-gateway")),
            "content": [{ "type": "text", "text": profile.fallback_message }],
            "stop_reason": "content_filter",
            "stop_sequence": null,
            "usage": { "input_tokens": 0, "output_tokens": 0 },
        })
    }

    async fn forward(
        &self,
        http: &reqwest::Client,
        connection: &GatewayProviderConnection,
        api_key: &str,
        mut request: Value,
    ) -> Result<Value, String> {
        if request.get("model").is_none() {
            request["model"] = json!(connection.default_model);
        }
        let url = provider_url(connection, "https://api.anthropic.com", "/v1/messages");
        let response = http
            .post(url)
            .header("x-api-key", api_key)
            .header("anthropic-version", "2023-06-01")
            .header(header::CONTENT_TYPE, "application/json")
            .json(&request)
            .send()
            .await
            .map_err(|e| format!("provider request failed: {e}"))?;
        provider_json_response(response).await
    }
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
