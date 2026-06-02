use axum::http::{header, HeaderMap, StatusCode};
use axum::response::Response;
use serde_json::json;
use tl_core::{ApiErrorCode, PolicyValidateResponse, PolicyValidationIssue};
use tl_policy::{Policy, ValidationIssue};

use super::api_error_response;
use super::response::api_error_response_with_details;

pub(super) struct ParsedPolicyBody {
    pub policy: Policy,
    pub source_yaml: String,
}

pub(super) fn parse_policy_body(
    headers: &HeaderMap,
    body: &[u8],
) -> Result<ParsedPolicyBody, Box<Response>> {
    let raw = std::str::from_utf8(body).map_err(|e| {
        Box::new(api_error_response(
            StatusCode::BAD_REQUEST,
            ApiErrorCode::Invalid,
            format!("body is not valid UTF-8: {e}"),
        ))
    })?;
    let policy = parse_policy(headers, raw).map_err(|issue| {
        Box::new(api_error_response(
            StatusCode::BAD_REQUEST,
            ApiErrorCode::Invalid,
            issue.message,
        ))
    })?;
    let source_yaml = if is_yaml_content_type(headers) {
        raw.to_string()
    } else {
        serde_yaml::to_string(&policy).map_err(|e| {
            Box::new(api_error_response(
                StatusCode::BAD_REQUEST,
                ApiErrorCode::Invalid,
                format!("policy yaml render: {e}"),
            ))
        })?
    };
    Ok(ParsedPolicyBody {
        policy,
        source_yaml,
    })
}

pub(super) fn validate_raw_policy(headers: &HeaderMap, raw: &str) -> PolicyValidateResponse {
    let parsed = parse_policy(headers, raw);
    let policy = match parsed {
        Ok(policy) => policy,
        Err(issue) => {
            return PolicyValidateResponse {
                valid: false,
                policy_id: None,
                errors: vec![issue],
            };
        }
    };

    match tl_policy::validate_policy(&policy) {
        Ok(()) => PolicyValidateResponse {
            valid: true,
            policy_id: Some(policy.id),
            errors: vec![],
        },
        Err(issues) => PolicyValidateResponse {
            valid: false,
            policy_id: Some(policy.id),
            errors: issues.iter().map(policy_validation_issue).collect(),
        },
    }
}

pub(super) fn policy_validation_error_response(issues: &[ValidationIssue]) -> Response {
    let details: Vec<_> = issues.iter().map(policy_validation_issue).collect();
    api_error_response_with_details(
        StatusCode::UNPROCESSABLE_ENTITY,
        ApiErrorCode::Unprocessable,
        "policy failed validation".into(),
        json!(details),
    )
}

pub(super) fn policy_validation_issue(issue: &ValidationIssue) -> PolicyValidationIssue {
    PolicyValidationIssue {
        path: issue.path.clone(),
        message: issue.message.clone(),
    }
}

fn parse_policy(headers: &HeaderMap, raw: &str) -> Result<Policy, PolicyValidationIssue> {
    if is_yaml_content_type(headers) {
        serde_yaml::from_str(raw).map_err(|e| PolicyValidationIssue {
            path: "$".into(),
            message: format!("yaml parse: {e}"),
        })
    } else {
        serde_json::from_str(raw).map_err(|e| PolicyValidationIssue {
            path: "$".into(),
            message: format!("json parse: {e}"),
        })
    }
}

fn is_yaml_content_type(headers: &HeaderMap) -> bool {
    let content_type = headers
        .get(header::CONTENT_TYPE)
        .and_then(|v| v.to_str().ok())
        .unwrap_or("application/json")
        .to_ascii_lowercase();

    content_type.starts_with("application/yaml")
        || content_type.starts_with("application/x-yaml")
        || content_type.starts_with("text/yaml")
        || content_type.starts_with("text/x-yaml")
}
