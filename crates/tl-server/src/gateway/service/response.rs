use axum::{
    http::{header, HeaderName, HeaderValue, StatusCode},
    response::{IntoResponse, Response},
    Json,
};
use serde_json::Value;

use super::super::errors::api_error_response;
use super::super::provider::GatewayProvider;
use super::super::provider::ProviderError;

pub(super) struct EnforcementHeaders<'a> {
    pub effect: &'static str,
    pub trace_id: &'a str,
    pub phase: &'static str,
    pub policy_id: Option<&'a str>,
}

pub(super) fn finalize_gateway_response<P: GatewayProvider>(
    provider: &P,
    wants_stream: bool,
    body: Value,
    headers: Option<EnforcementHeaders<'_>>,
) -> Response {
    let mut response = if wants_stream {
        let mut response = provider.streaming_sse_body(&body).into_response();
        let headers = response.headers_mut();
        headers.insert(
            header::CONTENT_TYPE,
            HeaderValue::from_static("text/event-stream"),
        );
        headers.insert(
            HeaderName::from_static("cache-control"),
            HeaderValue::from_static("no-cache"),
        );
        response
    } else {
        Json(body).into_response()
    };

    if let Some(headers) = headers {
        apply_enforcement_headers(&mut response, &headers);
    }
    response
}

pub(super) fn handle_provider_failure(error: ProviderError) -> Response {
    tracing::warn!(
        code = error.code,
        status = error.status,
        "upstream provider request failed"
    );
    api_error_response(
        StatusCode::BAD_GATEWAY,
        "upstream provider request failed".into(),
    )
}

fn apply_enforcement_headers(response: &mut Response, headers: &EnforcementHeaders<'_>) {
    let response_headers = response.headers_mut();
    response_headers.insert(
        HeaderName::from_static("x-featherlane-ai-effect"),
        HeaderValue::from_static(headers.effect),
    );
    response_headers.insert(
        HeaderName::from_static("x-featherlane-ai-trace-id"),
        HeaderValue::from_str(headers.trace_id).unwrap_or_else(|_| HeaderValue::from_static("")),
    );
    response_headers.insert(
        HeaderName::from_static("x-featherlane-ai-phase"),
        HeaderValue::from_static(headers.phase),
    );

    if let Some(policy_id) = headers.policy_id {
        if let Ok(value) = HeaderValue::from_str(policy_id) {
            response_headers.insert(HeaderName::from_static("x-featherlane-ai-policy-id"), value);
        }
    }
}
