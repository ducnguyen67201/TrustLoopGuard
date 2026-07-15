use std::sync::Arc;

use axum::{
    extract::{Path, State},
    http::{HeaderMap, StatusCode},
    response::{IntoResponse, Response},
    Json,
};
#[allow(unused_imports)]
use tl_core::ApiError;
use tl_core::{ApiErrorCode, GuardrailGenerateResponse, GuardrailListResponse};
use tl_llm::LlmClient;

use crate::agents::{AgentStore, AgentStoreError};
use crate::environments::EnvironmentStore;

use super::draft::{
    parse_policy_set, policy_from_draft, policy_set_draft_json_schema,
    POLICY_SET_DRAFT_SYSTEM_PROMPT,
};
use super::{
    api_error_response, policy_store_error_response, workspace_id_from_headers, PolicyStore,
};

/// State for the `/v1/agents/{id}/guardrails*` endpoints. Carries both
/// stores plus the LLM client used to derive the policy set.
#[derive(Clone)]
pub struct GuardrailState {
    pub agent_store: Arc<dyn AgentStore>,
    pub policy_store: Arc<dyn PolicyStore>,
    pub environment_store: Arc<dyn EnvironmentStore>,
    pub draft_llm: Option<Arc<dyn LlmClient>>,
    pub draft_model: String,
}

/// `POST /v1/agents/{id}/guardrails/generate` — derive a set of
/// guardrail policies tailored to an agent's stored `system_prompt`,
/// auto-persist them with `enabled=false`, and return what was saved.
///
/// Callers review the set and flip individual policies on via
/// `PATCH /v1/policies/{id}/enabled`. Runtime checks never see these
/// policies until that happens (because runtime event evaluation filters by enabled).
#[utoipa::path(
    post,
    path = "/v1/agents/{id}/guardrails/generate",
    tag = "agents",
    params(("id" = String, Path, description = "Agent identifier")),
    responses(
        (status = 200, description = "Generated and persisted policies", body = GuardrailGenerateResponse),
        (status = 401, description = "Missing or invalid API key", body = ApiError),
        (status = 404, description = "Agent not registered", body = ApiError),
        (status = 422, description = "Agent has no system_prompt to derive guardrails from", body = ApiError),
        (status = 502, description = "LLM provider failed or returned invalid shape", body = ApiError),
        (status = 503, description = "LLM is not configured on this deployment", body = ApiError),
    ),
)]
pub async fn generate_guardrails(
    State(state): State<GuardrailState>,
    headers: HeaderMap,
    Path(agent_id): Path<String>,
) -> Response {
    let workspace_id = match workspace_id_from_headers(&headers) {
        Ok(workspace_id) => workspace_id,
        Err(response) => return response,
    };
    let environment_id = match resolve_environment_id(&state, &headers, &workspace_id).await {
        Ok(environment_id) => environment_id,
        Err(response) => return response,
    };
    let agent = match state.agent_store.get(&workspace_id, &agent_id).await {
        Ok(agent) => agent,
        Err(AgentStoreError::NotFound) => {
            return api_error_response(
                StatusCode::NOT_FOUND,
                ApiErrorCode::NotFound,
                format!("agent `{agent_id}` not found"),
            );
        }
        Err(e) => {
            return api_error_response(
                StatusCode::INTERNAL_SERVER_ERROR,
                ApiErrorCode::Internal,
                e.to_string(),
            );
        }
    };

    let prompt = match agent.system_prompt.as_deref().map(str::trim) {
        Some(p) if !p.is_empty() => p.to_string(),
        _ => {
            return api_error_response(
                StatusCode::UNPROCESSABLE_ENTITY,
                ApiErrorCode::Unprocessable,
                format!(
                    "agent `{agent_id}` has no system_prompt — set it via POST /v1/agents \
                     before generating guardrails."
                ),
            );
        }
    };

    let Some(client) = state.draft_llm.clone() else {
        return api_error_response(
            StatusCode::SERVICE_UNAVAILABLE,
            ApiErrorCode::Unavailable,
            "guardrail generation is not configured on this deployment (no LLM key)".into(),
        );
    };

    let composed = format!("{POLICY_SET_DRAFT_SYSTEM_PROMPT}\nAgent system prompt:\n{prompt}");
    let schema = policy_set_draft_json_schema();
    let out = match client
        .complete(
            &state.draft_model,
            &composed,
            &schema,
            std::time::Duration::from_secs(60),
        )
        .await
    {
        Ok(out) => out,
        Err(e) => {
            return api_error_response(
                StatusCode::BAD_GATEWAY,
                ApiErrorCode::Unavailable,
                format!("llm provider error: {e}"),
            );
        }
    };

    let drafts = match parse_policy_set(out.json) {
        Ok(drafts) => drafts,
        Err(message) => {
            return api_error_response(StatusCode::BAD_GATEWAY, ApiErrorCode::Internal, message);
        }
    };

    let mut persisted = Vec::with_capacity(drafts.len());
    let mut seen_ids = std::collections::HashSet::new();
    for draft in drafts {
        // De-dupe at the response boundary even if the model misbehaved.
        if !seen_ids.insert(draft.id.clone()) {
            continue;
        }
        let policy = policy_from_draft(&draft, &agent_id);
        if let Err(issues) = tl_policy::validate_policy(&policy) {
            // One bad draft shouldn't sink the whole batch — skip and
            // keep the rest. Logged so operators notice.
            tracing::warn!(
                draft_id = %policy.id,
                agent_id = %agent_id,
                issues = ?issues,
                "skipping LLM-drafted policy that failed validation"
            );
            continue;
        }
        let source_yaml = match serde_yaml::to_string(&policy) {
            Ok(yaml) => yaml,
            Err(e) => {
                return api_error_response(
                    StatusCode::INTERNAL_SERVER_ERROR,
                    ApiErrorCode::Internal,
                    format!("policy yaml render: {e}"),
                );
            }
        };
        match state
            .policy_store
            .upsert(&workspace_id, &environment_id, &policy, &source_yaml)
            .await
        {
            Ok(_) => {}
            Err(e) => return policy_store_error_response(e),
        }
        // Auto-persist starts disabled — runtime event evaluation ignores it
        // until an operator opts in via PATCH /v1/policies/{id}/enabled.
        match state
            .policy_store
            .set_enabled(&workspace_id, &environment_id, &policy.id, false)
            .await
        {
            Ok(document) => persisted.push(document),
            Err(e) => return policy_store_error_response(e),
        }
    }

    Json(GuardrailGenerateResponse {
        generated: persisted,
    })
    .into_response()
}

/// `GET /v1/agents/{id}/guardrails` — list active policies owned by an
/// agent. Does not require the agent to exist; missing agent → empty list.
#[utoipa::path(
    get,
    path = "/v1/agents/{id}/guardrails",
    tag = "agents",
    params(("id" = String, Path, description = "Agent identifier")),
    responses(
        (status = 200, description = "Policies owned by the agent", body = GuardrailListResponse),
        (status = 401, description = "Missing or invalid API key", body = ApiError),
    ),
)]
pub async fn list_guardrails(
    State(state): State<GuardrailState>,
    headers: HeaderMap,
    Path(agent_id): Path<String>,
) -> Response {
    let workspace_id = match workspace_id_from_headers(&headers) {
        Ok(workspace_id) => workspace_id,
        Err(response) => return response,
    };
    let environment_id = match resolve_environment_id(&state, &headers, &workspace_id).await {
        Ok(environment_id) => environment_id,
        Err(response) => return response,
    };
    match state
        .policy_store
        .list_for_agent(&workspace_id, &environment_id, &agent_id)
        .await
    {
        Ok(policies) => Json(GuardrailListResponse { policies }).into_response(),
        Err(e) => policy_store_error_response(e),
    }
}

async fn resolve_environment_id(
    state: &GuardrailState,
    headers: &HeaderMap,
    workspace_id: &str,
) -> Result<String, Response> {
    crate::environments::resolve_environment_id(
        headers,
        state.environment_store.as_ref(),
        workspace_id,
    )
    .await
    .map_err(crate::environments::environment_error_response)
}
