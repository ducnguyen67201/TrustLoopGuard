use axum::{http::StatusCode, response::Response};
use bytes::Bytes;
use serde_json::Value;
use tl_core::{EnforcementProfile, ResponseMode};

use super::super::errors::api_error_response;
use super::super::provider::GatewayProvider;

const MAX_BODY_BYTES: usize = 4 * 1024 * 1024;

#[allow(clippy::result_large_err)]
pub(super) fn parse_provider_request(body: &Bytes) -> Result<Value, Response> {
    if body.len() > MAX_BODY_BYTES {
        return Err(api_error_response(
            StatusCode::BAD_REQUEST,
            format!("request body exceeds maximum size of {MAX_BODY_BYTES} bytes"),
        ));
    }

    serde_json::from_slice::<Value>(body).map_err(|error| {
        api_error_response(
            StatusCode::BAD_REQUEST,
            format!("provider request body must be JSON: {error}"),
        )
    })
}

#[allow(clippy::result_large_err)]
pub(super) fn prepare_streaming_request<P: GatewayProvider>(
    provider: &P,
    profile: &EnforcementProfile,
    request: &mut Value,
) -> Result<bool, Response> {
    let wants_stream = provider.is_streaming(request);
    if !wants_stream {
        return Ok(false);
    }
    if profile.response_mode != ResponseMode::Streaming {
        return Err(api_error_response(
            StatusCode::BAD_REQUEST,
            "streaming is not enabled for this route; set the enforcement profile response_mode to \"streaming\"".into(),
        ));
    }
    provider.strip_streaming_fields(request);
    Ok(true)
}
