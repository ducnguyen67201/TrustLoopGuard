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
async fn check_redacts_before_engine_evaluation() {
    let state = memory_app_state(Arc::new(Engine::empty()));
    let app = router(state, None);

    let body = serde_json::json!({
        "agent_id": "acme-support-v3",
        "channel": "chat",
        "input": "My SIN is 123-456-789 and my email is alice@example.com.",
        "proposed_output": "I will email alice@example.com with the update.",
        "context": {
            "document_type": "T4",
            "notes": "Alice Example earns $82,000."
        },
        "redaction": {
            "mode": "server",
            "status": "not_requested",
            "entities": [],
            "input_redacted": false,
            "proposed_output_redacted": false,
            "context_redacted": false
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

    let response_body = read_body(resp).await;
    let response_text = response_body.to_string();
    assert!(!response_text.contains("alice@example.com"));
    assert!(!response_text.contains("123-456-789"));
    assert!(!response_text.contains("$82,000"));
    assert_eq!(response_body["verdict"], "allow");
    assert_eq!(response_body["redaction"]["status"], "applied");
    // Positive assertions: every sensitive value type produced a stable
    // token. Without these, a regression that silently stops redacting
    // SIN or INCOME_AMOUNT would still pass the negative checks above.
    assert!(response_text.contains("[EMAIL_1]"));
    assert!(response_text.contains("[SIN_1]"));
    assert!(response_text.contains("[INCOME_AMOUNT_1]"));
}

#[derive(Debug)]
struct RedactedOnlySettingsStore;

#[async_trait]
impl SettingsStore for RedactedOnlySettingsStore {
    async fn get(
        &self,
        _workspace_id: &str,
    ) -> Result<WorkspaceSettings, dashboard_admin::DashboardAdminStoreError> {
        let mut settings = dashboard_admin::default_settings();
        settings.data_handling_mode = DataHandlingMode::RedactedOnly;
        Ok(settings)
    }
}

#[tokio::test]
async fn redacted_only_workspace_rejects_obvious_raw_sensitive_content() {
    let mut state = memory_app_state(Arc::new(Engine::empty()));
    state.settings_store = Arc::new(RedactedOnlySettingsStore);
    let app = router(state, None);

    let body = serde_json::json!({
        "agent_id": "tax-document-agent",
        "channel": "chat",
        "input": "My SIN is 123-456-789.",
        "proposed_output": "Email alice@example.com."
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

    assert_eq!(resp.status(), StatusCode::BAD_REQUEST);
    let response_body = read_body(resp).await;
    assert_eq!(response_body["code"], "invalid");
    assert_eq!(
        response_body["message"],
        "workspace requires redacted check content"
    );
}

#[tokio::test]
async fn redacted_only_workspace_rejects_client_asserted_applied_with_raw_values() {
    // A misconfigured or hostile client could flip `status: applied` while
    // still shipping raw sensitive values. The server must verify by
    // scanning content; client-asserted status is not load-bearing.
    let mut state = memory_app_state(Arc::new(Engine::empty()));
    state.settings_store = Arc::new(RedactedOnlySettingsStore);
    let app = router(state, None);

    let body = serde_json::json!({
        "agent_id": "tax-document-agent",
        "channel": "chat",
        "input": "My SIN is 123-456-789.",
        "proposed_output": "Email alice@example.com.",
        "redaction": {
            "mode": "sdk_local",
            "status": "applied",
            "entities": [],
            "input_redacted": false,
            "proposed_output_redacted": false,
            "context_redacted": false
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

    assert_eq!(resp.status(), StatusCode::BAD_REQUEST);
    let response_body = read_body(resp).await;
    assert_eq!(response_body["code"], "invalid");
}

#[tokio::test]
async fn server_redaction_produces_stable_tokens_across_fields() {
    // Same raw value in input, proposed_output, and context must get the
    // same token. Without this, policies and humans can't correlate
    // sanitized references back to one entity.
    let state = memory_app_state(Arc::new(Engine::empty()));
    let app = router(state, None);

    let body = serde_json::json!({
        "agent_id": "acme-support-v3",
        "channel": "chat",
        "input": "Contact alice@example.com today.",
        "proposed_output": "Will email alice@example.com tomorrow.",
        "context": {
            "notes": "Customer alice@example.com requested a callback."
        },
        "redaction": {
            "mode": "server",
            "status": "not_requested",
            "entities": [],
            "input_redacted": false,
            "proposed_output_redacted": false,
            "context_redacted": false
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

    let response_body = read_body(resp).await;
    let response_text = response_body.to_string();
    assert!(!response_text.contains("alice@example.com"));
    // Token is allocated once for the value and reused across all three
    // surfaces; the entity count records three occurrences.
    let entities = response_body["redaction"]["entities"]
        .as_array()
        .expect("entities array");
    let email = entities
        .iter()
        .find(|entity| entity["entity_type"] == "EMAIL")
        .expect("EMAIL entity present");
    assert_eq!(email["token"], "[EMAIL_1]");
    assert_eq!(email["count"], 3);
}

#[tokio::test]
async fn run_lifecycle_endpoints_group_execution_state() {
    let state = memory_app_state(Arc::new(Engine::empty()));
    let app = router(state, None);

    let create_body = serde_json::json!({
        "agent_id": "acme-support-v3",
        "kind": "chat_session",
        "external_id": "chat-123",
        "metadata": { "tier": "enterprise" }
    });
    let created = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/v1/runs")
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from(create_body.to_string()))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(created.status(), StatusCode::CREATED);
    let created_body = read_body(created).await;
    let run_id = created_body["id"].as_str().unwrap();
    assert_eq!(created_body["status"], "running");
    assert_eq!(created_body["external_id"], "chat-123");

    let encoded_create_body = serde_json::json!({
        "agent_id": "acme-support-v3",
        "kind": "chat_session",
        "external_id": "chat/encoded"
    });
    let encoded_created = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/v1/runs")
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from(encoded_create_body.to_string()))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(encoded_created.status(), StatusCode::CREATED);

    let listed = app
        .clone()
        .oneshot(
            Request::builder()
                .method("GET")
                .uri("/v1/runs?external_id=chat-123")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(listed.status(), StatusCode::OK);
    let listed_body = read_body(listed).await;
    assert_eq!(listed_body["runs"].as_array().unwrap().len(), 1);

    let encoded_listed = app
        .clone()
        .oneshot(
            Request::builder()
                .method("GET")
                .uri("/v1/runs?external_id=chat%2Fencoded")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(encoded_listed.status(), StatusCode::OK);
    let encoded_listed_body = read_body(encoded_listed).await;
    assert_eq!(encoded_listed_body["runs"].as_array().unwrap().len(), 1);

    let event = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri(format!("/v1/runs/{run_id}/events"))
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from(
                    serde_json::json!({
                        "kind": "user_turn",
                        "label": "Turn 1",
                        "input_summary": "Customer asks for a refund"
                    })
                    .to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(event.status(), StatusCode::CREATED);
    let event_body = read_body(event).await;
    assert_eq!(event_body["sequence"], 1);
    assert_eq!(event_body["kind"], "user_turn");

    let updated = app
        .clone()
        .oneshot(
            Request::builder()
                .method("PATCH")
                .uri(format!("/v1/runs/{run_id}"))
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from(r#"{"status":"completed"}"#))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(updated.status(), StatusCode::OK);
    let updated_body = read_body(updated).await;
    assert_eq!(updated_body["status"], "completed");
    assert!(updated_body["ended_at"].as_str().is_some());

    let detail = app
        .oneshot(
            Request::builder()
                .method("GET")
                .uri(format!("/v1/runs/{run_id}"))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(detail.status(), StatusCode::OK);
    let detail_body = read_body(detail).await;
    assert_eq!(detail_body["run"]["id"], run_id);
    assert_eq!(detail_body["events"].as_array().unwrap().len(), 1);
    assert!(detail_body["traces"].as_array().unwrap().is_empty());
}

#[tokio::test]
async fn check_can_create_run_event_inline() {
    let state = memory_app_state(Arc::new(Engine::empty()));
    let app = router(state, None);

    let created = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/v1/runs")
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from(
                    serde_json::json!({
                        "agent_id": "acme-support-v3",
                        "kind": "chat_session"
                    })
                    .to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(created.status(), StatusCode::CREATED);
    let run_id = read_body(created).await["id"].as_str().unwrap().to_string();

    let checked = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/v1/check")
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from(
                    serde_json::json!({
                        "run_id": run_id,
                        "run_event": {
                            "kind": "assistant_turn",
                            "label": "Turn 1",
                            "input_summary": "Customer asks about a refund",
                            "output_summary": "Agent drafts refund answer"
                        },
                        "agent_id": "acme-support-v3",
                        "channel": "chat",
                        "input": "Can I get a refund?",
                        "proposed_output": "I can help check refund options."
                    })
                    .to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(checked.status(), StatusCode::OK);

    let detail = app
        .oneshot(
            Request::builder()
                .method("GET")
                .uri(format!("/v1/runs/{run_id}"))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(detail.status(), StatusCode::OK);
    let detail_body = read_body(detail).await;
    let events = detail_body["events"].as_array().unwrap();
    assert_eq!(events.len(), 1);
    assert_eq!(events[0]["kind"], "assistant_turn");
    assert_eq!(events[0]["label"], "Turn 1");
}

#[tokio::test]
async fn check_rejects_malformed_run_id_before_engine_execution() {
    let state = memory_app_state(Arc::new(Engine::empty()));
    let app = router(state, None);

    let body = serde_json::json!({
        "run_id": "not-a-uuid",
        "agent_id": "anon",
        "channel": "chat",
        "input": "hi",
        "proposed_output": "hello"
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
    assert_eq!(resp.status(), StatusCode::BAD_REQUEST);
    let body = read_body(resp).await;
    assert_eq!(body["code"], "invalid");
}

#[tokio::test]
async fn check_rejects_run_event_id_without_run_id() {
    let state = memory_app_state(Arc::new(Engine::empty()));
    let app = router(state, None);

    let body = serde_json::json!({
        "run_event_id": "018f2222-2222-7222-8222-222222222222",
        "agent_id": "anon",
        "channel": "chat",
        "input": "hi",
        "proposed_output": "hello"
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

    assert_eq!(resp.status(), StatusCode::BAD_REQUEST);
    let body = read_body(resp).await;
    assert_eq!(
        body["message"],
        "run_id is required when run_event_id is provided"
    );
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

#[tokio::test]
async fn check_uses_enabled_policy_created_through_authoring_api() {
    let state = memory_app_state(Arc::new(Engine::empty()));
    let app = router(state, None);

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
    let app = router(state, None);

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
