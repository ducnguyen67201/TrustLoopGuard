use super::{
    request_context::{request_user_id, X_USER_EMAIL_HEADER},
    response::{api_error, internal_error},
    AddMemberOutcome, TeamState, TeamStoreError,
};
use crate::{
    auth::{InternalServiceContext, WorkspaceKeyContext},
    dashboard_admin::authorize_workspace_admin,
    jwt::UserContext,
};
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
    InviteListResponse, MemberListResponse, MyWorkspacesResponse, WorkspaceRole,
};

/// GET /v1/team/members
pub async fn list_members(State(state): State<TeamState>, headers: HeaderMap) -> Response {
    let workspace_id = match crate::policies::workspace_id_from_headers(&headers) {
        Ok(workspace_id) => workspace_id,
        Err(response) => return response,
    };
    match state.store.list_members(&workspace_id).await {
        Ok(members) => Json(MemberListResponse { members }).into_response(),
        Err(e) => internal_error(e),
    }
}

/// `GET /v1/team/invites` — list pending workspace invites.
#[utoipa::path(
    get,
    path = "/v1/team/invites",
    tag = "team",
    responses(
        (status = 200, description = "Pending invites returned", body = InviteListResponse),
        (status = 401, description = "Missing or invalid bearer token", body = ApiError),
        (status = 403, description = "Owner or Admin role required", body = ApiError),
        (status = 500, description = "Internal error", body = ApiError),
    ),
)]
pub async fn list_invites(
    State(state): State<TeamState>,
    user: Option<Extension<UserContext>>,
    internal: Option<Extension<InternalServiceContext>>,
    runtime_key: Option<Extension<WorkspaceKeyContext>>,
    headers: HeaderMap,
) -> Response {
    let (workspace_id, _) = match authorize_workspace_admin(
        &state.store,
        &headers,
        user,
        internal,
        runtime_key,
        "manage team invites",
    )
    .await
    {
        Ok(authorized) => authorized,
        Err(response) => return response,
    };
    match state.store.list_pending_invites(&workspace_id).await {
        Ok(invites) => Json(InviteListResponse { invites }).into_response(),
        Err(e) => internal_error(e),
    }
}

/// `POST /v1/team/invites` — add a member or create a pending invite.
#[utoipa::path(
    post,
    path = "/v1/team/invites",
    tag = "team",
    request_body = CreateInviteRequest,
    responses(
        (status = 201, description = "Member added or invite created", body = CreateInviteResponse),
        (status = 400, description = "Invalid email", body = ApiError),
        (status = 401, description = "Missing or invalid bearer token", body = ApiError),
        (status = 403, description = "Owner or Admin role required", body = ApiError),
        (status = 409, description = "Pending invite already exists", body = ApiError),
        (status = 422, description = "Owner role cannot be assigned through invites", body = ApiError),
        (status = 500, description = "Internal error", body = ApiError),
    ),
)]
pub async fn create_invite(
    State(state): State<TeamState>,
    user: Option<Extension<UserContext>>,
    internal: Option<Extension<InternalServiceContext>>,
    runtime_key: Option<Extension<WorkspaceKeyContext>>,
    headers: HeaderMap,
    Json(req): Json<CreateInviteRequest>,
) -> Response {
    let (workspace_id, invited_by) = match authorize_workspace_admin(
        &state.store,
        &headers,
        user,
        internal,
        runtime_key,
        "manage team invites",
    )
    .await
    {
        Ok(authorized) => authorized,
        Err(response) => return response,
    };
    let email = req.email.trim();
    if email.is_empty() || !email.contains('@') {
        return api_error(
            StatusCode::BAD_REQUEST,
            ApiErrorCode::Invalid,
            "email is required".into(),
        );
    }
    if req.role == WorkspaceRole::Owner {
        return api_error(
            StatusCode::UNPROCESSABLE_ENTITY,
            ApiErrorCode::Invalid,
            "owner role cannot be assigned through invites".into(),
        );
    }

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

/// `DELETE /v1/team/invites/:id` — revoke a pending invite.
#[utoipa::path(
    delete,
    path = "/v1/team/invites/{id}",
    tag = "team",
    params(("id" = String, Path, description = "Invite id")),
    responses(
        (status = 204, description = "Invite revoked"),
        (status = 401, description = "Missing or invalid bearer token", body = ApiError),
        (status = 403, description = "Owner or Admin role required", body = ApiError),
        (status = 404, description = "Invite not found", body = ApiError),
        (status = 500, description = "Internal error", body = ApiError),
    ),
)]
pub async fn revoke_invite(
    State(state): State<TeamState>,
    user: Option<Extension<UserContext>>,
    internal: Option<Extension<InternalServiceContext>>,
    runtime_key: Option<Extension<WorkspaceKeyContext>>,
    headers: HeaderMap,
    Path(invite_id): Path<String>,
) -> Response {
    let (workspace_id, _) = match authorize_workspace_admin(
        &state.store,
        &headers,
        user,
        internal,
        runtime_key,
        "manage team invites",
    )
    .await
    {
        Ok(authorized) => authorized,
        Err(response) => return response,
    };
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

    let is_platform_admin = match state.store.is_platform_admin(user_id).await {
        Ok(is_platform_admin) => is_platform_admin,
        Err(e) => return internal_error(e),
    };
    let workspaces = if is_platform_admin {
        tracing::info!(
            user_id = %user_id,
            "platform administrator listed all active workspaces"
        );
        state.store.list_all_workspaces().await
    } else {
        state.store.list_workspaces_for_user(user_id).await
    };
    match workspaces {
        Ok(workspaces) => Json(MyWorkspacesResponse {
            is_platform_admin,
            workspaces,
        })
        .into_response(),
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

/// DELETE /v1/team/my-workspaces/:id — soft-delete a workspace owned
/// by the signed-in caller and revoke its pending access paths.
#[utoipa::path(
    delete,
    path = "/v1/team/my-workspaces/{id}",
    tag = "team",
    params(("id" = String, Path, description = "Workspace id")),
    responses(
        (status = 204, description = "Workspace deleted"),
        (status = 400, description = "Missing or invalid user id", body = ApiError),
        (status = 401, description = "Missing or invalid bearer token", body = ApiError),
        (status = 403, description = "Runtime key rejected or caller is not the workspace owner", body = ApiError),
        (status = 404, description = "Workspace not found", body = ApiError),
        (status = 500, description = "Internal error", body = ApiError),
    ),
)]
pub async fn delete_my_workspace(
    State(state): State<TeamState>,
    headers: HeaderMap,
    user: Option<Extension<UserContext>>,
    _internal: Option<Extension<InternalServiceContext>>,
    runtime_key: Option<Extension<WorkspaceKeyContext>>,
    Path(workspace_id): Path<String>,
) -> Response {
    if runtime_key.is_some() {
        return api_error(
            StatusCode::FORBIDDEN,
            ApiErrorCode::Forbidden,
            "workspace runtime keys cannot delete workspaces".into(),
        );
    }

    let Some(user_id) = request_user_id(&headers, user) else {
        return api_error(
            StatusCode::BAD_REQUEST,
            ApiErrorCode::Invalid,
            "X-TLG-User-Id header is required and must be a UUID".into(),
        );
    };

    match state.store.delete_workspace(user_id, &workspace_id).await {
        Ok(()) => StatusCode::NO_CONTENT.into_response(),
        Err(TeamStoreError::Forbidden) => api_error(
            StatusCode::FORBIDDEN,
            ApiErrorCode::Forbidden,
            "only the workspace owner can delete this workspace".into(),
        ),
        Err(TeamStoreError::NotFound) => api_error(
            StatusCode::NOT_FOUND,
            ApiErrorCode::NotFound,
            "workspace not found".into(),
        ),
        Err(error) => internal_error(error),
    }
}
