use std::sync::Arc;

use axum::{
    extract::{Request, State},
    http::StatusCode,
    middleware::Next,
    response::{IntoResponse, Response},
    Json,
};
use tl_core::{ApiError, ApiErrorCode, MyWorkspace};

use crate::{auth::McpAccessContext, AppState};

use super::McpGatewayStore;

#[derive(Clone)]
pub struct McpGatewayState {
    pub app: AppState,
    pub store: Arc<dyn McpGatewayStore>,
    pub seal_key: [u8; 32],
}

pub async fn require_mcp_workspace_access(
    State(app): State<AppState>,
    req: Request,
    next: Next,
) -> Result<Response, Response> {
    let context = req
        .extensions()
        .get::<McpAccessContext>()
        .cloned()
        .ok_or_else(|| {
            error_response(
                StatusCode::UNAUTHORIZED,
                ApiErrorCode::Unauthorized,
                "MCP identity is required",
            )
        })?;
    require_signed_member_feature(&app, &context).await?;
    Ok(next.run(req).await)
}

pub(super) async fn require_signed_member_feature(
    app: &AppState,
    context: &McpAccessContext,
) -> Result<MyWorkspace, Response> {
    let workspace = app.team_store.list_workspaces_for_user(context.user_id).await
        .map_err(|error| { tracing::error!(workspace_id = %context.workspace_id, error = %error, "MCP workspace membership lookup failed"); error_response(StatusCode::INTERNAL_SERVER_ERROR, ApiErrorCode::Internal, "workspace membership lookup failed") })?
        .into_iter().find(|workspace| workspace.id == context.workspace_id)
        .ok_or_else(|| error_response(StatusCode::FORBIDDEN, ApiErrorCode::Forbidden, "workspace membership is required"))?;
    if !workspace.is_mcp_gateway_enabled {
        return Err(error_response(
            StatusCode::FORBIDDEN,
            ApiErrorCode::Forbidden,
            "MCP access is not enabled for this workspace",
        ));
    }
    Ok(workspace)
}

pub(super) fn error_response(status: StatusCode, code: ApiErrorCode, message: &str) -> Response {
    (
        status,
        Json(ApiError {
            code,
            message: message.to_string(),
            retriable: matches!(code, ApiErrorCode::Unavailable | ApiErrorCode::RateLimited),
            details: serde_json::Value::Null,
        }),
    )
        .into_response()
}
