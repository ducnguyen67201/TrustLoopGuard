use axum::http::{header, HeaderMap};

use super::draft::{
    policy_draft_item_schema, policy_set_draft_json_schema, POLICY_SET_DRAFT_SYSTEM_PROMPT,
};
use super::validate_raw_policy;

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
