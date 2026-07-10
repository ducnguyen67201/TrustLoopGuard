use axum::{
    extract::{Path, State},
    http::HeaderMap,
    response::Response,
    Extension,
};
use bytes::Bytes;
#[allow(unused_imports)]
use tl_core::{ApiError, GatewayProviderKind};

use super::GatewayState;
use crate::gateway::provider::{AnthropicGatewayProvider, OpenAiCompatibleGatewayProvider};
use crate::gateway::service::proxy_provider_request;

#[utoipa::path(
    post,
    path = "/v1/gateway/{route_id}/openai/chat/completions",
    tag = "gateway",
    params(("route_id" = String, Path, description = "Gateway route id")),
    request_body(content = String, content_type = "application/json", description = "OpenAI-compatible request. With an active LLM budget, bounded requests receive strict preflight admission; requests without max_tokens are soft-admitted while current spend remains below the cap."),
    responses(
        (status = 200, description = "OpenAI-compatible chat completion response"),
        (status = 400, description = "Unsupported or malformed request", body = ApiError),
        (status = 404, description = "Gateway route not found", body = ApiError),
        (status = 429, description = "The request's reserved maximum would exceed the budget, or an unbounded request arrived after the cap was reached"),
        (status = 502, description = "Provider request failed", body = ApiError),
        (status = 503, description = "No trusted price is configured for the requested model"),
    ),
)]
pub async fn proxy_openai_chat_completions(
    State(state): State<GatewayState>,
    headers: HeaderMap,
    Path(route_id): Path<String>,
    runtime_key: Option<Extension<crate::auth::WorkspaceKeyContext>>,
    body: Bytes,
) -> Response {
    proxy_provider_request(
        state,
        headers,
        route_id,
        body,
        GatewayProviderKind::OpenaiCompatible,
        OpenAiCompatibleGatewayProvider,
        runtime_key.map(|Extension(key)| key),
    )
    .await
}

#[utoipa::path(
    post,
    path = "/v1/gateway/{route_id}/anthropic/v1/messages",
    tag = "gateway",
    params(("route_id" = String, Path, description = "Gateway route id")),
    request_body(content = String, content_type = "application/json", description = "Raw provider request body, proxied through unchanged"),
    responses(
        (status = 200, description = "Anthropic messages response"),
        (status = 400, description = "Unsupported or malformed request", body = ApiError),
        (status = 404, description = "Gateway route not found", body = ApiError),
        (status = 502, description = "Provider request failed", body = ApiError),
    ),
)]
pub async fn proxy_anthropic_messages(
    State(state): State<GatewayState>,
    headers: HeaderMap,
    Path(route_id): Path<String>,
    runtime_key: Option<Extension<crate::auth::WorkspaceKeyContext>>,
    body: Bytes,
) -> Response {
    proxy_provider_request(
        state,
        headers,
        route_id,
        body,
        GatewayProviderKind::Anthropic,
        AnthropicGatewayProvider,
        runtime_key.map(|Extension(key)| key),
    )
    .await
}
