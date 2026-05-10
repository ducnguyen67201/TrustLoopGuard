//! Agent profile CRUD endpoints + storage abstraction.
//!
//! The endpoints live behind the bearer-auth layer (PR 13) and accept
//! either YAML (`application/yaml`, `application/x-yaml`, `text/yaml`)
//! or JSON bodies. YAML is the canonical format authors use; JSON is
//! the SDK-friendly form.
//!
//! `AgentStore` is a small trait so the server can run without sqlx in
//! tests and local dev. PR 15 plugs in `tl_storage::AgentRepo` (the
//! Postgres-backed impl) via an adapter.

use std::sync::Arc;

use async_trait::async_trait;
use axum::{
    extract::{Path, State},
    http::{header, HeaderMap, StatusCode},
    response::{IntoResponse, Response},
    Json,
};
use serde::Serialize;
use serde_json::json;
use tl_core::{AgentProfile, ApiError, ApiErrorCode};
use tokio::sync::RwLock;
use utoipa::ToSchema;

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
        profile: &AgentProfile,
        source_yaml: &str,
    ) -> Result<(), AgentStoreError>;
    async fn get(&self, agent_id: &str) -> Result<Arc<AgentProfile>, AgentStoreError>;
    async fn delete(&self, agent_id: &str) -> Result<(), AgentStoreError>;
    async fn list(&self) -> Result<Vec<Arc<AgentProfile>>, AgentStoreError>;
}

/// Process-local agent store. Useful for local dev, tests, and the
/// "no database configured" boot path. Not durable across restarts.
#[derive(Debug, Default)]
pub struct MemoryAgentStore {
    inner: RwLock<std::collections::HashMap<String, Arc<AgentProfile>>>,
}

impl MemoryAgentStore {
    pub fn new() -> Self {
        Self::default()
    }
}

#[async_trait]
impl AgentStore for MemoryAgentStore {
    async fn upsert(
        &self,
        profile: &AgentProfile,
        _source_yaml: &str,
    ) -> Result<(), AgentStoreError> {
        self.inner
            .write()
            .await
            .insert(profile.agent_id.clone(), Arc::new(profile.clone()));
        Ok(())
    }

    async fn get(&self, agent_id: &str) -> Result<Arc<AgentProfile>, AgentStoreError> {
        self.inner
            .read()
            .await
            .get(agent_id)
            .cloned()
            .ok_or(AgentStoreError::NotFound)
    }

    async fn delete(&self, agent_id: &str) -> Result<(), AgentStoreError> {
        if self.inner.write().await.remove(agent_id).is_none() {
            return Err(AgentStoreError::NotFound);
        }
        Ok(())
    }

    async fn list(&self) -> Result<Vec<Arc<AgentProfile>>, AgentStoreError> {
        let mut all: Vec<_> = self.inner.read().await.values().cloned().collect();
        all.sort_by(|a, b| a.agent_id.cmp(&b.agent_id));
        Ok(all)
    }
}

// -- Endpoint handlers ----------------------------------------------------

/// Shared state used by the agent endpoints.
#[derive(Clone)]
pub struct AgentState {
    pub store: Arc<dyn AgentStore>,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct AgentListResponse {
    pub agents: Vec<AgentProfile>,
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

    if let Err(msg) = validate_profile(&profile) {
        return api_error_response(
            StatusCode::UNPROCESSABLE_ENTITY,
            ApiErrorCode::Unprocessable,
            msg,
        );
    }

    match state.store.upsert(&profile, &source).await {
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
pub async fn get_agent(State(state): State<AgentState>, Path(id): Path<String>) -> Response {
    match state.store.get(&id).await {
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
pub async fn delete_agent(State(state): State<AgentState>, Path(id): Path<String>) -> Response {
    match state.store.delete(&id).await {
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
pub async fn list_agents(State(state): State<AgentState>) -> Response {
    match state.store.list().await {
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

// -- Helpers --------------------------------------------------------------

// `Response` is intentionally returned on the Err arm — callers
// short-circuit by calling `.into_response()` directly. Boxing to
// shrink the Err variant would just push allocation onto every
// request path. Clippy flags it; we silence locally.
#[allow(clippy::result_large_err)]
fn parse_body(headers: &HeaderMap, body: &[u8]) -> Result<(AgentProfile, String), Response> {
    let content_type = headers
        .get(header::CONTENT_TYPE)
        .and_then(|v| v.to_str().ok())
        .unwrap_or("application/json")
        .to_ascii_lowercase();

    let raw = std::str::from_utf8(body).map_err(|e| {
        api_error_response(
            StatusCode::BAD_REQUEST,
            ApiErrorCode::Invalid,
            format!("body is not valid UTF-8: {e}"),
        )
    })?;

    if is_yaml_content_type(&content_type) {
        let profile = tl_policy::load_agent_str(raw).map_err(|e| {
            api_error_response(
                StatusCode::BAD_REQUEST,
                ApiErrorCode::Invalid,
                format!("yaml: {e}"),
            )
        })?;
        Ok((profile, raw.to_string()))
    } else {
        let profile: AgentProfile = serde_json::from_str(raw).map_err(|e| {
            api_error_response(
                StatusCode::BAD_REQUEST,
                ApiErrorCode::Invalid,
                format!("json: {e}"),
            )
        })?;
        // Synthesize a YAML representation for storage. Keeping a YAML
        // copy alongside the parsed form means the AgentRepo
        // `profile_yaml` column always has a populated source.
        let yaml = serde_yaml::to_string(&profile).map_err(|e| {
            api_error_response(
                StatusCode::INTERNAL_SERVER_ERROR,
                ApiErrorCode::Internal,
                format!("yaml serialize: {e}"),
            )
        })?;
        Ok((profile, yaml))
    }
}

fn is_yaml_content_type(s: &str) -> bool {
    s.starts_with("application/yaml")
        || s.starts_with("application/x-yaml")
        || s.starts_with("text/yaml")
        || s.starts_with("text/x-yaml")
}

fn validate_profile(p: &AgentProfile) -> Result<(), String> {
    if p.agent_id.trim().is_empty() {
        return Err("agent_id is required".into());
    }
    if p.display_name.trim().is_empty() {
        return Err("display_name is required".into());
    }
    if p.scope.in_scope.is_empty() {
        return Err("scope.in_scope must contain at least one entry".into());
    }
    Ok(())
}

fn api_error_response(status: StatusCode, code: ApiErrorCode, message: String) -> Response {
    let retriable = matches!(
        code,
        ApiErrorCode::RateLimited | ApiErrorCode::Internal | ApiErrorCode::Unavailable
    );
    let body = ApiError {
        code,
        message,
        retriable,
        details: json!(null),
    };
    (status, Json(body)).into_response()
}

#[cfg(test)]
mod tests {
    use super::*;
    use tl_core::{AgentAuthority, AgentScope, AgentTone};

    fn profile(id: &str) -> AgentProfile {
        AgentProfile {
            agent_id: id.into(),
            display_name: format!("{id} display"),
            scope: AgentScope {
                in_scope: vec!["billing".into()],
                out_of_scope: vec![],
            },
            authority: AgentAuthority::default(),
            tone: AgentTone {
                target: "neutral".into(),
                forbidden: vec![],
            },
            knowledge_sources: vec![],
            escalation_triggers: vec![],
        }
    }

    #[tokio::test]
    async fn memory_store_round_trip() {
        let s = MemoryAgentStore::new();
        s.upsert(&profile("a"), "yaml").await.unwrap();
        let got = s.get("a").await.unwrap();
        assert_eq!(got.agent_id, "a");
    }

    #[tokio::test]
    async fn memory_store_delete_then_get_not_found() {
        let s = MemoryAgentStore::new();
        s.upsert(&profile("a"), "yaml").await.unwrap();
        s.delete("a").await.unwrap();
        assert!(matches!(s.get("a").await, Err(AgentStoreError::NotFound)));
    }

    #[tokio::test]
    async fn memory_store_list_sorted() {
        let s = MemoryAgentStore::new();
        s.upsert(&profile("z"), "y").await.unwrap();
        s.upsert(&profile("a"), "y").await.unwrap();
        s.upsert(&profile("m"), "y").await.unwrap();
        let ids: Vec<String> = s
            .list()
            .await
            .unwrap()
            .iter()
            .map(|p| p.agent_id.clone())
            .collect();
        assert_eq!(ids, vec!["a", "m", "z"]);
    }

    #[tokio::test]
    async fn delete_missing_yields_not_found() {
        let s = MemoryAgentStore::new();
        assert!(matches!(
            s.delete("nope").await,
            Err(AgentStoreError::NotFound)
        ));
    }

    #[test]
    fn yaml_content_types() {
        for ct in [
            "application/yaml",
            "application/x-yaml",
            "text/yaml",
            "text/x-yaml",
            "application/yaml; charset=utf-8",
        ] {
            assert!(is_yaml_content_type(ct), "should be YAML: {ct}");
        }
        for ct in ["application/json", "text/plain", ""] {
            assert!(!is_yaml_content_type(ct), "should NOT be YAML: {ct}");
        }
    }

    #[test]
    fn validate_rejects_empty_agent_id() {
        let mut p = profile("ok");
        p.agent_id = "".into();
        assert!(validate_profile(&p).is_err());
    }

    #[test]
    fn validate_rejects_empty_in_scope() {
        let mut p = profile("ok");
        p.scope.in_scope.clear();
        assert!(validate_profile(&p).is_err());
    }
}
