use axum::{
    extract::{Extension, Path, State},
    http::{HeaderMap, StatusCode},
    response::{IntoResponse, Response},
    Json,
};
use tl_core::{
    ApiErrorCode, CreateMcpGatewayConnectionRequest, McpGatewayAuthKind, McpGatewayConnectInfo,
    McpGatewayConnectionListResponse, McpGatewaySyncResponse, McpGatewayToolAssignmentsResponse,
    McpGatewayToolListResponse, ReplaceMcpGatewayToolAssignmentsRequest,
    UpdateMcpGatewayConnectionRequest, UpdateMcpGatewayToolRequest,
};
use uuid::Uuid;

use crate::{
    auth::{InternalServiceContext, WorkspaceKeyContext},
    dashboard_admin::{authorize_workspace_admin, authorize_workspace_member},
    jwt::UserContext,
};

use super::{
    naming::normalize_server_slug,
    service::error_response,
    upstream::{sync_catalog, validate_endpoint_url},
    CredentialPatch, McpConnectionPatch, McpGatewayState, McpGatewayStoreError, NewMcpConnection,
};

#[utoipa::path(get, path = "/v1/mcp-gateway/connect-info", tag = "mcp-gateway", responses((status = 200, body = McpGatewayConnectInfo), (status = 403, body = tl_core::ApiError)))]
pub async fn connect_info(
    State(state): State<McpGatewayState>,
    headers: HeaderMap,
    user: Option<Extension<UserContext>>,
    internal: Option<Extension<InternalServiceContext>>,
    runtime: Option<Extension<WorkspaceKeyContext>>,
) -> Response {
    let workspace = match authorize_workspace_member(
        &state.app.team_store,
        &headers,
        user,
        internal,
        runtime,
        "view MCP connection details",
    )
    .await
    {
        Ok(value) => value,
        Err(response) => return response,
    };
    if !workspace.is_mcp_gateway_enabled {
        return feature_disabled();
    }
    let environment_id = match state
        .app
        .environment_store
        .default_environment_id(&workspace.id)
        .await
    {
        Ok(value) => value,
        Err(error) => {
            tracing::error!(workspace_id = %workspace.id, error = %error, "default MCP environment lookup failed");
            return error_response(
                StatusCode::INTERNAL_SERVER_ERROR,
                ApiErrorCode::Internal,
                "default environment lookup failed",
            );
        }
    };
    let environment = match state
        .app
        .environment_store
        .get(&workspace.id, &environment_id)
        .await
    {
        Ok(value) => value,
        Err(error) => {
            tracing::error!(workspace_id = %workspace.id, error = %error, "default MCP environment load failed");
            return error_response(
                StatusCode::INTERNAL_SERVER_ERROR,
                ApiErrorCode::Internal,
                "default environment lookup failed",
            );
        }
    };
    Json(McpGatewayConnectInfo {
        resource_url: crate::oauth::mcp_resource_url(),
        scope: crate::oauth::MCP_SCOPE.into(),
        oauth_configured: state.app.jwt_signer.is_some(),
        default_environment_id: environment.id,
        default_environment_name: environment.name,
    })
    .into_response()
}

#[utoipa::path(get, path = "/v1/mcp-gateway/connections", tag = "mcp-gateway", responses((status = 200, body = McpGatewayConnectionListResponse)))]
pub async fn list_connections(
    State(state): State<McpGatewayState>,
    headers: HeaderMap,
    user: Option<Extension<UserContext>>,
    internal: Option<Extension<InternalServiceContext>>,
    runtime: Option<Extension<WorkspaceKeyContext>>,
) -> Response {
    let workspace_id = match authorize_admin(
        &state,
        &headers,
        user,
        internal,
        runtime,
        "manage MCP servers",
    )
    .await
    {
        Ok(value) => value,
        Err(response) => return response,
    };
    match state.store.list_connections(&workspace_id).await {
        Ok(connections) => Json(McpGatewayConnectionListResponse { connections }).into_response(),
        Err(error) => store_error(error),
    }
}

#[utoipa::path(post, path = "/v1/mcp-gateway/connections", tag = "mcp-gateway", request_body = CreateMcpGatewayConnectionRequest, responses((status = 201, body = tl_core::McpGatewayConnection)))]
pub async fn create_connection(
    State(state): State<McpGatewayState>,
    headers: HeaderMap,
    user: Option<Extension<UserContext>>,
    internal: Option<Extension<InternalServiceContext>>,
    runtime: Option<Extension<WorkspaceKeyContext>>,
    Json(input): Json<CreateMcpGatewayConnectionRequest>,
) -> Response {
    let workspace_id = match authorize_admin(
        &state,
        &headers,
        user,
        internal,
        runtime,
        "manage MCP servers",
    )
    .await
    {
        Ok(value) => value,
        Err(response) => return response,
    };
    if let Err(message) = validate_display_name(&input.display_name) {
        return invalid(message);
    }
    let slug = match normalize_server_slug(&input.server_slug) {
        Ok(value) => value,
        Err(message) => return invalid(message),
    };
    if let Err(message) = validate_endpoint_url(&input.endpoint_url) {
        return invalid(message);
    }
    let credential = match seal_credential(input.auth_kind, input.credential, &state.seal_key) {
        Ok(value) => value,
        Err(message) => return invalid(message),
    };
    match state
        .store
        .create_connection(NewMcpConnection {
            workspace_id,
            id: Uuid::new_v4(),
            display_name: input.display_name.trim().to_string(),
            server_slug: slug,
            endpoint_url: input.endpoint_url,
            auth_kind: input.auth_kind,
            encrypted_credential: credential,
            enabled: true,
        })
        .await
    {
        Ok(connection) => (StatusCode::CREATED, Json(connection)).into_response(),
        Err(error) => store_error(error),
    }
}

#[utoipa::path(patch, path = "/v1/mcp-gateway/connections/{id}", tag = "mcp-gateway", params(("id" = String, Path)), request_body = UpdateMcpGatewayConnectionRequest, responses((status = 200, body = tl_core::McpGatewayConnection)))]
pub async fn patch_connection(
    State(state): State<McpGatewayState>,
    headers: HeaderMap,
    Path(id): Path<String>,
    user: Option<Extension<UserContext>>,
    internal: Option<Extension<InternalServiceContext>>,
    runtime: Option<Extension<WorkspaceKeyContext>>,
    Json(input): Json<UpdateMcpGatewayConnectionRequest>,
) -> Response {
    let workspace_id = match authorize_admin(
        &state,
        &headers,
        user,
        internal,
        runtime,
        "manage MCP servers",
    )
    .await
    {
        Ok(value) => value,
        Err(response) => return response,
    };
    let id = match Uuid::parse_str(&id) {
        Ok(value) => value,
        Err(_) => return not_found(),
    };
    if let Some(name) = input.display_name.as_deref() {
        if let Err(message) = validate_display_name(name) {
            return invalid(message);
        }
    }
    if let Some(endpoint) = input.endpoint_url.as_deref() {
        if let Err(message) = validate_endpoint_url(endpoint) {
            return invalid(message);
        }
    }
    let current = match state.store.get_connection_secret(&workspace_id, id).await {
        Ok(value) => value,
        Err(error) => return store_error(error),
    };
    let effective_kind = input.auth_kind.unwrap_or(current.connection.auth_kind);
    let credential = if effective_kind == McpGatewayAuthKind::None {
        CredentialPatch::Clear
    } else if let Some(secret) = input.credential {
        if secret.trim().is_empty() {
            return invalid("credential cannot be empty");
        }
        CredentialPatch::Replace(crate::gateway::seal_provider_key(
            secret.trim(),
            &state.seal_key,
        ))
    } else {
        CredentialPatch::Preserve
    };
    let invalidate_catalog = input
        .endpoint_url
        .as_deref()
        .is_some_and(|value| value != current.connection.endpoint_url)
        || input
            .auth_kind
            .is_some_and(|value| value != current.connection.auth_kind);
    match state
        .store
        .update_connection(
            &workspace_id,
            id,
            McpConnectionPatch {
                display_name: input.display_name.map(|value| value.trim().to_string()),
                endpoint_url: input.endpoint_url,
                auth_kind: input.auth_kind,
                credential,
                enabled: input.enabled,
                invalidate_catalog,
            },
        )
        .await
    {
        Ok(connection) => Json(connection).into_response(),
        Err(error) => store_error(error),
    }
}

#[utoipa::path(delete, path = "/v1/mcp-gateway/connections/{id}", tag = "mcp-gateway", params(("id" = String, Path)), responses((status = 204)))]
pub async fn delete_connection(
    State(state): State<McpGatewayState>,
    headers: HeaderMap,
    Path(id): Path<String>,
    user: Option<Extension<UserContext>>,
    internal: Option<Extension<InternalServiceContext>>,
    runtime: Option<Extension<WorkspaceKeyContext>>,
) -> Response {
    let workspace_id = match authorize_admin(
        &state,
        &headers,
        user,
        internal,
        runtime,
        "manage MCP servers",
    )
    .await
    {
        Ok(value) => value,
        Err(response) => return response,
    };
    let id = match Uuid::parse_str(&id) {
        Ok(value) => value,
        Err(_) => return not_found(),
    };
    match state.store.delete_connection(&workspace_id, id).await {
        Ok(()) => StatusCode::NO_CONTENT.into_response(),
        Err(error) => store_error(error),
    }
}

#[utoipa::path(post, path = "/v1/mcp-gateway/connections/{id}/sync", tag = "mcp-gateway", params(("id" = String, Path)), responses((status = 200, body = McpGatewaySyncResponse), (status = 502, body = tl_core::ApiError)))]
pub async fn sync_connection(
    State(state): State<McpGatewayState>,
    headers: HeaderMap,
    Path(id): Path<String>,
    user: Option<Extension<UserContext>>,
    internal: Option<Extension<InternalServiceContext>>,
    runtime: Option<Extension<WorkspaceKeyContext>>,
) -> Response {
    let workspace_id = match authorize_admin(
        &state,
        &headers,
        user,
        internal,
        runtime,
        "sync MCP servers",
    )
    .await
    {
        Ok(value) => value,
        Err(response) => return response,
    };
    let id = match Uuid::parse_str(&id) {
        Ok(value) => value,
        Err(_) => return not_found(),
    };
    let secret = match state.store.get_connection_secret(&workspace_id, id).await {
        Ok(value) => value,
        Err(error) => return store_error(error),
    };
    let bearer = match secret.encrypted_credential.as_deref() {
        Some(value) => match crate::gateway::unseal_provider_key(value, &state.seal_key) {
            Ok(value) => Some(value),
            Err(error) => {
                tracing::error!(workspace_id, connection_id = %id, error = %error, "MCP credential unseal failed");
                return error_response(
                    StatusCode::INTERNAL_SERVER_ERROR,
                    ApiErrorCode::Internal,
                    "server credential could not be read",
                );
            }
        },
        None => None,
    };
    match sync_catalog(
        &secret.connection.endpoint_url,
        bearer.as_deref(),
        &secret.connection.server_slug,
        id,
    )
    .await
    {
        Ok(tools) => match state
            .store
            .replace_catalog_snapshot(&workspace_id, id, tools)
            .await
        {
            Ok(connection) => {
                let count = connection.tool_count;
                Json(McpGatewaySyncResponse {
                    connection,
                    tool_count: count,
                })
                .into_response()
            }
            Err(error) => store_error(error),
        },
        Err(error) => {
            tracing::warn!(workspace_id, connection_id = %id, error = %error, "MCP catalog sync failed");
            let _ = state
                .store
                .record_sync_failure(
                    &workspace_id,
                    id,
                    "Upstream server could not be synchronized",
                )
                .await;
            error_response(
                StatusCode::BAD_GATEWAY,
                ApiErrorCode::Unavailable,
                "Upstream server could not be synchronized",
            )
        }
    }
}

#[utoipa::path(get, path = "/v1/mcp-gateway/tools", tag = "mcp-gateway", responses((status = 200, body = McpGatewayToolListResponse)))]
pub async fn list_tools(
    State(state): State<McpGatewayState>,
    headers: HeaderMap,
    user: Option<Extension<UserContext>>,
    internal: Option<Extension<InternalServiceContext>>,
    runtime: Option<Extension<WorkspaceKeyContext>>,
) -> Response {
    let workspace_id = match authorize_admin(
        &state,
        &headers,
        user,
        internal,
        runtime,
        "manage MCP tool access",
    )
    .await
    {
        Ok(value) => value,
        Err(response) => return response,
    };
    match state.store.list_tools(&workspace_id).await {
        Ok(tools) => Json(McpGatewayToolListResponse { tools }).into_response(),
        Err(error) => store_error(error),
    }
}
#[utoipa::path(patch, path = "/v1/mcp-gateway/tools/{id}", tag = "mcp-gateway", params(("id" = String, Path)), request_body = UpdateMcpGatewayToolRequest, responses((status = 200, body = tl_core::McpGatewayTool)))]
pub async fn patch_tool(
    State(state): State<McpGatewayState>,
    headers: HeaderMap,
    Path(id): Path<String>,
    user: Option<Extension<UserContext>>,
    internal: Option<Extension<InternalServiceContext>>,
    runtime: Option<Extension<WorkspaceKeyContext>>,
    Json(input): Json<UpdateMcpGatewayToolRequest>,
) -> Response {
    let workspace_id = match authorize_admin(
        &state,
        &headers,
        user,
        internal,
        runtime,
        "classify MCP tools",
    )
    .await
    {
        Ok(value) => value,
        Err(response) => return response,
    };
    let id = match Uuid::parse_str(&id) {
        Ok(value) => value,
        Err(_) => return not_found(),
    };
    match state
        .store
        .update_tool_side_effect(&workspace_id, id, input.side_effect)
        .await
    {
        Ok(tool) => Json(tool).into_response(),
        Err(error) => store_error(error),
    }
}
#[utoipa::path(put, path = "/v1/mcp-gateway/tools/{id}/assignments", tag = "mcp-gateway", params(("id" = String, Path)), request_body = ReplaceMcpGatewayToolAssignmentsRequest, responses((status = 200, body = McpGatewayToolAssignmentsResponse)))]
pub async fn replace_assignments(
    State(state): State<McpGatewayState>,
    headers: HeaderMap,
    Path(id): Path<String>,
    user: Option<Extension<UserContext>>,
    internal: Option<Extension<InternalServiceContext>>,
    runtime: Option<Extension<WorkspaceKeyContext>>,
    Json(input): Json<ReplaceMcpGatewayToolAssignmentsRequest>,
) -> Response {
    let (workspace_id, created_by) = match authorize_admin_with_actor(
        &state,
        &headers,
        user,
        internal,
        runtime,
        "assign MCP tools",
    )
    .await
    {
        Ok(value) => value,
        Err(response) => return response,
    };
    let id = match Uuid::parse_str(&id) {
        Ok(value) => value,
        Err(_) => return not_found(),
    };
    if input.user_ids.len() > 500 {
        return invalid("at most 500 user assignments are allowed");
    }
    let mut users = Vec::with_capacity(input.user_ids.len());
    for value in input.user_ids {
        match Uuid::parse_str(&value) {
            Ok(id) => users.push(id),
            Err(_) => return invalid("every user_id must be a UUID"),
        }
    }
    match state
        .store
        .replace_assignments(&workspace_id, id, users, created_by)
        .await
    {
        Ok(user_ids) => Json(McpGatewayToolAssignmentsResponse {
            tool_id: id.to_string(),
            user_ids: user_ids.into_iter().map(|id| id.to_string()).collect(),
        })
        .into_response(),
        Err(error) => store_error(error),
    }
}

async fn authorize_admin(
    state: &McpGatewayState,
    headers: &HeaderMap,
    user: Option<Extension<UserContext>>,
    internal: Option<Extension<InternalServiceContext>>,
    runtime: Option<Extension<WorkspaceKeyContext>>,
    action: &str,
) -> Result<String, Response> {
    authorize_admin_with_actor(state, headers, user, internal, runtime, action)
        .await
        .map(|value| value.0)
}
async fn authorize_admin_with_actor(
    state: &McpGatewayState,
    headers: &HeaderMap,
    user: Option<Extension<UserContext>>,
    internal: Option<Extension<InternalServiceContext>>,
    runtime: Option<Extension<WorkspaceKeyContext>>,
    action: &str,
) -> Result<(String, Option<Uuid>), Response> {
    let (workspace_id, actor) = authorize_workspace_admin(
        &state.app.team_store,
        headers,
        user,
        internal,
        runtime,
        action,
    )
    .await?;
    let workspace = state
        .app
        .team_store
        .list_workspaces_for_user(actor.ok_or_else(|| {
            error_response(
                StatusCode::FORBIDDEN,
                ApiErrorCode::Forbidden,
                "signed-in user context is required",
            )
        })?)
        .await
        .map_err(|_| {
            error_response(
                StatusCode::INTERNAL_SERVER_ERROR,
                ApiErrorCode::Internal,
                "workspace membership lookup failed",
            )
        })?
        .into_iter()
        .find(|workspace| workspace.id == workspace_id)
        .ok_or_else(|| {
            error_response(
                StatusCode::FORBIDDEN,
                ApiErrorCode::Forbidden,
                "workspace membership is required",
            )
        })?;
    if !workspace.is_mcp_gateway_enabled {
        return Err(feature_disabled());
    }
    Ok((workspace_id, actor))
}

fn seal_credential(
    kind: McpGatewayAuthKind,
    credential: Option<String>,
    seal_key: &[u8; 32],
) -> Result<Option<String>, &'static str> {
    match kind {
        McpGatewayAuthKind::None => {
            if credential
                .as_deref()
                .is_some_and(|value| !value.trim().is_empty())
            {
                Err("credential is not allowed when auth_kind is none")
            } else {
                Ok(None)
            }
        }
        McpGatewayAuthKind::StaticBearer => {
            let value = credential
                .as_deref()
                .map(str::trim)
                .filter(|value| !value.is_empty())
                .ok_or("credential is required for static bearer authentication")?;
            Ok(Some(crate::gateway::seal_provider_key(value, seal_key)))
        }
    }
}
fn validate_display_name(value: &str) -> Result<(), &'static str> {
    let length = value.trim().chars().count();
    if length == 0 || length > 100 {
        Err("display_name must be 1-100 characters")
    } else {
        Ok(())
    }
}
fn feature_disabled() -> Response {
    error_response(
        StatusCode::FORBIDDEN,
        ApiErrorCode::Forbidden,
        "MCP access is not enabled for this workspace",
    )
}
fn invalid(message: &str) -> Response {
    error_response(
        StatusCode::UNPROCESSABLE_ENTITY,
        ApiErrorCode::Unprocessable,
        message,
    )
}
fn not_found() -> Response {
    error_response(
        StatusCode::NOT_FOUND,
        ApiErrorCode::NotFound,
        "MCP gateway resource not found",
    )
}
fn store_error(error: McpGatewayStoreError) -> Response {
    match error {
        McpGatewayStoreError::NotFound => not_found(),
        McpGatewayStoreError::Conflict(message) => {
            error_response(StatusCode::CONFLICT, ApiErrorCode::Unprocessable, &message)
        }
        McpGatewayStoreError::Internal(message) => {
            tracing::error!(error = %message, "MCP gateway store failed");
            error_response(
                StatusCode::INTERNAL_SERVER_ERROR,
                ApiErrorCode::Internal,
                "MCP gateway operation failed",
            )
        }
    }
}
