use std::sync::Arc;

use axum::{
    extract::Extension,
    http::{HeaderMap, StatusCode},
    response::Response,
};
use tl_core::ApiErrorCode;
use uuid::Uuid;

use super::response::api_error_response;
use super::AnalyticsState;
use crate::{
    auth::{InternalServiceContext, WorkspaceKeyContext},
    jwt::UserContext,
    team::{TeamStore, TeamStoreError},
};

pub(super) async fn authorize_analytics_workspace(
    state: &AnalyticsState,
    headers: &HeaderMap,
    user: Option<Extension<UserContext>>,
    internal: Option<Extension<InternalServiceContext>>,
    runtime_key: Option<Extension<WorkspaceKeyContext>>,
) -> Result<String, Response> {
    if runtime_key.is_some() {
        return Err(api_error_response(
            StatusCode::FORBIDDEN,
            ApiErrorCode::Forbidden,
            "workspace runtime keys cannot access analytics dashboard endpoints".to_string(),
        ));
    }

    let workspace_id = crate::policies::workspace_id_from_headers(headers);
    match analytics_user_id(headers, user, internal) {
        AnalyticsUserId::Some(user_id) => {
            require_workspace_member(&state.team_store, &workspace_id, user_id).await?;
        }
        AnalyticsUserId::MissingInternalUser => {
            return Err(api_error_response(
                StatusCode::FORBIDDEN,
                ApiErrorCode::Forbidden,
                "signed-in user context is required to access analytics dashboard endpoints"
                    .to_string(),
            ));
        }
        AnalyticsUserId::None => {}
    }

    Ok(workspace_id)
}

enum AnalyticsUserId {
    Some(Uuid),
    MissingInternalUser,
    None,
}

fn analytics_user_id(
    headers: &HeaderMap,
    user: Option<Extension<UserContext>>,
    internal: Option<Extension<InternalServiceContext>>,
) -> AnalyticsUserId {
    if let Some(Extension(user)) = user {
        return AnalyticsUserId::Some(user.user_id);
    }

    if internal.is_some() {
        return forwarded_user_id(headers)
            .map(AnalyticsUserId::Some)
            .unwrap_or(AnalyticsUserId::MissingInternalUser);
    }

    AnalyticsUserId::None
}

async fn require_workspace_member(
    team_store: &Arc<dyn TeamStore>,
    workspace_id: &str,
    user_id: Uuid,
) -> Result<(), Response> {
    let members = team_store
        .list_members(workspace_id)
        .await
        .map_err(team_error)?;
    let user_id = user_id.to_string();
    if members.iter().any(|member| member.user_id == user_id) {
        Ok(())
    } else {
        Err(api_error_response(
            StatusCode::FORBIDDEN,
            ApiErrorCode::Forbidden,
            "workspace membership is required to access analytics dashboard endpoints".to_string(),
        ))
    }
}

fn forwarded_user_id(headers: &HeaderMap) -> Option<Uuid> {
    headers
        .get("x-tlg-user-id")
        .and_then(|value| value.to_str().ok())
        .and_then(|value| Uuid::parse_str(value.trim()).ok())
}

fn team_error(error: TeamStoreError) -> Response {
    api_error_response(
        StatusCode::INTERNAL_SERVER_ERROR,
        ApiErrorCode::Internal,
        error.to_string(),
    )
}
