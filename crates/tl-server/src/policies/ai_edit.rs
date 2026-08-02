use axum::{
    extract::State,
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};
use tl_core::{AiEditRequest, AiEditResponse, ApiErrorCode};

use super::{api_error_response, PolicyState};

const AI_EDIT_SYSTEM_PROMPT: &str = concat!(
    "You are a Featherlane AI policy YAML editor. ",
    "Given the current policy YAML and an instruction, apply the instruction and return ",
    "ONLY the modified YAML — no explanation, no markdown fences, no surrounding text. ",
    "Preserve all unmodified fields exactly. ",
    "Valid fields: id, description, match (literal or regex), action, severity, rewrite, when.",
);

/// `POST /v1/policies/ai-edit` — apply a natural-language instruction to existing
/// policy YAML via LLM and return the modified YAML. Stateless; the caller decides
/// whether to save the result via the normal upsert endpoint.
pub async fn ai_edit_policy(State(state): State<PolicyState>, body: bytes::Bytes) -> Response {
    let req: AiEditRequest = match serde_json::from_slice(&body) {
        Ok(r) => r,
        Err(e) => {
            return api_error_response(
                StatusCode::BAD_REQUEST,
                ApiErrorCode::Invalid,
                format!("request body is not valid JSON: {e}"),
            );
        }
    };
    if req.yaml.trim().is_empty() || req.instruction.trim().is_empty() {
        return api_error_response(
            StatusCode::BAD_REQUEST,
            ApiErrorCode::Invalid,
            "yaml and instruction are required".into(),
        );
    }

    let Some(client) = state.draft_llm.clone() else {
        return api_error_response(
            StatusCode::SERVICE_UNAVAILABLE,
            ApiErrorCode::Unavailable,
            "AI editing is not configured on this deployment (no LLM key)".into(),
        );
    };

    let user_prompt = format!(
        "Current YAML:\n{}\n\nInstruction: {}",
        req.yaml.trim(),
        req.instruction.trim(),
    );

    // Use a simple text-return schema so the model returns raw YAML.
    let schema = tl_llm::JsonSchema {
        name: "yaml_edit_result".to_string(),
        schema: serde_json::json!({
            "type": "object",
            "additionalProperties": false,
            "required": ["yaml"],
            "properties": {
                "yaml": { "type": "string" }
            }
        }),
    };

    let out = match client
        .complete(
            &state.draft_model,
            &format!("{AI_EDIT_SYSTEM_PROMPT}\n\n{user_prompt}"),
            &schema,
            std::time::Duration::from_secs(30),
        )
        .await
    {
        Ok(out) => out,
        Err(e) => {
            return api_error_response(
                StatusCode::BAD_GATEWAY,
                ApiErrorCode::Unavailable,
                format!("LLM provider error: {e}"),
            );
        }
    };

    let yaml = match out.json.get("yaml").and_then(|v| v.as_str()) {
        Some(s) => {
            // Strip markdown fences if the model ignored the strict-mode schema.
            let stripped = s
                .trim()
                .trim_start_matches("```yaml")
                .trim_start_matches("```")
                .trim_end_matches("```")
                .trim();
            stripped.to_string()
        }
        None => {
            return api_error_response(
                StatusCode::BAD_GATEWAY,
                ApiErrorCode::Internal,
                "model returned unexpected shape".into(),
            );
        }
    };

    Json(AiEditResponse { yaml }).into_response()
}
