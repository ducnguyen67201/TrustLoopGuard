use axum::{
    http::{header, HeaderMap, StatusCode},
    response::Response,
};
use tl_core::{AgentProfile, ApiErrorCode};

use super::response::api_error_response;

// `Response` is intentionally returned on the Err arm: callers short-circuit by
// returning it directly, and boxing would push allocation into the request path.
#[allow(clippy::result_large_err)]
pub(super) fn parse_body(
    headers: &HeaderMap,
    body: &[u8],
) -> Result<(AgentProfile, String), Response> {
    let content_type = headers
        .get(header::CONTENT_TYPE)
        .and_then(|v| v.to_str().ok())
        .unwrap_or("application/json")
        .to_ascii_lowercase();

    let raw = std::str::from_utf8(body).map_err(|e| {
        api_error_response(
            StatusCode::BAD_REQUEST,
            ApiErrorCode::Invalid,
            format!("body is not valid UTF-8: {e}"),
        )
    })?;

    if is_yaml_content_type(&content_type) {
        let profile = tl_policy::load_agent_str(raw).map_err(|e| {
            api_error_response(
                StatusCode::BAD_REQUEST,
                ApiErrorCode::Invalid,
                format!("yaml: {e}"),
            )
        })?;
        Ok((profile, raw.to_string()))
    } else {
        let profile: AgentProfile = serde_json::from_str(raw).map_err(|e| {
            api_error_response(
                StatusCode::BAD_REQUEST,
                ApiErrorCode::Invalid,
                format!("json: {e}"),
            )
        })?;
        let yaml = serde_yaml::to_string(&profile).map_err(|e| {
            api_error_response(
                StatusCode::INTERNAL_SERVER_ERROR,
                ApiErrorCode::Internal,
                format!("yaml serialize: {e}"),
            )
        })?;
        Ok((profile, yaml))
    }
}

pub(super) fn is_yaml_content_type(s: &str) -> bool {
    s.starts_with("application/yaml")
        || s.starts_with("application/x-yaml")
        || s.starts_with("text/yaml")
        || s.starts_with("text/x-yaml")
}

pub(super) fn validate_profile(profile: &AgentProfile) -> Result<(), String> {
    if profile.agent_id.trim().is_empty() {
        return Err("agent_id is required".into());
    }
    if profile.display_name.trim().is_empty() {
        return Err("display_name is required".into());
    }
    if profile.scope.in_scope.is_empty() {
        return Err("scope.in_scope must contain at least one entry".into());
    }
    Ok(())
}
