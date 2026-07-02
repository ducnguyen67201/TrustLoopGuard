//! Payment-family enforcement through `/v1/events` (no Postgres).
//!
//! Seeds a `payment` policy in the in-memory store, submits `pay` events, and
//! asserts the composed decision. Covers per-call caps + hold + conservative
//! posture + owner scoping. (Windowed daily/monthly caps need trace history,
//! which the memory trace store doesn't accumulate, so they're covered by the
//! `tl-engine` unit tests instead.)

use std::sync::Arc;

use axum::{
    body::Body,
    http::{header, Request},
};
use http_body_util::BodyExt;
use tl_core::{DEFAULT_ENVIRONMENT_ID, DEFAULT_WORKSPACE_ID};
use tl_engine::Engine;
use tl_policy::{Action, FamilyPolicy, PaymentPolicy, PaymentWhen};
use tl_server::{memory_app_state, router, AppState};
use tower::ServiceExt;

fn payment_policy() -> FamilyPolicy {
    FamilyPolicy::Payment(PaymentPolicy {
        id: "pay-alice".into(),
        description: None,
        severity: tl_core::Severity::High,
        when: PaymentWhen {
            agents: vec!["alice".into()],
            operations: vec!["pay".into()],
        },
        per_transaction_minor: Some(10_000),
        hold_above_minor: Some(5_000),
        daily_minor: None,
        monthly_minor: None,
        on_breach: Action::Block,
    })
}

async fn seeded_state() -> AppState {
    let state = memory_app_state(Arc::new(Engine::empty()));
    let policy = payment_policy();
    let yaml = serde_yaml::to_string(&policy).unwrap();
    state
        .policy_store
        .upsert_family(DEFAULT_WORKSPACE_ID, DEFAULT_ENVIRONMENT_ID, &policy, &yaml)
        .await
        .unwrap();
    state
}

fn pay_event(owner: &str, amount: Option<i64>) -> serde_json::Value {
    let mut params = serde_json::Map::new();
    if let Some(amount) = amount {
        params.insert("amount".into(), amount.into());
    }
    params.insert("merchant".into(), "Coffee".into());
    serde_json::json!({
        "kind": "tool.call.proposed",
        "principal": {
            "workspace_id": DEFAULT_WORKSPACE_ID,
            "environment_id": DEFAULT_ENVIRONMENT_ID,
            "agent_id": owner
        },
        "action": { "operation": "pay", "parameters": params }
    })
}

async fn verdict(state: AppState, owner: &str, amount: Option<i64>) -> String {
    let app = router(state, None, [0u8; 32]);
    let request = Request::builder()
        .method("POST")
        .uri("/v1/events")
        .header(header::CONTENT_TYPE, "application/json")
        .header("x-tlg-workspace-id", DEFAULT_WORKSPACE_ID)
        .body(Body::from(pay_event(owner, amount).to_string()))
        .unwrap();
    let resp = app.oneshot(request).await.unwrap();
    let bytes = resp.into_body().collect().await.unwrap().to_bytes();
    let decision: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
    decision["verdict"].as_str().unwrap().to_string()
}

#[tokio::test]
async fn within_caps_allows() {
    let state = seeded_state().await;
    assert_eq!(verdict(state, "alice", Some(4_000)).await, "allow");
}

#[tokio::test]
async fn over_per_transaction_blocks() {
    let state = seeded_state().await;
    assert_eq!(verdict(state, "alice", Some(80_000)).await, "block");
}

#[tokio::test]
async fn hold_band_escalates() {
    let state = seeded_state().await;
    assert_eq!(verdict(state, "alice", Some(6_000)).await, "escalate");
}

#[tokio::test]
async fn missing_amount_escalates_conservatively() {
    let state = seeded_state().await;
    assert_eq!(verdict(state, "alice", None).await, "escalate");
}

#[tokio::test]
async fn other_owner_not_in_scope_allows() {
    let state = seeded_state().await;
    assert_eq!(verdict(state, "bob", Some(80_000)).await, "allow");
}

// ---------------------------------------------------------------------------
// Families REST listing (Phase 1) and inline execution via the pay gate
// (Phase 2). The gate is driven directly — the MCP layer is a thin shim over
// it — and the provider is a wiremock server, so these tests prove:
// judge-then-execute, never execute on block, execute a hold exactly once,
// honest reporting on provider failure, and approved holds counting toward
// windowed caps.
// ---------------------------------------------------------------------------

use tl_server::{PayGate, PayRequest, SpendCaps};
use wiremock::matchers::{header, method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

const SEAL_KEY: [u8; 32] = [0u8; 32];

fn gate(state: &AppState) -> PayGate {
    PayGate::new(state.clone(), SEAL_KEY, reqwest::Client::new())
}

fn pay_request(owner: &str, amount_minor: i64) -> PayRequest {
    PayRequest {
        owner: owner.into(),
        amount_minor,
        merchant: "Coffee".into(),
        category: None,
        memo: None,
    }
}

/// Create a `payment_http` provider connection through the REST surface so
/// the credential is sealed with the same key the gate unseals with.
async fn create_payment_connection(state: &AppState, base_url: &str) {
    let app = router(state.clone(), None, SEAL_KEY);
    let body = serde_json::json!({
        "display_name": "Test payments",
        "kind": "payment_http",
        "base_url": base_url,
        "provider_api_key": "test-provider-key",
    });
    let request = Request::builder()
        .method("POST")
        .uri("/v1/gateway/provider-connections")
        .header(header::CONTENT_TYPE, "application/json")
        .header("x-tlg-workspace-id", DEFAULT_WORKSPACE_ID)
        .body(Body::from(body.to_string()))
        .unwrap();
    let resp = app.oneshot(request).await.unwrap();
    assert_eq!(resp.status(), 201, "payment connection create failed");
}

async fn get_json(state: &AppState, uri: &str) -> serde_json::Value {
    let app = router(state.clone(), None, SEAL_KEY);
    let request = Request::builder()
        .method("GET")
        .uri(uri)
        .header("x-tlg-workspace-id", DEFAULT_WORKSPACE_ID)
        .body(Body::empty())
        .unwrap();
    let resp = app.oneshot(request).await.unwrap();
    assert_eq!(resp.status(), 200);
    let bytes = resp.into_body().collect().await.unwrap().to_bytes();
    serde_json::from_slice(&bytes).unwrap()
}

#[tokio::test]
async fn families_listing_returns_seeded_policy() {
    let state = seeded_state().await;
    let body = get_json(&state, "/v1/policies/families").await;
    let listed = body["policies"].as_array().expect("policies array");
    assert_eq!(listed.len(), 1);
    assert_eq!(listed[0]["id"], "pay-alice");
    assert_eq!(listed[0]["family"], "payment");
    assert_eq!(listed[0]["per_transaction_minor"], 10_000);
}

#[tokio::test]
async fn families_listing_empty_workspace() {
    let state = memory_app_state(Arc::new(Engine::empty()));
    let body = get_json(&state, "/v1/policies/families").await;
    assert_eq!(body["policies"], serde_json::json!([]));
}

/// Pins the `set_policy` construction ↔ enforcement contract without MCP
/// transport plumbing: caps set through the gate are enforced by `pay`.
#[tokio::test]
async fn set_policy_then_pay_over_cap_blocks() {
    let state = memory_app_state(Arc::new(Engine::empty()));
    let gate = gate(&state);
    gate.set_policy(
        DEFAULT_WORKSPACE_ID,
        DEFAULT_ENVIRONMENT_ID,
        SpendCaps {
            owner: "alice".into(),
            per_transaction_minor: Some(10_000),
            daily_minor: None,
            monthly_minor: None,
            hold_above_minor: None,
        },
    )
    .await
    .unwrap();
    let outcome = gate
        .pay(
            DEFAULT_WORKSPACE_ID,
            DEFAULT_ENVIRONMENT_ID,
            pay_request("alice", 80_000),
        )
        .await
        .unwrap();
    assert_eq!(outcome["status"], "block");
}

#[tokio::test]
async fn within_caps_without_provider_reports_allow_no_provider() {
    let state = seeded_state().await;
    let outcome = gate(&state)
        .pay(
            DEFAULT_WORKSPACE_ID,
            DEFAULT_ENVIRONMENT_ID,
            pay_request("alice", 4_000),
        )
        .await
        .unwrap();
    assert_eq!(outcome["status"], "allow_no_provider");
}

#[tokio::test]
async fn allow_executes_via_provider() {
    let state = seeded_state().await;
    let provider = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/payments"))
        .and(header("authorization", "Bearer test-provider-key"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({"ok": true})))
        .expect(1)
        .mount(&provider)
        .await;
    create_payment_connection(&state, &provider.uri()).await;

    let outcome = gate(&state)
        .pay(
            DEFAULT_WORKSPACE_ID,
            DEFAULT_ENVIRONMENT_ID,
            pay_request("alice", 4_000),
        )
        .await
        .unwrap();
    assert_eq!(outcome["status"], "executed");
    assert_eq!(outcome["provider_response"]["ok"], true);
}

#[tokio::test]
async fn block_never_reaches_provider() {
    let state = seeded_state().await;
    let provider = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/payments"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({"ok": true})))
        .expect(0)
        .mount(&provider)
        .await;
    create_payment_connection(&state, &provider.uri()).await;

    let outcome = gate(&state)
        .pay(
            DEFAULT_WORKSPACE_ID,
            DEFAULT_ENVIRONMENT_ID,
            pay_request("alice", 80_000),
        )
        .await
        .unwrap();
    assert_eq!(outcome["status"], "block");
}

#[tokio::test]
async fn hold_executes_only_after_approve() {
    let state = seeded_state().await;
    let provider = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/payments"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({"ok": true})))
        .expect(1)
        .mount(&provider)
        .await;
    create_payment_connection(&state, &provider.uri()).await;
    let gate = gate(&state);

    // 6_000 is in the hold band [5_000, 10_000) → held, nothing forwarded.
    let outcome = gate
        .pay(
            DEFAULT_WORKSPACE_ID,
            DEFAULT_ENVIRONMENT_ID,
            pay_request("alice", 6_000),
        )
        .await
        .unwrap();
    assert_eq!(outcome["status"], "hold");
    let decision_id = outcome["decision_id"].as_str().unwrap().to_string();
    assert!(provider.received_requests().await.unwrap().is_empty());

    // Approve → executed exactly once, idempotency-keyed by the decision id.
    let resolved = gate
        .resolve_hold(
            DEFAULT_WORKSPACE_ID,
            DEFAULT_ENVIRONMENT_ID,
            &decision_id,
            true,
        )
        .await
        .unwrap();
    assert_eq!(resolved["status"], "executed");
    let requests = provider.received_requests().await.unwrap();
    assert_eq!(requests.len(), 1);
    assert_eq!(
        requests[0].headers.get("idempotency-key").unwrap(),
        decision_id.as_str()
    );

    // Second approve is refused — still exactly one provider call.
    let again = gate
        .resolve_hold(
            DEFAULT_WORKSPACE_ID,
            DEFAULT_ENVIRONMENT_ID,
            &decision_id,
            true,
        )
        .await
        .unwrap();
    assert_eq!(again["status"], "already_approved");
    assert_eq!(provider.received_requests().await.unwrap().len(), 1);
}

#[tokio::test]
async fn denied_hold_never_executes() {
    let state = seeded_state().await;
    let provider = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/payments"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({"ok": true})))
        .expect(0)
        .mount(&provider)
        .await;
    create_payment_connection(&state, &provider.uri()).await;
    let gate = gate(&state);

    let outcome = gate
        .pay(
            DEFAULT_WORKSPACE_ID,
            DEFAULT_ENVIRONMENT_ID,
            pay_request("alice", 6_000),
        )
        .await
        .unwrap();
    let decision_id = outcome["decision_id"].as_str().unwrap().to_string();
    let resolved = gate
        .resolve_hold(
            DEFAULT_WORKSPACE_ID,
            DEFAULT_ENVIRONMENT_ID,
            &decision_id,
            false,
        )
        .await
        .unwrap();
    assert_eq!(resolved["status"], "denied");
}

#[tokio::test]
async fn provider_error_reported_honestly() {
    let state = seeded_state().await;
    let provider = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/payments"))
        .respond_with(ResponseTemplate::new(500))
        .mount(&provider)
        .await;
    create_payment_connection(&state, &provider.uri()).await;

    let outcome = gate(&state)
        .pay(
            DEFAULT_WORKSPACE_ID,
            DEFAULT_ENVIRONMENT_ID,
            pay_request("alice", 4_000),
        )
        .await
        .unwrap();
    assert_eq!(outcome["status"], "allow_failed_execute");
}

/// An approved-and-executed hold counts toward the daily window: 6_000
/// executed + 5_000 proposed > 10_000 cap → the second pay is blocked.
#[tokio::test]
async fn approved_hold_counts_toward_daily_cap() {
    let state = memory_app_state(Arc::new(Engine::empty()));
    let gate = gate(&state);
    gate.set_policy(
        DEFAULT_WORKSPACE_ID,
        DEFAULT_ENVIRONMENT_ID,
        SpendCaps {
            owner: "alice".into(),
            per_transaction_minor: None,
            daily_minor: Some(10_000),
            monthly_minor: None,
            hold_above_minor: Some(5_000),
        },
    )
    .await
    .unwrap();

    let provider = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/payments"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({"ok": true})))
        .expect(1)
        .mount(&provider)
        .await;
    create_payment_connection(&state, &provider.uri()).await;

    let held = gate
        .pay(
            DEFAULT_WORKSPACE_ID,
            DEFAULT_ENVIRONMENT_ID,
            pay_request("alice", 6_000),
        )
        .await
        .unwrap();
    assert_eq!(held["status"], "hold");
    let decision_id = held["decision_id"].as_str().unwrap().to_string();
    let resolved = gate
        .resolve_hold(
            DEFAULT_WORKSPACE_ID,
            DEFAULT_ENVIRONMENT_ID,
            &decision_id,
            true,
        )
        .await
        .unwrap();
    assert_eq!(resolved["status"], "executed");

    // 6_000 already spent today; 5_000 more would break the 10_000 cap.
    let second = gate
        .pay(
            DEFAULT_WORKSPACE_ID,
            DEFAULT_ENVIRONMENT_ID,
            pay_request("alice", 4_999),
        )
        .await
        .unwrap();
    assert_eq!(
        second["status"], "block",
        "daily cap must count the executed hold"
    );
}

// ---------------------------------------------------------------------------
// Regression coverage for code-review findings (PR #281):
//  - negative/zero amount never reaches the provider (CRITICAL)
//  - a failed hold execution stays retryable, not permanently stuck (HIGH)
// ---------------------------------------------------------------------------

#[tokio::test]
async fn negative_amount_blocked_never_reaches_provider() {
    let state = seeded_state().await;
    let provider = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/payments"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({"ok": true})))
        .expect(0)
        .mount(&provider)
        .await;
    create_payment_connection(&state, &provider.uri()).await;

    // Negative amount would slip under every `amount > cap` check if unguarded.
    let outcome = gate(&state)
        .pay(
            DEFAULT_WORKSPACE_ID,
            DEFAULT_ENVIRONMENT_ID,
            pay_request("alice", -999_999),
        )
        .await
        .unwrap();
    assert_eq!(outcome["status"], "block");
    assert!(provider.received_requests().await.unwrap().is_empty());
}

#[tokio::test]
async fn zero_amount_blocked() {
    let state = seeded_state().await;
    let outcome = gate(&state)
        .pay(
            DEFAULT_WORKSPACE_ID,
            DEFAULT_ENVIRONMENT_ID,
            pay_request("alice", 0),
        )
        .await
        .unwrap();
    assert_eq!(outcome["status"], "block");
}

/// A hold approval whose forward fails must NOT record acceptance, so a
/// retry can still execute it — the payment is never permanently stuck.
#[tokio::test]
async fn failed_hold_execution_is_retryable() {
    let state = seeded_state().await;
    let provider = MockServer::start().await;
    // First approval attempt: provider is down (500) → 1 up, then removed.
    Mock::given(method("POST"))
        .and(path("/payments"))
        .respond_with(ResponseTemplate::new(500))
        .up_to_n_times(1)
        .expect(1)
        .mount(&provider)
        .await;
    // Retry: provider recovers.
    Mock::given(method("POST"))
        .and(path("/payments"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({"ok": true})))
        .expect(1)
        .mount(&provider)
        .await;
    create_payment_connection(&state, &provider.uri()).await;
    let gate = gate(&state);

    let held = gate
        .pay(
            DEFAULT_WORKSPACE_ID,
            DEFAULT_ENVIRONMENT_ID,
            pay_request("alice", 6_000),
        )
        .await
        .unwrap();
    assert_eq!(held["status"], "hold");
    let decision_id = held["decision_id"].as_str().unwrap().to_string();

    // First approve fails to execute — must be a failure status, not stuck.
    let first = gate
        .resolve_hold(
            DEFAULT_WORKSPACE_ID,
            DEFAULT_ENVIRONMENT_ID,
            &decision_id,
            true,
        )
        .await
        .unwrap();
    assert_eq!(first["status"], "approved_failed_execute");

    // Retry executes: proves acceptance was NOT recorded on the failed attempt
    // (otherwise this would short-circuit to already_approved).
    let second = gate
        .resolve_hold(
            DEFAULT_WORKSPACE_ID,
            DEFAULT_ENVIRONMENT_ID,
            &decision_id,
            true,
        )
        .await
        .unwrap();
    assert_eq!(second["status"], "executed");
}
