use axum::http::{header, HeaderMap};

use super::draft::{
    policy_draft_item_schema, policy_set_draft_json_schema, POLICY_SET_DRAFT_SYSTEM_PROMPT,
};
use super::validate_raw_policy;
use super::validation::parse_policy_body;

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
    let yaml = include_str!("../../../../docs/policies/examples/refund-guarantee.yaml");
    let out = validate_raw_policy(&headers, yaml);
    assert!(out.valid);
    let parsed = tl_policy::load_str(yaml).expect("policy");
    assert_eq!(out.policy_id.as_deref(), Some(parsed.id.as_str()));
}

fn yaml_headers() -> HeaderMap {
    let mut headers = HeaderMap::new();
    headers.insert(header::CONTENT_TYPE, "application/yaml".parse().unwrap());
    headers
}

#[test]
fn family_policy_yaml_validates_through_endpoint_path() {
    let yaml = include_str!("../../../../docs/policies/examples/approval-payments.yaml");
    let out = validate_raw_policy(&yaml_headers(), yaml);
    assert!(out.valid, "errors: {:?}", out.errors);
    assert_eq!(out.policy_id.as_deref(), Some("payments-need-admin"));
}

#[test]
fn family_policy_json_validates_through_endpoint_path() {
    let headers = HeaderMap::new();
    let out = validate_raw_policy(
        &headers,
        r#"{"family":"memory","id":"json-memory","deny_untrusted_authority_writes":true,"action":"escalate"}"#,
    );
    assert!(out.valid, "errors: {:?}", out.errors);
    assert_eq!(out.policy_id.as_deref(), Some("json-memory"));
}

#[test]
fn invalid_family_policy_returns_structured_issues_and_id() {
    let out = validate_raw_policy(
        &yaml_headers(),
        r#"
family: approval
id: unconditional
when: {}
action: escalate
"#,
    );
    assert!(!out.valid);
    assert_eq!(out.policy_id.as_deref(), Some("unconditional"));
    assert!(out.errors.iter().any(|e| e.path == "when"));
}

#[test]
fn unknown_family_is_invalid_with_truncated_echo() {
    let long_family = "x".repeat(500);
    let out = validate_raw_policy(
        &yaml_headers(),
        &format!("family: {long_family}\nid: nonsense\naction: block\n"),
    );
    assert!(!out.valid);
    assert_eq!(out.errors[0].path, "family");
    assert!(out.errors[0].message.contains("unknown policy family"));
    assert!(out.errors[0].message.len() < 256, "echo not truncated");
}

#[test]
fn create_path_rejects_family_policies_with_clear_message() {
    let yaml = include_str!("../../../../docs/policies/examples/approval-payments.yaml");
    let err = parse_policy_body(&yaml_headers(), yaml.as_bytes())
        .err()
        .expect("family policy must not be storable");
    assert_eq!(err.status(), axum::http::StatusCode::BAD_REQUEST);
}

#[test]
fn policy_set_draft_schema_wraps_array_with_bounds() {
    let schema = policy_set_draft_json_schema();
    assert_eq!(schema.name, "policy_set_draft");
    let policies = &schema.schema["properties"]["policies"];
    assert_eq!(policies["type"], "array");
    assert_eq!(policies["minItems"], 3);
    assert_eq!(policies["maxItems"], 8);
    assert_eq!(policies["items"], policy_draft_item_schema());
}

#[test]
fn policy_set_draft_system_prompt_mentions_required_coverage() {
    let prompt = POLICY_SET_DRAFT_SYSTEM_PROMPT;
    for needle in ["PII", "Scope", "Tone", "Hallucinated"] {
        assert!(prompt.contains(needle), "prompt should mention {needle}");
    }
}

#[test]
fn policy_error_type_still_formats_for_cli() {
    let err = tl_policy::PolicyError::Validation("id: id is required".into()).to_string();
    assert!(err.contains("policy validation"));
}
