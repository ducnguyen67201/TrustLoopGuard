//! Tool metadata registry CRUD endpoints + storage abstraction.
//!
//! The endpoints live behind the bearer-auth layer and manage the
//! workspace-scoped tool registry that action resolution reads at
//! runtime. `ToolMetadataStore` is a small trait so the server can run
//! without Postgres in tests and local dev; the Postgres impl is an
//! adapter over `tl_storage::ToolMetadataRepo`.

use std::sync::Arc;

use async_trait::async_trait;
use axum::{
    extract::{Extension, Path, State},
    http::{HeaderMap, StatusCode},
    response::{IntoResponse, Response},
    Json,
};
#[allow(unused_imports)]
use tl_core::ApiError;
use tl_core::{
    ApiErrorCode, ToolMetadata, ToolMetadataEntry, ToolMetadataListResponse,
    UpsertToolMetadataRequest,
};

mod memory_store;
mod validation;

pub use memory_store::MemoryToolMetadataStore;
use validation::validate_metadata;

use crate::{
    app::error::api_error_response,
    auth::{InternalServiceContext, WorkspaceKeyContext},
    jwt::UserContext,
    team::TeamStore,
};

#[derive(Debug, thiserror::Error)]
pub enum ToolMetadataStoreError {
    #[error("not found")]
    NotFound,
    #[error("internal: {0}")]
    Internal(String),
}

/// Minimal write/read surface the endpoints need from the registry.
/// Concrete impls: `MemoryToolMetadataStore` (in this module) and an
/// adapter over `tl_storage::ToolMetadataRepo`.
#[async_trait]
pub trait ToolMetadataStore: Send + Sync {
    async fn upsert(
        &self,
        workspace_id: &str,
        metadata: &ToolMetadata,
        enabled: bool,
    ) -> Result<(), ToolMetadataStoreError>;
    async fn get(
        &self,
        workspace_id: &str,
        tool: &str,
    ) -> Result<ToolMetadataEntry, ToolMetadataStoreError>;
    async fn delete(&self, workspace_id: &str, tool: &str) -> Result<(), ToolMetadataStoreError>;
    async fn list(
        &self,
        workspace_id: &str,
    ) -> Result<Vec<ToolMetadataEntry>, ToolMetadataStoreError>;
}

// -- Endpoint handlers ----------------------------------------------------

/// Log store failure details server-side and return a generic 500 so
/// backend/storage internals never leak into API responses.
fn store_error_response(operation: &'static str, err: &ToolMetadataStoreError) -> Response {
    tracing::error!(error = %err, operation, "tool metadata store error");
    api_error_response(
        StatusCode::INTERNAL_SERVER_ERROR,
        ApiErrorCode::Internal,
        "internal error".to_string(),
    )
}

/// Shared state used by the tool metadata endpoints.
#[derive(Clone)]
pub struct ToolMetadataState {
    pub store: Arc<dyn ToolMetadataStore>,
    pub team_store: Arc<dyn TeamStore>,
}

/// `POST /v1/tool-metadata` — upsert a registry entry.
#[utoipa::path(
    post,
    path = "/v1/tool-metadata",
    tag = "tool-metadata",
    request_body = UpsertToolMetadataRequest,
    responses(
        (status = 201, description = "Tool metadata created or updated", body = ToolMetadataEntry),
        (status = 400, description = "Malformed request body", body = ApiError),
        (status = 401, description = "Missing or invalid API key", body = ApiError),
        (status = 422, description = "Tool metadata failed validation", body = ApiError),
    ),
)]
pub async fn upsert_tool_metadata(
    State(state): State<ToolMetadataState>,
    user: Option<Extension<UserContext>>,
    internal: Option<Extension<InternalServiceContext>>,
    runtime_key: Option<Extension<WorkspaceKeyContext>>,
    headers: HeaderMap,
    Json(req): Json<UpsertToolMetadataRequest>,
) -> Response {
    let (workspace_id, _) = match crate::dashboard_admin::authorize_workspace_admin(
        &state.team_store,
        &headers,
        user,
        internal,
        runtime_key,
        "modify tool metadata",
    )
    .await
    {
        Ok(authorized) => authorized,
        Err(response) => return response,
    };

    if let Err(msg) = validate_metadata(&req.metadata) {
        return api_error_response(
            StatusCode::UNPROCESSABLE_ENTITY,
            ApiErrorCode::Unprocessable,
            msg,
        );
    }

    match state
        .store
        .upsert(&workspace_id, &req.metadata, req.enabled)
        .await
    {
        Ok(()) => (
            StatusCode::CREATED,
            Json(ToolMetadataEntry {
                metadata: req.metadata,
                enabled: req.enabled,
            }),
        )
            .into_response(),
        Err(e) => store_error_response("upsert", &e),
    }
}

/// `GET /v1/tool-metadata/:tool`.
#[utoipa::path(
    get,
    path = "/v1/tool-metadata/{tool}",
    tag = "tool-metadata",
    params(("tool" = String, Path, description = "Tool name")),
    responses(
        (status = 200, description = "Tool metadata found", body = ToolMetadataEntry),
        (status = 401, description = "Missing or invalid API key", body = ApiError),
        (status = 404, description = "Tool not registered", body = ApiError),
    ),
)]
pub async fn get_tool_metadata(
    State(state): State<ToolMetadataState>,
    headers: HeaderMap,
    Path(tool): Path<String>,
) -> Response {
    let workspace_id = match crate::policies::workspace_id_from_headers(&headers) {
        Ok(workspace_id) => workspace_id,
        Err(response) => return response,
    };
    match state.store.get(&workspace_id, &tool).await {
        Ok(entry) => Json(entry).into_response(),
        Err(ToolMetadataStoreError::NotFound) => api_error_response(
            StatusCode::NOT_FOUND,
            ApiErrorCode::NotFound,
            format!("tool `{tool}` not registered"),
        ),
        Err(e) => store_error_response("get", &e),
    }
}

/// `DELETE /v1/tool-metadata/:tool`. Soft-delete via the store.
#[utoipa::path(
    delete,
    path = "/v1/tool-metadata/{tool}",
    tag = "tool-metadata",
    params(("tool" = String, Path, description = "Tool name")),
    responses(
        (status = 204, description = "Tool metadata deleted"),
        (status = 401, description = "Missing or invalid API key", body = ApiError),
        (status = 404, description = "Tool not registered", body = ApiError),
    ),
)]
pub async fn delete_tool_metadata(
    State(state): State<ToolMetadataState>,
    user: Option<Extension<UserContext>>,
    internal: Option<Extension<InternalServiceContext>>,
    runtime_key: Option<Extension<WorkspaceKeyContext>>,
    headers: HeaderMap,
    Path(tool): Path<String>,
) -> Response {
    let (workspace_id, _) = match crate::dashboard_admin::authorize_workspace_admin(
        &state.team_store,
        &headers,
        user,
        internal,
        runtime_key,
        "modify tool metadata",
    )
    .await
    {
        Ok(authorized) => authorized,
        Err(response) => return response,
    };
    match state.store.delete(&workspace_id, &tool).await {
        Ok(()) => StatusCode::NO_CONTENT.into_response(),
        Err(ToolMetadataStoreError::NotFound) => api_error_response(
            StatusCode::NOT_FOUND,
            ApiErrorCode::NotFound,
            format!("tool `{tool}` not registered"),
        ),
        Err(e) => store_error_response("delete", &e),
    }
}

/// `GET /v1/tool-metadata`. Returns all active (non-deleted) registry
/// entries for the workspace, including disabled ones.
#[utoipa::path(
    get,
    path = "/v1/tool-metadata",
    tag = "tool-metadata",
    responses(
        (status = 200, description = "All registered tools", body = ToolMetadataListResponse),
        (status = 401, description = "Missing or invalid API key", body = ApiError),
    ),
)]
pub async fn list_tool_metadata(
    State(state): State<ToolMetadataState>,
    headers: HeaderMap,
) -> Response {
    let workspace_id = match crate::policies::workspace_id_from_headers(&headers) {
        Ok(workspace_id) => workspace_id,
        Err(response) => return response,
    };
    match state.store.list(&workspace_id).await {
        Ok(tools) => Json(ToolMetadataListResponse { tools }).into_response(),
        Err(e) => store_error_response("list", &e),
    }
}
