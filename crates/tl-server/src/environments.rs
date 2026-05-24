//! Workspace environment endpoints.

use std::collections::HashMap;
use std::sync::Arc;

use async_trait::async_trait;
use axum::{
    extract::{Path, State},
    http::{HeaderMap, StatusCode},
    response::{IntoResponse, Response},
    Json,
};
use serde_json::json;
use tl_core::{
    ApiError, ApiErrorCode, CreateWorkspaceEnvironmentRequest, UpdateWorkspaceEnvironmentRequest,
    WorkspaceEnvironment, WorkspaceEnvironmentListResponse, DEFAULT_ENVIRONMENT_ID,
};
use tokio::sync::RwLock;

#[derive(Debug, thiserror::Error)]
pub enum EnvironmentStoreError {
    #[error("not found")]
    NotFound,
    #[error("validation: {0}")]
    Validation(String),
    #[error("internal: {0}")]
    Internal(String),
}

#[async_trait]
pub trait EnvironmentStore: Send + Sync {
    async fn list(
        &self,
        workspace_id: &str,
    ) -> Result<Vec<WorkspaceEnvironment>, EnvironmentStoreError>;
    async fn get(
        &self,
        workspace_id: &str,
        environment_id: &str,
    ) -> Result<WorkspaceEnvironment, EnvironmentStoreError>;
    async fn default_environment_id(
        &self,
        workspace_id: &str,
    ) -> Result<String, EnvironmentStoreError>;
    async fn create(
        &self,
        workspace_id: &str,
        input: CreateWorkspaceEnvironmentRequest,
    ) -> Result<WorkspaceEnvironment, EnvironmentStoreError>;
    async fn update(
        &self,
        workspace_id: &str,
        environment_id: &str,
        input: UpdateWorkspaceEnvironmentRequest,
    ) -> Result<WorkspaceEnvironment, EnvironmentStoreError>;
    async fn delete(
        &self,
        workspace_id: &str,
        environment_id: &str,
    ) -> Result<(), EnvironmentStoreError>;
}

#[derive(Debug, Default)]
pub struct MemoryEnvironmentStore {
    environments: RwLock<HashMap<(String, String), WorkspaceEnvironment>>,
}

impl MemoryEnvironmentStore {
    pub fn new() -> Self {
        Self::default()
    }
}

#[async_trait]
impl EnvironmentStore for MemoryEnvironmentStore {
    async fn list(
        &self,
        workspace_id: &str,
    ) -> Result<Vec<WorkspaceEnvironment>, EnvironmentStoreError> {
        ensure_default(&self.environments, workspace_id).await;
        let workspace = workspace_id.to_string();
        let mut rows = self
            .environments
            .read()
            .await
            .iter()
            .filter(|((ws, _), _)| ws == &workspace)
            .map(|(_, env)| env.clone())
            .collect::<Vec<_>>();
        rows.sort_by(|a, b| {
            b.is_default
                .cmp(&a.is_default)
                .then_with(|| a.slug.cmp(&b.slug))
        });
        Ok(rows)
    }

    async fn get(
        &self,
        workspace_id: &str,
        environment_id: &str,
    ) -> Result<WorkspaceEnvironment, EnvironmentStoreError> {
        ensure_default(&self.environments, workspace_id).await;
        self.environments
            .read()
            .await
            .get(&(workspace_id.to_string(), environment_id.to_string()))
            .cloned()
            .ok_or(EnvironmentStoreError::NotFound)
    }

    async fn default_environment_id(
        &self,
        workspace_id: &str,
    ) -> Result<String, EnvironmentStoreError> {
        ensure_default(&self.environments, workspace_id).await;
        Ok(DEFAULT_ENVIRONMENT_ID.to_string())
    }

    async fn create(
        &self,
        workspace_id: &str,
        input: CreateWorkspaceEnvironmentRequest,
    ) -> Result<WorkspaceEnvironment, EnvironmentStoreError> {
        validate_slug(&input.slug)?;
        validate_name(&input.name)?;
        let now = chrono::Utc::now().to_rfc3339();
        let id = if input.slug == DEFAULT_ENVIRONMENT_ID {
            DEFAULT_ENVIRONMENT_ID.to_string()
        } else {
            format!("env_{}", uuid::Uuid::now_v7())
        };
        let env = WorkspaceEnvironment {
            id: id.clone(),
            slug: input.slug,
            name: input.name.trim().to_string(),
            description: input.description.and_then(clean_optional),
            is_default: input.is_default,
            created_at: now.clone(),
            updated_at: now,
        };
        let mut guard = self.environments.write().await;
        if guard.iter().any(|((ws, _), existing)| {
            ws == workspace_id && existing.slug == env.slug && existing.id != id
        }) {
            return Err(EnvironmentStoreError::Validation(
                "environment slug already exists".into(),
            ));
        }
        if env.is_default {
            for ((ws, _), existing) in guard.iter_mut() {
                if ws == workspace_id {
                    existing.is_default = false;
                }
            }
        }
        guard.insert((workspace_id.to_string(), id), env.clone());
        Ok(env)
    }

    async fn update(
        &self,
        workspace_id: &str,
        environment_id: &str,
        input: UpdateWorkspaceEnvironmentRequest,
    ) -> Result<WorkspaceEnvironment, EnvironmentStoreError> {
        let mut guard = self.environments.write().await;
        if input.is_default == Some(true) {
            for ((ws, _), existing) in guard.iter_mut() {
                if ws == workspace_id {
                    existing.is_default = false;
                }
            }
        }
        let env = guard
            .get_mut(&(workspace_id.to_string(), environment_id.to_string()))
            .ok_or(EnvironmentStoreError::NotFound)?;
        if let Some(slug) = input.slug {
            validate_slug(&slug)?;
            env.slug = slug;
        }
        if let Some(name) = input.name {
            validate_name(&name)?;
            env.name = name.trim().to_string();
        }
        if let Some(description) = input.description {
            env.description = clean_optional(description);
        }
        if let Some(is_default) = input.is_default {
            env.is_default = is_default;
        }
        env.updated_at = chrono::Utc::now().to_rfc3339();
        Ok(env.clone())
    }

    async fn delete(
        &self,
        workspace_id: &str,
        environment_id: &str,
    ) -> Result<(), EnvironmentStoreError> {
        if environment_id == DEFAULT_ENVIRONMENT_ID {
            return Err(EnvironmentStoreError::Validation(
                "default production environment cannot be deleted".into(),
            ));
        }
        let removed = self
            .environments
            .write()
            .await
            .remove(&(workspace_id.to_string(), environment_id.to_string()));
        removed.map(|_| ()).ok_or(EnvironmentStoreError::NotFound)
    }
}

#[derive(Clone)]
pub struct EnvironmentState {
    pub store: Arc<dyn EnvironmentStore>,
}

#[utoipa::path(
    get,
    path = "/v1/environments",
    tag = "environments",
    responses(
        (status = 200, description = "Workspace environments", body = WorkspaceEnvironmentListResponse),
        (status = 401, description = "Missing or invalid API key", body = ApiError),
    ),
)]
pub async fn list_environments(
    State(state): State<EnvironmentState>,
    headers: HeaderMap,
) -> Response {
    let workspace_id = crate::policies::workspace_id_from_headers(&headers);
    match state.store.list(&workspace_id).await {
        Ok(environments) => Json(WorkspaceEnvironmentListResponse { environments }).into_response(),
        Err(e) => environment_error_response(e),
    }
}

#[utoipa::path(
    post,
    path = "/v1/environments",
    tag = "environments",
    request_body = CreateWorkspaceEnvironmentRequest,
    responses(
        (status = 201, description = "Workspace environment created", body = WorkspaceEnvironment),
        (status = 400, description = "Malformed request", body = ApiError),
        (status = 401, description = "Missing or invalid API key", body = ApiError),
    ),
)]
pub async fn create_environment(
    State(state): State<EnvironmentState>,
    headers: HeaderMap,
    Json(input): Json<CreateWorkspaceEnvironmentRequest>,
) -> Response {
    let workspace_id = crate::policies::workspace_id_from_headers(&headers);
    match state.store.create(&workspace_id, input).await {
        Ok(environment) => (StatusCode::CREATED, Json(environment)).into_response(),
        Err(e) => environment_error_response(e),
    }
}

#[utoipa::path(
    patch,
    path = "/v1/environments/{id}",
    tag = "environments",
    params(("id" = String, Path, description = "Environment id")),
    request_body = UpdateWorkspaceEnvironmentRequest,
    responses(
        (status = 200, description = "Workspace environment updated", body = WorkspaceEnvironment),
        (status = 400, description = "Malformed request", body = ApiError),
        (status = 401, description = "Missing or invalid API key", body = ApiError),
        (status = 404, description = "Environment not found", body = ApiError),
    ),
)]
pub async fn update_environment(
    State(state): State<EnvironmentState>,
    headers: HeaderMap,
    Path(id): Path<String>,
    Json(input): Json<UpdateWorkspaceEnvironmentRequest>,
) -> Response {
    let workspace_id = crate::policies::workspace_id_from_headers(&headers);
    match state.store.update(&workspace_id, &id, input).await {
        Ok(environment) => Json(environment).into_response(),
        Err(e) => environment_error_response(e),
    }
}

#[utoipa::path(
    delete,
    path = "/v1/environments/{id}",
    tag = "environments",
    params(("id" = String, Path, description = "Environment id")),
    responses(
        (status = 204, description = "Workspace environment deleted"),
        (status = 400, description = "Environment cannot be deleted", body = ApiError),
        (status = 401, description = "Missing or invalid API key", body = ApiError),
        (status = 404, description = "Environment not found", body = ApiError),
    ),
)]
pub async fn delete_environment(
    State(state): State<EnvironmentState>,
    headers: HeaderMap,
    Path(id): Path<String>,
) -> Response {
    let workspace_id = crate::policies::workspace_id_from_headers(&headers);
    match state.store.delete(&workspace_id, &id).await {
        Ok(()) => StatusCode::NO_CONTENT.into_response(),
        Err(e) => environment_error_response(e),
    }
}

pub fn environment_id_from_headers(headers: &HeaderMap) -> String {
    headers
        .get("x-tlg-environment-id")
        .and_then(|value| value.to_str().ok())
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(str::to_string)
        .unwrap_or_else(|| DEFAULT_ENVIRONMENT_ID.to_string())
}

fn validate_slug(slug: &str) -> Result<(), EnvironmentStoreError> {
    let valid = !slug.is_empty()
        && slug.len() <= 64
        && slug
            .chars()
            .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-');
    if valid {
        Ok(())
    } else {
        Err(EnvironmentStoreError::Validation(
            "environment slug must use lowercase letters, digits, and hyphens".into(),
        ))
    }
}

fn validate_name(name: &str) -> Result<(), EnvironmentStoreError> {
    if name.trim().is_empty() {
        Err(EnvironmentStoreError::Validation(
            "environment name is required".into(),
        ))
    } else {
        Ok(())
    }
}

fn clean_optional(value: String) -> Option<String> {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        None
    } else {
        Some(trimmed.to_string())
    }
}

async fn ensure_default(
    environments: &RwLock<HashMap<(String, String), WorkspaceEnvironment>>,
    workspace_id: &str,
) {
    let key = (workspace_id.to_string(), DEFAULT_ENVIRONMENT_ID.to_string());
    if environments.read().await.contains_key(&key) {
        return;
    }
    let now = chrono::Utc::now().to_rfc3339();
    environments.write().await.insert(
        key,
        WorkspaceEnvironment {
            id: DEFAULT_ENVIRONMENT_ID.to_string(),
            slug: DEFAULT_ENVIRONMENT_ID.to_string(),
            name: "Production".to_string(),
            description: None,
            is_default: true,
            created_at: now.clone(),
            updated_at: now,
        },
    );
}

fn environment_error_response(error: EnvironmentStoreError) -> Response {
    let (status, code) = match error {
        EnvironmentStoreError::NotFound => (StatusCode::NOT_FOUND, ApiErrorCode::NotFound),
        EnvironmentStoreError::Validation(_) => (StatusCode::BAD_REQUEST, ApiErrorCode::Invalid),
        EnvironmentStoreError::Internal(_) => {
            (StatusCode::INTERNAL_SERVER_ERROR, ApiErrorCode::Internal)
        }
    };
    crate::log_api_error(status, code, &error.to_string());
    let body = ApiError {
        code,
        message: error.to_string(),
        retriable: false,
        details: json!(null),
    };
    (status, Json(body)).into_response()
}
