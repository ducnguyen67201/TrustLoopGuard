use axum::{
    extract::{Path, State},
    http::{HeaderMap, StatusCode},
    response::{IntoResponse, Response},
    Extension, Json,
};
#[allow(unused_imports)]
use tl_core::ApiError;
#[allow(unused_imports)]
use tl_core::MyWorkspace;
use tl_core::{
    ApiErrorCode, CreateInviteRequest, CreateInviteResponse, CreateWorkspaceRequest,
    InviteListResponse, MemberListResponse, MyWorkspacesResponse,
};
use uuid::Uuid;

use super::{
    request_context::{request_user_id, X_USER_EMAIL_HEADER, X_USER_HEADER},
    response::{api_error, internal_error},
    AddMemberOutcome, TeamState, TeamStoreError,
};
use crate::jwt::UserContext;

/// GET /v1/team/members
pub async fn list_members(State(state): State<TeamState>, headers: HeaderMap) -> Response {
    let workspace_id = crate::policies::workspace_id_from_headers(&headers);
    match state.store.list_members(&workspace_id).await {
        Ok(members) => Json(MemberListResponse { members }).into_response(),
        Err(e) => internal_error(e),
    }
}

/// GET /v1/team/invites
pub async fn list_invites(State(state): State<TeamState>, headers: HeaderMap) -> Response {
    let workspace_id = crate::policies::workspace_id_from_headers(&headers);
    match state.store.list_pending_invites(&workspace_id).await {
        Ok(invites) => Json(InviteListResponse { invites }).into_response(),
        Err(e) => internal_error(e),
    }
}

/// POST /v1/team/invites
pub async fn create_invite(
    State(state): State<TeamState>,
    headers: HeaderMap,
    Json(req): Json<CreateInviteRequest>,
) -> Response {
    let workspace_id = crate::policies::workspace_id_from_headers(&headers);
    let email = req.email.trim();
    if email.is_empty() || !email.contains('@') {
        return api_error(
            StatusCode::BAD_REQUEST,
            ApiErrorCode::Invalid,
            "email is required".into(),
        );
    }
    let invited_by = headers
        .get(X_USER_HEADER)
        .and_then(|v| v.to_str().ok())
        .and_then(|s| Uuid::parse_str(s.trim()).ok());

    match state
        .store
        .add_member_or_invite(&workspace_id, email, req.role, invited_by)
        .await
    {
        Ok(AddMemberOutcome::Invited(invite)) => (
            StatusCode::CREATED,
            Json(CreateInviteResponse::Invited { invite }),
        )
            .into_response(),
        Ok(AddMemberOutcome::Added(member)) => (
            StatusCode::CREATED,
            Json(CreateInviteResponse::Added { member }),
        )
            .into_response(),
        Err(TeamStoreError::Conflict) => api_error(
            StatusCode::CONFLICT,
            ApiErrorCode::Unprocessable,
            "a pending invite already exists for this email".into(),
        ),
        Err(e) => internal_error(e),
    }
}

/// DELETE /v1/team/invites/:id
pub async fn revoke_invite(
    State(state): State<TeamState>,
    headers: HeaderMap,
    Path(invite_id): Path<String>,
) -> Response {
    let workspace_id = crate::policies::workspace_id_from_headers(&headers);
    match state.store.revoke_invite(&workspace_id, &invite_id).await {
        Ok(()) => StatusCode::NO_CONTENT.into_response(),
        Err(TeamStoreError::NotFound) => api_error(
            StatusCode::NOT_FOUND,
            ApiErrorCode::NotFound,
            "invite not found".into(),
        ),
        Err(e) => internal_error(e),
    }
}

/// GET /v1/team/my-workspaces — list workspaces for the signed-in user.
///
/// Reads `X-TLG-User-Id` (required, UUID) and `X-TLG-User-Email`
/// (optional). When the email is present we first bulk-accept any
/// pending invites addressed to it; the membership query then sees
/// the new rows in the same response. This is the dashboard's
/// "auto-bind on next request" mechanism.
#[utoipa::path(
    get,
    path = "/v1/team/my-workspaces",
    tag = "team",
    responses(
        (status = 200, description = "User workspaces returned", body = MyWorkspacesResponse),
        (status = 400, description = "Missing or invalid user id", body = ApiError),
        (status = 401, description = "Missing or invalid bearer token", body = ApiError),
        (status = 403, description = "User not approved", body = ApiError),
        (status = 500, description = "Internal error", body = ApiError),
    ),
)]
pub async fn list_my_workspaces(
    State(state): State<TeamState>,
    headers: HeaderMap,
    user: Option<Extension<UserContext>>,
) -> Response {
    let user_id = request_user_id(&headers, user);
    let Some(user_id) = user_id else {
        return api_error(
            StatusCode::BAD_REQUEST,
            ApiErrorCode::Invalid,
            "X-TLG-User-Id header is required and must be a UUID".into(),
        );
    };

    if let Some(email) = headers
        .get(X_USER_EMAIL_HEADER)
        .and_then(|v| v.to_str().ok())
        .map(str::trim)
        .filter(|s| !s.is_empty())
    {
        if let Err(e) = state
            .store
            .accept_pending_invites_for_email(email, user_id)
            .await
        {
            tracing::warn!(
                user_id = %user_id,
                error = %e,
                "auto-bind pending invites failed; continuing with existing memberships"
            );
        }
    }

    match state.store.list_workspaces_for_user(user_id).await {
        Ok(workspaces) => Json(MyWorkspacesResponse { workspaces }).into_response(),
        Err(e) => internal_error(e),
    }
}

/// POST /v1/team/my-workspaces — create a new workspace owned by
/// the caller. Bootstraps a fresh organization too, so a user who
/// signed up without an invite can self-serve.
#[utoipa::path(
    post,
    path = "/v1/team/my-workspaces",
    tag = "team",
    request_body = CreateWorkspaceRequest,
    responses(
        (status = 201, description = "Workspace created", body = MyWorkspace),
        (status = 400, description = "Validation failed", body = ApiError),
        (status = 401, description = "Missing or invalid bearer token", body = ApiError),
        (status = 403, description = "User not approved or workspace self-service disabled", body = ApiError),
        (status = 500, description = "Internal error", body = ApiError),
    ),
)]
pub async fn create_my_workspace(
    State(state): State<TeamState>,
    headers: HeaderMap,
    user: Option<Extension<UserContext>>,
    Json(req): Json<CreateWorkspaceRequest>,
) -> Response {
    if !state.workspace_self_service_enabled {
        return api_error(
            StatusCode::FORBIDDEN,
            ApiErrorCode::Forbidden,
            "workspace self-service creation is disabled for this deployment".into(),
        );
    }

    let user_id = request_user_id(&headers, user);
    let Some(user_id) = user_id else {
        return api_error(
            StatusCode::BAD_REQUEST,
            ApiErrorCode::Invalid,
            "X-TLG-User-Id header is required and must be a UUID".into(),
        );
    };
    let name = req.name.trim();
    if name.is_empty() {
        return api_error(
            StatusCode::BAD_REQUEST,
            ApiErrorCode::Invalid,
            "workspace name is required".into(),
        );
    }
    match state.store.create_workspace(user_id, name).await {
        Ok(ws) => (StatusCode::CREATED, Json(ws)).into_response(),
        Err(TeamStoreError::NotFound) => api_error(
            StatusCode::UNAUTHORIZED,
            ApiErrorCode::Unauthorized,
            "signed-in user was not found; sign in again".into(),
        ),
        Err(e) => internal_error(e),
    }
}
