use async_trait::async_trait;
use reqwest::header;
use serde_json::{json, Value};
use tl_core::GatewayProviderConnection;
use uuid::Uuid;

use super::{
    latest_user_message_input_text, message_content_text, provider_json_response, provider_url,
    GatewayProvider, BLOCKED_MESSAGE,
};

pub(in crate::gateway) struct AnthropicGatewayProvider;

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

    fn blocked_response(&self, request: &Value) -> Value {
        json!({
            "id": format!("msg_tlg_{}", Uuid::now_v7()),
            "type": "message",
            "role": "assistant",
            "model": request.get("model").cloned().unwrap_or_else(|| json!("trustloopguard-gateway")),
            "content": [{ "type": "text", "text": BLOCKED_MESSAGE }],
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
