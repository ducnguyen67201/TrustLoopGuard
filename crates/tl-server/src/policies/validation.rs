use axum::http::{header, HeaderMap, StatusCode};
use axum::response::Response;
use serde::Deserialize;
use serde_json::json;
use tl_core::{ApiErrorCode, PolicyValidateResponse, PolicyValidationIssue};
use tl_policy::{FamilyPolicy, Policy, ValidationIssue, KNOWN_FAMILIES};

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
    match document_family(headers, raw).as_deref() {
        None | Some("content") => {}
        Some(family) if KNOWN_FAMILIES.contains(&family) => {
            // Family policies parse and validate (POST /v1/policies/validate)
            // but have no storage or runtime evaluation path yet; reject
            // creation clearly instead of with a missing-`match` parse error.
            return Err(Box::new(api_error_response(
                StatusCode::BAD_REQUEST,
                ApiErrorCode::Invalid,
                format!(
                    "`{family}` policies cannot be stored yet; POST /v1/policies accepts \
                     content policies only"
                ),
            )));
        }
        Some(other) => {
            return Err(Box::new(api_error_response(
                StatusCode::BAD_REQUEST,
                ApiErrorCode::Invalid,
                unknown_family_message(other),
            )));
        }
    }
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
    match document_family(headers, raw).as_deref() {
        None | Some("content") => validate_raw_content_policy(headers, raw),
        Some(family) if KNOWN_FAMILIES.contains(&family) => {
            validate_raw_family_policy(headers, raw)
        }
        Some(other) => PolicyValidateResponse {
            valid: false,
            policy_id: None,
            errors: vec![PolicyValidationIssue {
                path: "family".into(),
                message: unknown_family_message(other),
            }],
        },
    }
}

fn validate_raw_content_policy(headers: &HeaderMap, raw: &str) -> PolicyValidateResponse {
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

fn validate_raw_family_policy(headers: &HeaderMap, raw: &str) -> PolicyValidateResponse {
    let policy: FamilyPolicy = match parse_document(headers, raw) {
        Ok(policy) => policy,
        Err(issue) => {
            return PolicyValidateResponse {
                valid: false,
                policy_id: None,
                errors: vec![issue],
            };
        }
    };

    match tl_policy::validate_family_policy(&policy) {
        Ok(()) => PolicyValidateResponse {
            valid: true,
            policy_id: Some(policy.id().to_string()),
            errors: vec![],
        },
        Err(issues) => PolicyValidateResponse {
            valid: false,
            policy_id: Some(policy.id().to_string()),
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
    parse_document(headers, raw)
}

fn parse_document<T: serde::de::DeserializeOwned>(
    headers: &HeaderMap,
    raw: &str,
) -> Result<T, PolicyValidationIssue> {
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

#[derive(Deserialize)]
struct FamilyTag {
    #[serde(default)]
    family: Option<String>,
}

/// The document's `family:` tag, when present and probeable. Documents
/// that fail the probe parse return `None` so the content path reports
/// the underlying parse error.
fn document_family(headers: &HeaderMap, raw: &str) -> Option<String> {
    let tag: FamilyTag = parse_document(headers, raw).ok()?;
    tag.family
}

/// Truncate before echoing: the family value is caller-supplied and
/// reaches API responses. Mirrors `tl_policy::load_any_str`.
fn unknown_family_message(family: &str) -> String {
    let display: String = family.chars().take(64).collect();
    format!(
        "unknown policy family `{display}` (expected one of: {})",
        KNOWN_FAMILIES.join(", ")
    )
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
