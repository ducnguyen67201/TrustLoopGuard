use std::sync::Arc;

use axum::{
    extract::Extension,
    http::{HeaderMap, StatusCode},
    response::Response,
};
use tl_core::{ApiErrorCode, WorkspaceRole};
use uuid::Uuid;

use crate::{
    auth::{InternalServiceContext, WorkspaceKeyContext},
    jwt::UserContext,
    team::TeamStore,
};

use super::response::api_error_response;

pub(super) async fn authorize_api_key_management(
    team_store: &Arc<dyn TeamStore>,
    headers: &HeaderMap,
    user: Option<Extension<UserContext>>,
    internal: Option<Extension<InternalServiceContext>>,
    runtime_key: Option<Extension<WorkspaceKeyContext>>,
) -> Result<(String, Option<Uuid>), Response> {
    authorize_workspace_admin(
        team_store,
        headers,
        user,
        internal,
        runtime_key,
        "manage API keys",
    )
    .await
}

/// Owner/Admin gate shared by workspace admin surfaces (API keys,
/// settings writes, budget alert configs, LLM pricing). Runtime keys
/// are rejected outright: a running agent must never be able to change
/// the controls that govern it.
pub(crate) async fn authorize_workspace_admin(
    team_store: &Arc<dyn TeamStore>,
    headers: &HeaderMap,
    user: Option<Extension<UserContext>>,
    internal: Option<Extension<InternalServiceContext>>,
    runtime_key: Option<Extension<WorkspaceKeyContext>>,
    action: &str,
) -> Result<(String, Option<Uuid>), Response> {
    if runtime_key.is_some() {
        return Err(api_error_response(
            StatusCode::FORBIDDEN,
            ApiErrorCode::Forbidden,
            format!("workspace runtime keys cannot {action}"),
        ));
    }

    let workspace_id = crate::policies::workspace_id_from_headers(headers);
    let user_id = match user {
        Some(Extension(ctx)) => ctx.user_id,
        None if internal.is_some() => match forwarded_user_id(headers) {
            Some(user_id) => user_id,
            None => {
                return Err(api_error_response(
                    StatusCode::FORBIDDEN,
                    ApiErrorCode::Forbidden,
                    format!("signed-in user context is required to {action}"),
                ));
            }
        },
        // Local dev can run with `auth=None`, which disables the bearer
        // middleware and therefore never attaches `InternalServiceContext`.
        // In that mode the router is already intentionally unauthenticated;
        // still require a forwarded user id so the workspace role check below
        // remains the source of truth for workspace admin operations.
        None => match forwarded_user_id(headers) {
            Some(user_id) => user_id,
            None => {
                return Err(api_error_response(
                    StatusCode::UNAUTHORIZED,
                    ApiErrorCode::Unauthorized,
                    format!("authenticated user is required to {action}"),
                ));
            }
        },
    };

    require_admin_role(team_store, &workspace_id, user_id, action).await?;
    Ok((workspace_id, Some(user_id)))
}

async fn require_admin_role(
    team_store: &Arc<dyn TeamStore>,
    workspace_id: &str,
    user_id: Uuid,
    action: &str,
) -> Result<(), Response> {
    let members = team_store
        .list_members(workspace_id)
        .await
        .map_err(|error| {
            api_error_response(
                StatusCode::INTERNAL_SERVER_ERROR,
                ApiErrorCode::Internal,
                error.to_string(),
            )
        })?;

    let user_id = user_id.to_string();
    let role = members
        .iter()
        .find(|member| member.user_id == user_id)
        .map(|member| member.role);

    match role {
        Some(WorkspaceRole::Owner | WorkspaceRole::Admin) => Ok(()),
        Some(WorkspaceRole::Editor | WorkspaceRole::Viewer) | None => Err(api_error_response(
            StatusCode::FORBIDDEN,
            ApiErrorCode::Forbidden,
            format!("workspace owner or admin role is required to {action}"),
        )),
    }
}

fn forwarded_user_id(headers: &HeaderMap) -> Option<Uuid> {
    headers
        .get("x-tlg-user-id")
        .and_then(|value| value.to_str().ok())
        .and_then(|value| Uuid::parse_str(value.trim()).ok())
}
