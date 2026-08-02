use async_trait::async_trait;
use serde_json::{json, Value};
use tl_core::GatewayProviderConnection;
use uuid::Uuid;

use super::{provider_json_response, provider_url, GatewayProvider, BLOCKED_MESSAGE};

pub(in crate::gateway) struct OpenAiCompatibleGatewayProvider;

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
            .unwrap_or("chatcmpl_featherlane_ai_stream");
        let model = response
            .get("model")
            .cloned()
            .unwrap_or(json!("featherlane-ai-gateway"));
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

    fn blocked_response(&self, request: &Value) -> Value {
        json!({
            "id": format!("chatcmpl_featherlane_ai_{}", Uuid::now_v7()),
            "object": "chat.completion",
            "created": chrono::Utc::now().timestamp(),
            "model": request.get("model").cloned().unwrap_or_else(|| json!("featherlane-ai-gateway")),
            "choices": [{
                "index": 0,
                "message": {
                    "role": "assistant",
                    "content": BLOCKED_MESSAGE,
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
