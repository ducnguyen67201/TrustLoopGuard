//! End-to-end test for the wired stack: register an agent, then
//! POST /v1/check and verify the response shape matches the engine's
//! `check_async` output (tier_results populated, cache wired).
//!
//! Memory-only — no Postgres required. The Postgres-backed path is
//! exercised by `tl-storage`'s postgres-it tests; PR 16 adds a
//! Postgres-backed E2E test here under the same gate.

use std::sync::Arc;

use async_trait::async_trait;
use axum::{
    body::Body,
    http::{header, Request, StatusCode},
};
use http_body_util::BodyExt;
use tl_core::{DataHandlingMode, Decision, Verdict, WorkspaceSettings};
use tl_engine::Engine;
use tl_server::{dashboard_admin, memory_app_state, router, SettingsStore};
use tower::ServiceExt;

const ACME_YAML: &str = r#"
agent_id: acme-support-v3
display_name: Acme Support Assistant
scope:
  in_scope:
    - billing questions
authority:
  can_promise:
    - we'll respond within 24h
  cannot_promise:
    - refunds
tone:
  target: warm-professional
"#;

const REFUND_POLICY_YAML: &str = r#"
id: refund-guarantee
description: Prevent guaranteed refund promises.
match:
  literal: guaranteed refund
action: block
severity: high
"#;

async fn read_body(resp: axum::response::Response) -> serde_json::Value {
    let bytes = resp.into_body().collect().await.unwrap().to_bytes();
    serde_json::from_slice(&bytes).unwrap()
}

#[tokio::test]
async fn register_agent_then_check_returns_full_decision() {
    let state = memory_app_state(Arc::new(Engine::empty()));
    let app = router(state, None, [0u8; 32]);

    // 1. Register profile.
    let resp = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/v1/agents")
                .header(header::CONTENT_TYPE, "application/yaml")
                .body(Body::from(ACME_YAML))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::CREATED);

    // 2. POST /v1/check with that agent_id.
    let body = serde_json::json!({
        "agent_id": "acme-support-v3",
        "channel": "chat",
        "input": "what time do you open?",
        "proposed_output": "We're open 9am to 5pm weekdays."
    });
    let resp = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/v1/check")
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from(body.to_string()))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);

    // 3. Decision shape: full async pipeline → 3 tier_results, verdict
    //    Allow because no policies and Tier 3 has no LlmRouter routes.
    let decision: Decision = serde_json::from_value(read_body(resp).await).unwrap();
    assert_eq!(decision.verdict, Verdict::Allow);
    assert_eq!(decision.tier_results.len(), 3, "all three tiers reported");
    assert_eq!(decision.tier_results[0].tier, tl_core::Tier::Deterministic);
    assert_eq!(decision.tier_results[1].tier, tl_core::Tier::Fuzzy);
    assert_eq!(decision.tier_results[2].tier, tl_core::Tier::Llm);
}

#[tokio::test]
async fn gateway_full_body_checks_include_checked_text_excerpts() {
    let state = memory_app_state(Arc::new(Engine::empty()));
    let app = router(state, None, [0u8; 32]);

    let body = serde_json::json!({
        "agent_id": "acme-support-v3",
        "channel": "chat",
        "input": "caller asked a normal scheduling question",
        "proposed_output": "That is a stupid question. Figure it out yourself.",
        "domain": "gateway_output_check",
        "context": {
            "integration_mode": "gateway",
            "gateway_phase": "gateway_output_check",
            "retention_mode": "full_body"
        }
    });
    let resp = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/v1/check")
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from(body.to_string()))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);

    let decision: Decision = serde_json::from_value(read_body(resp).await).unwrap();
    assert_eq!(
        decision.checked_input_excerpt.as_deref(),
        Some("caller asked a normal scheduling question")
    );
    assert_eq!(
        decision.checked_output_excerpt.as_deref(),
        Some("That is a stupid question. Figure it out yourself.")
    );
}

#[tokio::test]
async fn gateway_metadata_only_checks_omit_checked_text_excerpts() {
    let state = memory_app_state(Arc::new(Engine::empty()));
    let app = router(state, None, [0u8; 32]);

    let body = serde_json::json!({
        "agent_id": "acme-support-v3",
        "channel": "chat",
        "input": "private caller text",
        "proposed_output": "private assistant text",
        "domain": "gateway_output_check",
        "context": {
            "integration_mode": "gateway",
            "gateway_phase": "gateway_output_check",
            "retention_mode": "metadata_only",
            "body_retention": "omitted"
        }
    });
    let resp = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/v1/check")
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from(body.to_string()))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);

    let decision: Decision = serde_json::from_value(read_body(resp).await).unwrap();
    assert_eq!(decision.checked_input_excerpt, None);
    assert_eq!(decision.checked_output_excerpt, None);
}

#[tokio::test]
async fn disabled_policy_no_longer_changes_check_decision() {
    let state = memory_app_state(Arc::new(Engine::empty()));
    let app = router(state, None, [0u8; 32]);

    let resp = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/v1/policies")
                .header(header::CONTENT_TYPE, "application/yaml")
                .body(Body::from(REFUND_POLICY_YAML))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::CREATED);

    let check_body = serde_json::json!({
        "agent_id": "acme-support-v3",
        "channel": "chat",
        "input": "Can I get my money back?",
        "proposed_output": "Yes, I can promise a guaranteed refund."
    });

    let resp = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/v1/check")
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from(check_body.to_string()))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let decision: Decision = serde_json::from_value(read_body(resp).await).unwrap();
    assert_eq!(decision.verdict, Verdict::Block);
    assert_eq!(decision.triggered_policies[0].id, "refund-guarantee");

    let resp = app
        .clone()
        .oneshot(
            Request::builder()
                .method("PATCH")
                .uri("/v1/policies/refund-guarantee/enabled")
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from(r#"{"enabled":false}"#))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);

    let resp = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/v1/check")
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from(check_body.to_string()))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let decision: Decision = serde_json::from_value(read_body(resp).await).unwrap();
    assert_eq!(decision.verdict, Verdict::Allow);
    assert!(decision.triggered_policies.is_empty());
}

#[tokio::test]
async fn same_agent_can_have_different_policy_deployments_per_environment() {
    let state = memory_app_state(Arc::new(Engine::empty()));
    let app = router(state, None, [0u8; 32]);

    let resp = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/v1/environments")
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from(
                    serde_json::json!({
                        "slug": "dev",
                        "name": "Development"
                    })
                    .to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::CREATED);
    let environment = read_body(resp).await;
    let dev_environment_id = environment["id"].as_str().unwrap();

    let resp = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/v1/policies")
                .header(header::CONTENT_TYPE, "application/yaml")
                .body(Body::from(REFUND_POLICY_YAML))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::CREATED);

    let check_body = serde_json::json!({
        "agent_id": "acme-support-v3",
        "channel": "chat",
        "input": "Can I get my money back?",
        "proposed_output": "Yes, I can promise a guaranteed refund."
    });

    let resp = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/v1/check")
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from(check_body.to_string()))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let decision: Decision = serde_json::from_value(read_body(resp).await).unwrap();
    assert_eq!(decision.verdict, Verdict::Block);

    let resp = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/v1/check")
                .header(header::CONTENT_TYPE, "application/json")
                .header("x-tlg-environment-id", dev_environment_id)
                .body(Body::from(check_body.to_string()))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let decision: Decision = serde_json::from_value(read_body(resp).await).unwrap();
    assert_eq!(decision.verdict, Verdict::Allow);
    assert!(decision.triggered_policies.is_empty());
}

include!("full_pipeline/redaction.rs");

include!("full_pipeline/runs.rs");

#[tokio::test]
async fn check_allows_sensitive_text_when_no_policy_is_deployed() {
    // Runtime decisions come from stored policies. With no deployed
    // policies, sensitive-looking text does not trigger a hardcoded block.
    let state = memory_app_state(Arc::new(Engine::empty()));
    let app = router(state, None, [0u8; 32]);

    let body = serde_json::json!({
        "agent_id": "anon",
        "channel": "chat",
        "input": "send my number",
        "proposed_output": "Sure — call me at 415-555-1212"
    });
    let resp = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/v1/check")
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from(body.to_string()))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let decision: Decision = serde_json::from_value(read_body(resp).await).unwrap();
    assert_eq!(decision.verdict, Verdict::Allow);
    assert!(decision.triggered_policies.is_empty());
}

#[tokio::test]
async fn second_identical_check_hits_cache() {
    let state = memory_app_state(Arc::new(Engine::empty()));
    let app = router(state, None, [0u8; 32]);

    let body = serde_json::json!({
        "agent_id": "anon",
        "channel": "chat",
        "input": "hi",
        "proposed_output": "hello there"
    });

    let r1 = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/v1/check")
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from(body.to_string()))
                .unwrap(),
        )
        .await
        .unwrap();
    let d1: Decision = serde_json::from_value(read_body(r1).await).unwrap();

    let r2 = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/v1/check")
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from(body.to_string()))
                .unwrap(),
        )
        .await
        .unwrap();
    let d2: Decision = serde_json::from_value(read_body(r2).await).unwrap();

    // Both succeed; second has a different trace_id (server refreshes
    // it on cache hits) but the verdict + reasons survive.
    assert_eq!(d1.verdict, d2.verdict);
    assert_ne!(d1.trace_id, d2.trace_id);
}

#[tokio::test]
async fn check_uses_enabled_policy_created_through_authoring_api() {
    let state = memory_app_state(Arc::new(Engine::empty()));
    let app = router(state, None, [0u8; 32]);

    let publish = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/v1/policies")
                .header(header::CONTENT_TYPE, "application/yaml")
                .body(Body::from(REFUND_POLICY_YAML))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(publish.status(), StatusCode::CREATED);

    let check_body = serde_json::json!({
        "agent_id": "acme-support-v3",
        "channel": "chat",
        "input": "can I get a refund?",
        "proposed_output": "Yes, you get a guaranteed refund."
    });
    let blocked = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/v1/check")
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from(check_body.to_string()))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(blocked.status(), StatusCode::OK);
    let decision: Decision = serde_json::from_value(read_body(blocked).await).unwrap();
    assert_eq!(decision.verdict, Verdict::Block);
    assert!(decision
        .triggered_policies
        .iter()
        .any(|policy| policy.id == "refund-guarantee"));

    let disabled = app
        .clone()
        .oneshot(
            Request::builder()
                .method("PATCH")
                .uri("/v1/policies/refund-guarantee/enabled")
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from(r#"{"enabled":false}"#))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(disabled.status(), StatusCode::OK);

    let allowed = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/v1/check")
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from(check_body.to_string()))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(allowed.status(), StatusCode::OK);
    let decision: Decision = serde_json::from_value(read_body(allowed).await).unwrap();
    assert_eq!(decision.verdict, Verdict::Allow);
    assert!(!decision
        .triggered_policies
        .iter()
        .any(|policy| policy.id == "refund-guarantee"));
}

#[tokio::test]
async fn check_uses_only_runtime_policies_from_request_workspace() {
    let state = memory_app_state(Arc::new(Engine::empty()));
    let app = router(state, None, [0u8; 32]);

    let publish = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/v1/policies")
                .header(header::CONTENT_TYPE, "application/yaml")
                .header("x-tlg-workspace-id", "ws_alpha")
                .body(Body::from(REFUND_POLICY_YAML))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(publish.status(), StatusCode::CREATED);

    let check_body = serde_json::json!({
        "agent_id": "acme-support-v3",
        "channel": "chat",
        "input": "can I get a refund?",
        "proposed_output": "Yes, you get a guaranteed refund."
    });

    let blocked = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/v1/check")
                .header(header::CONTENT_TYPE, "application/json")
                .header("x-tlg-workspace-id", "ws_alpha")
                .body(Body::from(check_body.to_string()))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(blocked.status(), StatusCode::OK);
    let decision: Decision = serde_json::from_value(read_body(blocked).await).unwrap();
    assert_eq!(decision.verdict, Verdict::Block);

    let allowed = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/v1/check")
                .header(header::CONTENT_TYPE, "application/json")
                .header("x-tlg-workspace-id", "ws_beta")
                .body(Body::from(check_body.to_string()))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(allowed.status(), StatusCode::OK);
    let decision: Decision = serde_json::from_value(read_body(allowed).await).unwrap();
    assert_eq!(decision.verdict, Verdict::Allow);
    assert!(!decision
        .triggered_policies
        .iter()
        .any(|policy| policy.id == "refund-guarantee"));
}
