use axum::{extract::Extension, http::HeaderMap, response::Response};

use super::AnalyticsState;
use crate::{
    auth::{InternalServiceContext, WorkspaceKeyContext},
    dashboard_admin::authorize_workspace_member,
    jwt::UserContext,
};

pub(super) async fn authorize_analytics_workspace(
    state: &AnalyticsState,
    headers: &HeaderMap,
    user: Option<Extension<UserContext>>,
    internal: Option<Extension<InternalServiceContext>>,
    runtime_key: Option<Extension<WorkspaceKeyContext>>,
) -> Result<String, Response> {
    // The no-auth memory boot path intentionally exposes analytics locally.
    // Hosted deployments reject missing bearer auth before reaching this handler.
    if user.is_none() && internal.is_none() && runtime_key.is_none() {
        return crate::policies::workspace_id_from_headers(headers);
    }

    let workspace = authorize_workspace_member(
        &state.team_store,
        headers,
        user,
        internal,
        runtime_key,
        "access analytics dashboard endpoints",
    )
    .await?;
    Ok(workspace.id)
}
