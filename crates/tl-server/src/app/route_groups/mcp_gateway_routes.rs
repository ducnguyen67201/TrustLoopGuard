use std::sync::Arc;

use axum::{
    middleware::from_fn_with_state,
    routing::{get, patch, post, put},
    Router,
};
use rmcp::transport::streamable_http_server::{
    session::local::LocalSessionManager, StreamableHttpServerConfig, StreamableHttpService,
};
use tower_http::limit::RequestBodyLimitLayer;

use crate::{auth, mcp_gateway, AppState, AuthConfig};

pub(crate) fn mcp_gateway_routes(state: &AppState, seal_key: [u8; 32]) -> Router {
    Router::new()
        .route(
            "/v1/mcp-gateway/connect-info",
            get(mcp_gateway::connect_info),
        )
        .route(
            "/v1/mcp-gateway/connections",
            get(mcp_gateway::list_connections).post(mcp_gateway::create_connection),
        )
        .route(
            "/v1/mcp-gateway/connections/:id",
            patch(mcp_gateway::patch_connection).delete(mcp_gateway::delete_connection),
        )
        .route(
            "/v1/mcp-gateway/connections/:id/sync",
            post(mcp_gateway::sync_connection),
        )
        .route("/v1/mcp-gateway/tools", get(mcp_gateway::list_tools))
        .route("/v1/mcp-gateway/tools/:id", patch(mcp_gateway::patch_tool))
        .route(
            "/v1/mcp-gateway/tools/:id/assignments",
            put(mcp_gateway::replace_assignments),
        )
        .with_state(mcp_gateway::McpGatewayState {
            app: state.clone(),
            store: state.mcp_gateway_store.clone(),
            seal_key,
        })
}

pub(crate) fn mcp_resource_routes(
    state: &AppState,
    auth_config: Arc<AuthConfig>,
    seal_key: [u8; 32],
) -> Router {
    let public_url = url::Url::parse(&crate::oauth::issuer())
        .expect("TL_PUBLIC_URL must be an absolute URL when MCP is enabled");
    let host = public_url
        .port()
        .map(|port| format!("{}:{port}", public_url.host_str().expect("public URL host")))
        .unwrap_or_else(|| public_url.host_str().expect("public URL host").to_string());
    let origin = format!("{}://{}", public_url.scheme(), host);
    let handler = mcp_gateway::HostedMcpHandler::new(
        state.clone(),
        state.mcp_gateway_store.clone(),
        seal_key,
    );
    let service: StreamableHttpService<_, LocalSessionManager> = StreamableHttpService::new(
        move || Ok(handler.clone()),
        Default::default(),
        StreamableHttpServerConfig::default()
            .with_stateful_mode(false)
            .with_json_response(true)
            .with_allowed_hosts(vec![host])
            .with_allowed_origins(vec![origin]),
    );
    Router::new()
        .nest_service("/mcp", service)
        .layer(RequestBodyLimitLayer::new(512 * 1024))
        .layer(from_fn_with_state(
            state.clone(),
            mcp_gateway::require_mcp_workspace_access,
        ))
        .layer(from_fn_with_state(auth_config, auth::require_mcp_bearer))
}
