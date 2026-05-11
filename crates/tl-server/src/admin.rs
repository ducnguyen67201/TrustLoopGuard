//! Admin endpoints for API key lifecycle. Gated behind a separate
//! `TL_ADMIN_KEY` bearer (NOT the per-user `TL_API_KEY`). Mounted under
//! `/v1/admin`.
//!
//! The dashboard is the only intended caller — it holds the admin key
//! server-side and never exposes it to the browser. Plaintext is
//! returned on create and never again.

use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use uuid::Uuid;

use tl_core::{ApiError, ApiErrorCode};

#[cfg(feature = "postgres")]
use tl_storage::{ApiKeyRecord, ApiKeyRepo};

#[cfg(feature = "postgres")]
#[derive(Clone)]
pub struct AdminState {
    pub repo: ApiKeyRepo,
}

/// Memory placeholder so the handlers compile without the `postgres`
/// feature. The memory build of `tl-server` doesn't persist API keys;
/// the admin routes are wired but the handlers report 503.
#[cfg(not(feature = "postgres"))]
#[derive(Clone, Default)]
pub struct AdminState;

#[derive(Debug, Deserialize, ToSchema)]
pub struct CreateKeyRequest {
    pub user_id: String,
    pub name: String,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct CreateKeyResponse {
    pub id: Uuid,
    pub plaintext: String,
    pub prefix: String,
    pub name: String,
    pub user_id: String,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct ApiKeyView {
    pub id: Uuid,
    pub user_id: String,
    pub name: String,
    pub prefix: String,
    pub last_used_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
    pub revoked_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct ApiKeyListResponse {
    pub keys: Vec<ApiKeyView>,
}

#[derive(Debug, Deserialize)]
pub struct UserScopedQuery {
    pub user_id: String,
}

#[cfg(feature = "postgres")]
impl From<ApiKeyRecord> for ApiKeyView {
    fn from(r: ApiKeyRecord) -> Self {
        Self {
            id: r.id,
            user_id: r.user_id,
            name: r.name,
            prefix: r.prefix,
            last_used_at: r.last_used_at,
            created_at: r.created_at,
            revoked_at: r.revoked_at,
        }
    }
}

#[utoipa::path(
    post,
    path = "/v1/admin/keys",
    tag = "admin",
    request_body = CreateKeyRequest,
    responses(
        (status = 201, description = "Key minted", body = CreateKeyResponse),
        (status = 400, description = "Invalid input", body = ApiError),
        (status = 401, description = "Missing or invalid admin key", body = ApiError),
    ),
)]
#[cfg(feature = "postgres")]
pub async fn create_key(
    State(state): State<AdminState>,
    Json(req): Json<CreateKeyRequest>,
) -> Response {
    if req.user_id.trim().is_empty() || req.name.trim().is_empty() {
        return bad_request("user_id and name must be non-empty");
    }
    match state.repo.create(&req.user_id, &req.name).await {
        Ok(minted) => {
            let body = CreateKeyResponse {
                id: minted.record.id,
                plaintext: minted.plaintext,
                prefix: minted.record.prefix,
                name: minted.record.name,
                user_id: minted.record.user_id,
                created_at: minted.record.created_at,
            };
            (StatusCode::CREATED, Json(body)).into_response()
        }
        Err(e) => {
            tracing::error!(error = %e, "create_key failed");
            internal()
        }
    }
}

#[utoipa::path(
    post,
    path = "/v1/admin/keys",
    tag = "admin",
    request_body = CreateKeyRequest,
    responses(
        (status = 503, description = "Server built without postgres feature", body = ApiError),
    ),
)]
#[cfg(not(feature = "postgres"))]
pub async fn create_key(
    State(_state): State<AdminState>,
    Json(_req): Json<CreateKeyRequest>,
) -> Response {
    unavailable()
}

#[utoipa::path(
    get,
    path = "/v1/admin/keys",
    tag = "admin",
    params(("user_id" = String, Query, description = "Owning user id")),
    responses(
        (status = 200, description = "Listed", body = ApiKeyListResponse),
        (status = 401, description = "Missing or invalid admin key", body = ApiError),
    ),
)]
#[cfg(feature = "postgres")]
pub async fn list_keys(
    State(state): State<AdminState>,
    Query(q): Query<UserScopedQuery>,
) -> Response {
    match state.repo.list_by_user(&q.user_id).await {
        Ok(rows) => Json(ApiKeyListResponse {
            keys: rows.into_iter().map(Into::into).collect(),
        })
        .into_response(),
        Err(e) => {
            tracing::error!(error = %e, "list_keys failed");
            internal()
        }
    }
}

#[utoipa::path(
    get,
    path = "/v1/admin/keys",
    tag = "admin",
    params(("user_id" = String, Query, description = "Owning user id")),
    responses(
        (status = 503, description = "Server built without postgres feature", body = ApiError),
    ),
)]
#[cfg(not(feature = "postgres"))]
pub async fn list_keys(
    State(_state): State<AdminState>,
    Query(_q): Query<UserScopedQuery>,
) -> Response {
    unavailable()
}

#[utoipa::path(
    delete,
    path = "/v1/admin/keys/{id}",
    tag = "admin",
    params(
        ("id" = String, Path, description = "Key id"),
        ("user_id" = String, Query, description = "Owning user id"),
    ),
    responses(
        (status = 204, description = "Revoked"),
        (status = 404, description = "Not found", body = ApiError),
        (status = 401, description = "Missing or invalid admin key", body = ApiError),
    ),
)]
#[cfg(feature = "postgres")]
pub async fn revoke_key(
    State(state): State<AdminState>,
    Path(id): Path<Uuid>,
    Query(q): Query<UserScopedQuery>,
) -> Response {
    match state.repo.revoke(id, &q.user_id).await {
        Ok(true) => StatusCode::NO_CONTENT.into_response(),
        Ok(false) => (StatusCode::NOT_FOUND, Json(not_found_body())).into_response(),
        Err(e) => {
            tracing::error!(error = %e, "revoke_key failed");
            internal()
        }
    }
}

#[utoipa::path(
    delete,
    path = "/v1/admin/keys/{id}",
    tag = "admin",
    params(
        ("id" = String, Path, description = "Key id"),
        ("user_id" = String, Query, description = "Owning user id"),
    ),
    responses(
        (status = 503, description = "Server built without postgres feature", body = ApiError),
    ),
)]
#[cfg(not(feature = "postgres"))]
pub async fn revoke_key(
    State(_state): State<AdminState>,
    Path(_id): Path<Uuid>,
    Query(_q): Query<UserScopedQuery>,
) -> Response {
    unavailable()
}

#[cfg(feature = "postgres")]
fn bad_request(message: &str) -> Response {
    (
        StatusCode::BAD_REQUEST,
        Json(ApiError {
            code: ApiErrorCode::Invalid,
            message: message.into(),
            retriable: false,
            details: serde_json::Value::Null,
        }),
    )
        .into_response()
}

#[cfg(feature = "postgres")]
fn internal() -> Response {
    (
        StatusCode::INTERNAL_SERVER_ERROR,
        Json(ApiError {
            code: ApiErrorCode::Internal,
            message: "internal error".into(),
            retriable: true,
            details: serde_json::Value::Null,
        }),
    )
        .into_response()
}

#[cfg(feature = "postgres")]
fn not_found_body() -> ApiError {
    ApiError {
        code: ApiErrorCode::NotFound,
        message: "api key not found".into(),
        retriable: false,
        details: serde_json::Value::Null,
    }
}

#[cfg(not(feature = "postgres"))]
fn unavailable() -> Response {
    (
        StatusCode::SERVICE_UNAVAILABLE,
        Json(ApiError {
            code: ApiErrorCode::Unavailable,
            message: "admin api keys require postgres feature".into(),
            retriable: false,
            details: serde_json::Value::Null,
        }),
    )
        .into_response()
}
