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
