use std::sync::Arc;

use axum::{
    extract::Extension,
    http::{HeaderMap, StatusCode},
    response::Response,
};
use tl_core::{ApiErrorCode, MyWorkspace, WorkspaceRole};
use uuid::Uuid;

use crate::{
    auth::{InternalServiceContext, WorkspaceKeyContext},
    jwt::UserContext,
    team::{TeamStore, TeamStoreError},
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
    if let Some(response) = reject_workspace_runtime_key(runtime_key, action) {
        return Err(response);
    }

    let workspace_id = crate::policies::workspace_id_from_headers(headers)?;
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

/// Reject runtime credentials at control-plane mutation boundaries.
///
/// Some compatibility endpoints predate dashboard membership enforcement and
/// still support the internal operator lane. They must nevertheless reject a
/// workspace runtime key before touching governing configuration.
pub(crate) fn reject_workspace_runtime_key(
    runtime_key: Option<Extension<WorkspaceKeyContext>>,
    action: &str,
) -> Option<Response> {
    if runtime_key.is_some() {
        return Some(api_error_response(
            StatusCode::FORBIDDEN,
            ApiErrorCode::Forbidden,
            format!("workspace runtime keys cannot {action}"),
        ));
    }
    None
}

pub(crate) async fn authorize_workspace_admin_for_workspace(
    team_store: &Arc<dyn TeamStore>,
    workspace_id: &str,
    headers: &HeaderMap,
    user: Option<Extension<UserContext>>,
    internal: Option<Extension<InternalServiceContext>>,
    runtime_key: Option<Extension<WorkspaceKeyContext>>,
    action: &str,
) -> Result<Uuid, Response> {
    if let Some(response) = reject_workspace_runtime_key(runtime_key, action) {
        return Err(response);
    }

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

    require_admin_role(team_store, workspace_id, user_id, action).await?;
    Ok(user_id)
}

/// Resolve a dashboard member from signed/forwarded identity and return the
/// authoritative workspace record, including operational feature flags.
pub(crate) async fn authorize_workspace_member(
    team_store: &Arc<dyn TeamStore>,
    headers: &HeaderMap,
    user: Option<Extension<UserContext>>,
    internal: Option<Extension<InternalServiceContext>>,
    runtime_key: Option<Extension<WorkspaceKeyContext>>,
    action: &str,
) -> Result<MyWorkspace, Response> {
    if let Some(response) = reject_workspace_runtime_key(runtime_key, action) {
        return Err(response);
    }
    let workspace_id = crate::policies::workspace_id_from_headers(headers)?;
    let user_id = match user {
        Some(Extension(context)) => context.user_id,
        None if internal.is_some() => forwarded_user_id(headers).ok_or_else(|| {
            api_error_response(
                StatusCode::FORBIDDEN,
                ApiErrorCode::Forbidden,
                format!("signed-in user context is required to {action}"),
            )
        })?,
        None => forwarded_user_id(headers).ok_or_else(|| {
            api_error_response(
                StatusCode::UNAUTHORIZED,
                ApiErrorCode::Unauthorized,
                format!("authenticated user is required to {action}"),
            )
        })?,
    };
    let is_platform_admin = team_store
        .is_platform_admin(user_id)
        .await
        .map_err(|error| {
            api_error_response(
                StatusCode::INTERNAL_SERVER_ERROR,
                ApiErrorCode::Internal,
                error.to_string(),
            )
        })?;
    if is_platform_admin {
        tracing::info!(
            user_id = %user_id,
            workspace_id = %workspace_id,
            action,
            "platform administrator used cross-workspace member access"
        );
        return match team_store.get_workspace(&workspace_id).await {
            Ok(workspace) => Ok(workspace),
            Err(TeamStoreError::NotFound) => Err(workspace_membership_required(action)),
            Err(error) => Err(api_error_response(
                StatusCode::INTERNAL_SERVER_ERROR,
                ApiErrorCode::Internal,
                error.to_string(),
            )),
        };
    }
    team_store
        .list_workspaces_for_user(user_id)
        .await
        .map_err(|error| {
            api_error_response(
                StatusCode::INTERNAL_SERVER_ERROR,
                ApiErrorCode::Internal,
                error.to_string(),
            )
        })?
        .into_iter()
        .find(|workspace| workspace.id == workspace_id)
        .ok_or_else(|| workspace_membership_required(action))
}

fn workspace_membership_required(action: &str) -> Response {
    api_error_response(
        StatusCode::FORBIDDEN,
        ApiErrorCode::Forbidden,
        format!("workspace membership is required to {action}"),
    )
}

async fn require_admin_role(
    team_store: &Arc<dyn TeamStore>,
    workspace_id: &str,
    user_id: Uuid,
    action: &str,
) -> Result<(), Response> {
    if team_store
        .is_platform_admin(user_id)
        .await
        .map_err(|error| {
            api_error_response(
                StatusCode::INTERNAL_SERVER_ERROR,
                ApiErrorCode::Internal,
                error.to_string(),
            )
        })?
    {
        tracing::info!(
            user_id = %user_id,
            workspace_id,
            action,
            "platform administrator passed workspace admin gate"
        );
        return Ok(());
    }

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
        .get("x-featherlane-ai-user-id")
        .and_then(|value| value.to_str().ok())
        .and_then(|value| Uuid::parse_str(value.trim()).ok())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::team::{MemoryTeamStore, TeamStoreError};

    #[tokio::test]
    async fn platform_admin_passes_admin_gate_without_becoming_workspace_owner() {
        let store = Arc::new(MemoryTeamStore::new());
        let owner_id = Uuid::new_v4();
        let platform_admin_id = Uuid::new_v4();
        let workspace = store
            .create_workspace(owner_id, "Customer Workspace")
            .await
            .expect("workspace");
        let team_store: Arc<dyn TeamStore> = store.clone();

        assert!(require_admin_role(
            &team_store,
            &workspace.id,
            platform_admin_id,
            "inspect workspace"
        )
        .await
        .is_err());

        store
            .set_platform_admin_for_tests(platform_admin_id, true)
            .await;
        assert!(require_admin_role(
            &team_store,
            &workspace.id,
            platform_admin_id,
            "inspect workspace"
        )
        .await
        .is_ok());
        assert!(matches!(
            store
                .delete_workspace(platform_admin_id, &workspace.id)
                .await,
            Err(TeamStoreError::Forbidden)
        ));
    }
}
