//! Source label policy CRUD endpoints + storage abstraction.
//!
//! The endpoints live behind the bearer-auth layer and manage the
//! workspace-scoped per-origin label overrides that label resolution
//! reads at runtime. `LabelPolicyStore` is a small trait so the server
//! can run without Postgres in tests and local dev; the Postgres impl
//! is an adapter over the unified policy registry.

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
    ApiErrorCode, Origin, SourceLabelPolicy, SourceLabelPolicyEntry, SourceLabelPolicyListResponse,
    UpsertSourceLabelPolicyRequest,
};

mod memory_store;
mod validation;

pub use memory_store::MemoryLabelPolicyStore;
use validation::validate_policy;

use crate::app::error::api_error_response;
use crate::{auth::WorkspaceKeyContext, dashboard_admin::reject_workspace_runtime_key};

#[derive(Debug, thiserror::Error)]
pub enum LabelPolicyStoreError {
    #[error("not found")]
    NotFound,
    #[error("internal: {0}")]
    Internal(String),
}

/// Minimal write/read surface the endpoints need from the registry.
/// Concrete impls: `MemoryLabelPolicyStore` (in this module) and an
/// adapter over `tl_storage::PolicyRepo`.
#[async_trait]
pub trait LabelPolicyStore: Send + Sync {
    async fn upsert(
        &self,
        workspace_id: &str,
        policy: &SourceLabelPolicy,
        enabled: bool,
    ) -> Result<(), LabelPolicyStoreError>;
    async fn get(
        &self,
        workspace_id: &str,
        origin: Origin,
    ) -> Result<SourceLabelPolicyEntry, LabelPolicyStoreError>;
    async fn delete(&self, workspace_id: &str, origin: Origin)
        -> Result<(), LabelPolicyStoreError>;
    async fn list(
        &self,
        workspace_id: &str,
    ) -> Result<Vec<SourceLabelPolicyEntry>, LabelPolicyStoreError>;
}

// -- Endpoint handlers ----------------------------------------------------

/// Log store failure details server-side and return a generic 500 so
/// backend/storage internals never leak into API responses.
fn store_error_response(operation: &'static str, err: &LabelPolicyStoreError) -> Response {
    tracing::error!(error = %err, operation, "label policy store error");
    api_error_response(
        StatusCode::INTERNAL_SERVER_ERROR,
        ApiErrorCode::Internal,
        "internal error".to_string(),
    )
}

/// Parse an `:origin` path segment (serde snake_case name, e.g. `web`).
fn parse_origin(raw: &str) -> Option<Origin> {
    serde_json::from_value(serde_json::Value::String(raw.to_string())).ok()
}

/// The unparsed path segment is deliberately NOT echoed back: it is raw
/// caller input and would otherwise flow into response bodies and logs.
fn invalid_origin_response() -> Response {
    api_error_response(
        StatusCode::UNPROCESSABLE_ENTITY,
        ApiErrorCode::Unprocessable,
        "unknown origin; expected one of: user, system, tool, memory, file, web, email, api, \
         unknown"
            .to_string(),
    )
}

/// Shared state used by the label policy endpoints.
#[derive(Clone)]
pub struct LabelPolicyState {
    pub store: Arc<dyn LabelPolicyStore>,
}

/// `POST /v1/label-policies` — upsert a per-origin override.
#[utoipa::path(
    post,
    path = "/v1/label-policies",
    tag = "label-policies",
    request_body = UpsertSourceLabelPolicyRequest,
    responses(
        (status = 201, description = "Label policy created or updated", body = SourceLabelPolicyEntry),
        (status = 400, description = "Malformed request body", body = ApiError),
        (status = 401, description = "Missing or invalid API key", body = ApiError),
        (status = 403, description = "Workspace runtime keys cannot manage label policies", body = ApiError),
        (status = 422, description = "Label policy failed validation", body = ApiError),
    ),
)]
pub async fn upsert_label_policy(
    State(state): State<LabelPolicyState>,
    runtime_key: Option<Extension<WorkspaceKeyContext>>,
    headers: HeaderMap,
    Json(req): Json<UpsertSourceLabelPolicyRequest>,
) -> Response {
    if let Some(response) =
        reject_workspace_runtime_key(runtime_key, "manage source label policies")
    {
        return response;
    }
    let workspace_id = match crate::policies::workspace_id_from_headers(&headers) {
        Ok(workspace_id) => workspace_id,
        Err(response) => return response,
    };

    if let Err(msg) = validate_policy(&req.policy) {
        return api_error_response(
            StatusCode::UNPROCESSABLE_ENTITY,
            ApiErrorCode::Unprocessable,
            msg,
        );
    }

    match state
        .store
        .upsert(&workspace_id, &req.policy, req.enabled)
        .await
    {
        Ok(()) => (
            StatusCode::CREATED,
            Json(SourceLabelPolicyEntry {
                policy: req.policy,
                enabled: req.enabled,
            }),
        )
            .into_response(),
        Err(e) => store_error_response("upsert", &e),
    }
}

/// `GET /v1/label-policies/:origin`.
#[utoipa::path(
    get,
    path = "/v1/label-policies/{origin}",
    tag = "label-policies",
    params(("origin" = String, Path, description = "Source origin (snake_case)")),
    responses(
        (status = 200, description = "Label policy found", body = SourceLabelPolicyEntry),
        (status = 401, description = "Missing or invalid API key", body = ApiError),
        (status = 404, description = "No policy for this origin", body = ApiError),
        (status = 422, description = "Unknown origin", body = ApiError),
    ),
)]
pub async fn get_label_policy(
    State(state): State<LabelPolicyState>,
    headers: HeaderMap,
    Path(origin): Path<String>,
) -> Response {
    let workspace_id = match crate::policies::workspace_id_from_headers(&headers) {
        Ok(workspace_id) => workspace_id,
        Err(response) => return response,
    };
    let Some(parsed) = parse_origin(&origin) else {
        return invalid_origin_response();
    };
    match state.store.get(&workspace_id, parsed).await {
        Ok(entry) => Json(entry).into_response(),
        Err(LabelPolicyStoreError::NotFound) => api_error_response(
            StatusCode::NOT_FOUND,
            ApiErrorCode::NotFound,
            format!("no label policy for origin `{origin}`"),
        ),
        Err(e) => store_error_response("get", &e),
    }
}

/// `DELETE /v1/label-policies/:origin`. Soft-delete via the store.
#[utoipa::path(
    delete,
    path = "/v1/label-policies/{origin}",
    tag = "label-policies",
    params(("origin" = String, Path, description = "Source origin (snake_case)")),
    responses(
        (status = 204, description = "Label policy deleted"),
        (status = 401, description = "Missing or invalid API key", body = ApiError),
        (status = 403, description = "Workspace runtime keys cannot manage label policies", body = ApiError),
        (status = 404, description = "No policy for this origin", body = ApiError),
        (status = 422, description = "Unknown origin", body = ApiError),
    ),
)]
pub async fn delete_label_policy(
    State(state): State<LabelPolicyState>,
    runtime_key: Option<Extension<WorkspaceKeyContext>>,
    headers: HeaderMap,
    Path(origin): Path<String>,
) -> Response {
    if let Some(response) =
        reject_workspace_runtime_key(runtime_key, "manage source label policies")
    {
        return response;
    }
    let workspace_id = match crate::policies::workspace_id_from_headers(&headers) {
        Ok(workspace_id) => workspace_id,
        Err(response) => return response,
    };
    let Some(parsed) = parse_origin(&origin) else {
        return invalid_origin_response();
    };
    match state.store.delete(&workspace_id, parsed).await {
        Ok(()) => StatusCode::NO_CONTENT.into_response(),
        Err(LabelPolicyStoreError::NotFound) => api_error_response(
            StatusCode::NOT_FOUND,
            ApiErrorCode::NotFound,
            format!("no label policy for origin `{origin}`"),
        ),
        Err(e) => store_error_response("delete", &e),
    }
}

/// `GET /v1/label-policies`. Returns all active (non-deleted) policies
/// for the workspace, including disabled ones.
#[utoipa::path(
    get,
    path = "/v1/label-policies",
    tag = "label-policies",
    responses(
        (status = 200, description = "All label policies", body = SourceLabelPolicyListResponse),
        (status = 401, description = "Missing or invalid API key", body = ApiError),
    ),
)]
pub async fn list_label_policies(
    State(state): State<LabelPolicyState>,
    headers: HeaderMap,
) -> Response {
    let workspace_id = match crate::policies::workspace_id_from_headers(&headers) {
        Ok(workspace_id) => workspace_id,
        Err(response) => return response,
    };
    match state.store.list(&workspace_id).await {
        Ok(policies) => Json(SourceLabelPolicyListResponse { policies }).into_response(),
        Err(e) => store_error_response("list", &e),
    }
}
