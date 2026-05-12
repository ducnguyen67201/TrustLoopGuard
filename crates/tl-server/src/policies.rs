//! Policy authoring endpoints.
//!
//! Phase 3 only validates policy YAML/JSON. It does not persist policies
//! or change runtime policy resolution; those land in later cloud-policy
//! phases.

use axum::{
    http::{header, HeaderMap, StatusCode},
    response::{IntoResponse, Response},
    Json,
};
use serde_json::json;
use tl_core::{ApiError, ApiErrorCode, PolicyValidateResponse, PolicyValidationIssue};
use tl_policy::{Policy, ValidationIssue};

/// `POST /v1/policies/validate` — validate policy YAML or JSON without saving it.
#[utoipa::path(
    post,
    path = "/v1/policies/validate",
    tag = "policies",
    request_body(
        description = "Policy document, YAML or JSON",
        content_type = "application/yaml",
        content = String,
    ),
    responses(
        (status = 200, description = "Validation result", body = PolicyValidateResponse),
        (status = 400, description = "Malformed request body", body = ApiError),
        (status = 401, description = "Missing or invalid API key", body = ApiError),
    ),
)]
pub async fn validate_policy(headers: HeaderMap, body: bytes::Bytes) -> Response {
    let raw = match std::str::from_utf8(&body) {
        Ok(raw) => raw,
        Err(e) => {
            return api_error_response(
                StatusCode::BAD_REQUEST,
                ApiErrorCode::Invalid,
                format!("body is not valid UTF-8: {e}"),
            );
        }
    };

    Json(validate_raw_policy(&headers, raw)).into_response()
}

fn validate_raw_policy(headers: &HeaderMap, raw: &str) -> PolicyValidateResponse {
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

fn policy_validation_issue(issue: &ValidationIssue) -> PolicyValidationIssue {
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

fn api_error_response(status: StatusCode, code: ApiErrorCode, message: String) -> Response {
    let retriable = matches!(
        code,
        ApiErrorCode::RateLimited | ApiErrorCode::Internal | ApiErrorCode::Unavailable
    );
    let body = ApiError {
        code,
        message,
        retriable,
        details: json!(null),
    };
    (status, Json(body)).into_response()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn malformed_yaml_returns_validation_issue() {
        let mut headers = HeaderMap::new();
        headers.insert(header::CONTENT_TYPE, "application/yaml".parse().unwrap());
        let out = validate_raw_policy(&headers, "not: valid: yaml: [");
        assert!(!out.valid);
        assert_eq!(out.errors[0].path, "$");
        assert!(out.errors[0].message.contains("yaml parse"));
    }

    #[test]
    fn valid_yaml_returns_policy_id() {
        let mut headers = HeaderMap::new();
        headers.insert(header::CONTENT_TYPE, "application/yaml".parse().unwrap());
        let out = validate_raw_policy(
            &headers,
            r#"
id: refund-guarantee
description: Prevents agents from guaranteeing refunds.
match:
  literal: "guaranteed refund"
action: block
"#,
        );
        assert!(out.valid);
        assert_eq!(out.policy_id.as_deref(), Some("refund-guarantee"));
        assert!(out.errors.is_empty());
    }

    #[test]
    fn validation_errors_are_structured() {
        let mut headers = HeaderMap::new();
        headers.insert(header::CONTENT_TYPE, "application/yaml".parse().unwrap());
        let out = validate_raw_policy(
            &headers,
            r#"
id: "Refund Guarantee"
match:
  regex: "["
action: rewrite
"#,
        );
        assert!(!out.valid);
        assert!(out.errors.iter().any(|e| e.path == "id"));
        assert!(out.errors.iter().any(|e| e.path == "match.regex"));
        assert!(out.errors.iter().any(|e| e.path == "rewrite"));
    }

    #[test]
    fn valid_json_policy_works() {
        let headers = HeaderMap::new();
        let out = validate_raw_policy(
            &headers,
            r#"{"id":"json-policy","match":{"literal":"refund"},"action":"block"}"#,
        );
        assert!(out.valid);
        assert_eq!(out.policy_id.as_deref(), Some("json-policy"));
    }

    #[test]
    fn load_str_and_validate_endpoint_agree_on_valid_yaml() {
        let mut headers = HeaderMap::new();
        headers.insert(header::CONTENT_TYPE, "application/yaml".parse().unwrap());
        let yaml = include_str!("../../../docs/policies/examples/refund-guarantee.yaml");
        let out = validate_raw_policy(&headers, yaml);
        assert!(out.valid);
        let parsed = tl_policy::load_str(yaml).expect("policy");
        assert_eq!(out.policy_id.as_deref(), Some(parsed.id.as_str()));
    }

    #[test]
    fn policy_error_type_still_formats_for_cli() {
        let err = tl_policy::PolicyError::Validation("id: id is required".into()).to_string();
        assert!(err.contains("policy validation"));
    }
}
