use std::time::Duration;

use axum::{
    routing::{get, patch, post},
    Router,
};

use crate::{gateway, AppState};

pub(super) fn gateway_routes(state: &AppState, gateway_seal_key: [u8; 32]) -> Router {
    Router::new()
        .route(
            "/v1/gateway/provider-connections",
            get(gateway::list_gateway_provider_connections)
                .post(gateway::create_gateway_provider_connection),
        )
        .route(
            "/v1/gateway/provider-connections/:id",
            patch(gateway::patch_gateway_provider_connection),
        )
        .route(
            "/v1/enforcement-profiles",
            get(gateway::list_enforcement_profiles).post(gateway::create_enforcement_profile),
        )
        .route(
            "/v1/enforcement-profiles/:id",
            patch(gateway::patch_enforcement_profile),
        )
        .route(
            "/v1/gateway/routes",
            get(gateway::list_gateway_routes).post(gateway::create_gateway_route),
        )
        .route(
            "/v1/gateway/routes/:id",
            patch(gateway::patch_gateway_route),
        )
        .route(
            "/v1/gateway/:route_id/openai/chat/completions",
            post(gateway::proxy_openai_chat_completions),
        )
        .route(
            "/v1/gateway/:route_id/anthropic/v1/messages",
            post(gateway::proxy_anthropic_messages),
        )
        .with_state(gateway::GatewayState {
            app: state.clone(),
            store: state.gateway_store.clone(),
            http: build_gateway_http_client(),
            seal_key: gateway_seal_key,
        })
}

fn build_gateway_http_client() -> reqwest::Client {
    reqwest::Client::builder()
        .connect_timeout(Duration::from_secs(10))
        .timeout(Duration::from_secs(120))
        .build()
        .expect("gateway HTTP client")
}
