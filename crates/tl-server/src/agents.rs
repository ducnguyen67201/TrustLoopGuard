//! Agent profile CRUD endpoints + storage abstraction.
//!
//! The endpoints live behind the bearer-auth layer (PR 13) and accept
//! either YAML (`application/yaml`, `application/x-yaml`, `text/yaml`)
//! or JSON bodies. YAML is the canonical format authors use; JSON is
//! the SDK-friendly form.
//!
//! `AgentStore` is a small trait so the server can run without Postgres in
//! tests and local dev. PR 15 plugs in `tl_storage::AgentRepo` (the
//! Postgres-backed impl) via an adapter.

use std::sync::Arc;

use async_trait::async_trait;
use axum::{
    extract::{Path, State},
    http::{HeaderMap, StatusCode},
    response::{IntoResponse, Response},
    Json,
};
#[allow(unused_imports)]
use tl_core::ApiError;
use tl_core::{AgentListResponse, AgentProfile, ApiErrorCode};

mod memory_store;
mod response;
#[cfg(test)]
mod tests;
mod validation;

pub use memory_store::MemoryAgentStore;
use response::api_error_response;
use validation::{parse_body, validate_profile};

#[derive(Debug, thiserror::Error)]
pub enum AgentStoreError {
    #[error("not found")]
    NotFound,
    #[error("validation: {0}")]
    Validation(String),
    #[error("internal: {0}")]
    Internal(String),
}

/// Minimal write/read surface the endpoints need from an agent store.
/// Concrete impls: `MemoryAgentStore` (in this module) and an adapter
/// over `tl_storage::AgentRepo` (lands in PR 15).
#[async_trait]
pub trait AgentStore: Send + Sync {
    async fn upsert(
        &self,
        workspace_id: &str,
        profile: &AgentProfile,
        source_yaml: &str,
    ) -> Result<(), AgentStoreError>;
    async fn get(
        &self,
        workspace_id: &str,
        agent_id: &str,
    ) -> Result<Arc<AgentProfile>, AgentStoreError>;
    async fn delete(&self, workspace_id: &str, agent_id: &str) -> Result<(), AgentStoreError>;
    async fn list(&self, workspace_id: &str) -> Result<Vec<Arc<AgentProfile>>, AgentStoreError>;
}

// -- Endpoint handlers ----------------------------------------------------

/// Shared state used by the agent endpoints.
#[derive(Clone)]
pub struct AgentState {
    pub store: Arc<dyn AgentStore>,
    /// Policy store used by `DELETE /v1/agents/{id}` to cascade-soft-delete
    /// policies owned by the agent. `None` skips the cascade — only valid
    /// in tests / boot paths where guardrail features aren't wired.
    pub policy_store: Option<Arc<dyn crate::policies::PolicyStore>>,
}

/// `POST /v1/agents` — upsert a profile. Body is YAML (when
/// `Content-Type` is `*yaml*`) or JSON.
#[utoipa::path(
    post,
    path = "/v1/agents",
    tag = "agents",
    request_body(
        description = "Agent profile, YAML or JSON",
        content_type = "application/yaml",
        content = AgentProfile,
    ),
    responses(
        (status = 201, description = "Profile created or updated", body = AgentProfile),
        (status = 400, description = "Malformed request body", body = ApiError),
        (status = 401, description = "Missing or invalid API key", body = ApiError),
        (status = 422, description = "Profile failed validation", body = ApiError),
    ),
)]
pub async fn upsert_agent(
    State(state): State<AgentState>,
    headers: HeaderMap,
    body: bytes::Bytes,
) -> Response {
    let profile_and_source = match parse_body(&headers, &body) {
        Ok(p) => p,
        Err(e) => return e.into_response(),
    };
    let (profile, source) = profile_and_source;
    let workspace_id = crate::policies::workspace_id_from_headers(&headers);

    if let Err(msg) = validate_profile(&profile) {
        return api_error_response(
            StatusCode::UNPROCESSABLE_ENTITY,
            ApiErrorCode::Unprocessable,
            msg,
        );
    }

    match state.store.upsert(&workspace_id, &profile, &source).await {
        Ok(()) => (StatusCode::CREATED, Json(profile)).into_response(),
        Err(AgentStoreError::Validation(m)) => api_error_response(
            StatusCode::UNPROCESSABLE_ENTITY,
            ApiErrorCode::Unprocessable,
            m,
        ),
        Err(e) => api_error_response(
            StatusCode::INTERNAL_SERVER_ERROR,
            ApiErrorCode::Internal,
            e.to_string(),
        ),
    }
}

/// `GET /v1/agents/:id`.
#[utoipa::path(
    get,
    path = "/v1/agents/{id}",
    tag = "agents",
    params(("id" = String, Path, description = "Agent identifier")),
    responses(
        (status = 200, description = "Profile found", body = AgentProfile),
        (status = 401, description = "Missing or invalid API key", body = ApiError),
        (status = 404, description = "Agent not registered", body = ApiError),
    ),
)]
pub async fn get_agent(
    State(state): State<AgentState>,
    headers: HeaderMap,
    Path(id): Path<String>,
) -> Response {
    let workspace_id = crate::policies::workspace_id_from_headers(&headers);
    match state.store.get(&workspace_id, &id).await {
        Ok(profile) => Json(profile.as_ref().clone()).into_response(),
        Err(AgentStoreError::NotFound) => api_error_response(
            StatusCode::NOT_FOUND,
            ApiErrorCode::NotFound,
            format!("agent `{id}` not found"),
        ),
        Err(e) => api_error_response(
            StatusCode::INTERNAL_SERVER_ERROR,
            ApiErrorCode::Internal,
            e.to_string(),
        ),
    }
}

/// `DELETE /v1/agents/:id`. Soft-delete via the store.
#[utoipa::path(
    delete,
    path = "/v1/agents/{id}",
    tag = "agents",
    params(("id" = String, Path, description = "Agent identifier")),
    responses(
        (status = 204, description = "Profile deleted"),
        (status = 401, description = "Missing or invalid API key", body = ApiError),
        (status = 404, description = "Agent not registered", body = ApiError),
    ),
)]
pub async fn delete_agent(
    State(state): State<AgentState>,
    headers: HeaderMap,
    Path(id): Path<String>,
) -> Response {
    let workspace_id = crate::policies::workspace_id_from_headers(&headers);
    // Soft-delete owned policies first. If this fails, leave the agent
    // intact — we'd rather refuse the request than orphan an agent
    // whose generated guardrails are still active. The two-step path
    // is not transactional across stores; the index on policies'
    // owner_agent_id keeps the cleanup window small in practice and
    // the operation is idempotent on retry.
    if let Some(policies) = state.policy_store.as_ref() {
        if let Err(e) = policies.delete_for_agent(&workspace_id, &id).await {
            return api_error_response(
                StatusCode::INTERNAL_SERVER_ERROR,
                ApiErrorCode::Internal,
                format!("cascade delete policies for agent `{id}`: {e}"),
            );
        }
    }
    match state.store.delete(&workspace_id, &id).await {
        Ok(()) => StatusCode::NO_CONTENT.into_response(),
        Err(AgentStoreError::NotFound) => api_error_response(
            StatusCode::NOT_FOUND,
            ApiErrorCode::NotFound,
            format!("agent `{id}` not found"),
        ),
        Err(e) => api_error_response(
            StatusCode::INTERNAL_SERVER_ERROR,
            ApiErrorCode::Internal,
            e.to_string(),
        ),
    }
}

/// `GET /v1/agents`. Returns all registered profiles. Bypasses the
/// store's read cache because admin-list isn't on the hot path.
#[utoipa::path(
    get,
    path = "/v1/agents",
    tag = "agents",
    responses(
        (status = 200, description = "All agents", body = AgentListResponse),
        (status = 401, description = "Missing or invalid API key", body = ApiError),
    ),
)]
pub async fn list_agents(State(state): State<AgentState>, headers: HeaderMap) -> Response {
    let workspace_id = crate::policies::workspace_id_from_headers(&headers);
    match state.store.list(&workspace_id).await {
        Ok(arcs) => {
            let agents = arcs.iter().map(|a| (**a).clone()).collect();
            Json(AgentListResponse { agents }).into_response()
        }
        Err(e) => api_error_response(
            StatusCode::INTERNAL_SERVER_ERROR,
            ApiErrorCode::Internal,
            e.to_string(),
        ),
    }
}
