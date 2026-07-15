//! Workspace environment endpoints.

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
use tl_core::{
    CreateWorkspaceEnvironmentRequest, UpdateWorkspaceEnvironmentRequest, WorkspaceEnvironment,
    WorkspaceEnvironmentListResponse,
};

mod memory_store;
mod response;
mod validation;

pub use memory_store::MemoryEnvironmentStore;
pub(crate) use response::environment_error_response;

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
    let workspace_id = match crate::policies::workspace_id_from_headers(&headers) {
        Ok(workspace_id) => workspace_id,
        Err(response) => return response,
    };
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
    let workspace_id = match crate::policies::workspace_id_from_headers(&headers) {
        Ok(workspace_id) => workspace_id,
        Err(response) => return response,
    };
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
    let workspace_id = match crate::policies::workspace_id_from_headers(&headers) {
        Ok(workspace_id) => workspace_id,
        Err(response) => return response,
    };
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
    let workspace_id = match crate::policies::workspace_id_from_headers(&headers) {
        Ok(workspace_id) => workspace_id,
        Err(response) => return response,
    };
    match state.store.delete(&workspace_id, &id).await {
        Ok(()) => StatusCode::NO_CONTENT.into_response(),
        Err(e) => environment_error_response(e),
    }
}

pub fn environment_id_from_headers(headers: &HeaderMap) -> Option<String> {
    headers
        .get("x-tlg-environment-id")
        .and_then(|value| value.to_str().ok())
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(str::to_string)
}

pub async fn resolve_environment_id(
    headers: &HeaderMap,
    store: &dyn EnvironmentStore,
    workspace_id: &str,
) -> Result<String, EnvironmentStoreError> {
    match environment_id_from_headers(headers) {
        Some(environment_id) => store
            .get(workspace_id, &environment_id)
            .await
            .map(|environment| environment.id),
        None => store.default_environment_id(workspace_id).await,
    }
}
