use std::sync::Arc;

use axum::{http::StatusCode, response::Response, Extension};

use crate::AppState;

pub(crate) mod provider_connections;
pub(crate) mod proxy;
pub(crate) mod routes;

pub use provider_connections::{
    __path_create_gateway_provider_connection, __path_list_gateway_provider_connections,
    __path_patch_gateway_provider_connection, create_gateway_provider_connection,
    list_gateway_provider_connections, patch_gateway_provider_connection,
};
pub use proxy::{
    __path_proxy_anthropic_messages, __path_proxy_openai_chat_completions,
    proxy_anthropic_messages, proxy_openai_chat_completions,
};
pub use routes::{
    __path_create_gateway_route, __path_list_gateway_routes, __path_patch_gateway_route,
    create_gateway_route, list_gateway_routes, patch_gateway_route,
};

use super::errors::api_error_response;
use super::store::GatewayStore;

#[derive(Clone)]
pub struct GatewayState {
    pub app: AppState,
    pub store: Arc<dyn GatewayStore>,
    pub http: reqwest::Client,
    pub seal_key: [u8; 32],
}

pub(super) fn reject_runtime_key_config_access(
    runtime_key: Option<Extension<crate::auth::WorkspaceKeyContext>>,
) -> Option<Response> {
    runtime_key.map(|_| {
        api_error_response(
            StatusCode::FORBIDDEN,
            "workspace runtime keys cannot manage gateway configuration".into(),
        )
    })
}
