//! End-to-end test for the wired stack: register an agent, then
//! POST /v1/check and verify the response shape matches the engine's
//! `check_async` output (tier_results populated, cache wired).
//!
//! Memory-only — no Postgres required. The Postgres-backed path is
//! exercised by `tl-storage`'s postgres-it tests; PR 16 adds a
//! Postgres-backed E2E test here under the same gate.

use std::sync::Arc;

use axum::{
    body::Body,
    http::{header, Request, StatusCode},
};
use http_body_util::BodyExt;
use tl_core::{Decision, Verdict};
use tl_engine::Engine;
use tl_server::{memory_app_state, router};
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

async fn read_body(resp: axum::response::Response) -> serde_json::Value {
    let bytes = resp.into_body().collect().await.unwrap().to_bytes();
    serde_json::from_slice(&bytes).unwrap()
}

#[tokio::test]
async fn register_agent_then_check_returns_full_decision() {
    let state = memory_app_state(Arc::new(Engine::empty()));
    let app = router(state, None);

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
async fn check_uses_universal_pii_detector() {
    // No tenant policies and no profile registered, but universal
    // patterns should still fire (PII in proposed_output → Block).
    let state = memory_app_state(Arc::new(Engine::empty()));
    let app = router(state, None);

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
    assert_eq!(decision.verdict, Verdict::Block);
    assert!(decision
        .triggered_policies
        .iter()
        .any(|p| p.id.contains("pii.phone")));
}

#[tokio::test]
async fn second_identical_check_hits_cache() {
    let state = memory_app_state(Arc::new(Engine::empty()));
    let app = router(state, None);

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
