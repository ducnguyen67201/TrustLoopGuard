//! E2E tests for configurable budget alert thresholds. Uses `wiremock`
//! as the webhook receiver (mirroring `tests/escalation.rs`) and
//! drives the real router: policy caps → alert config → spends →
//! deduped firings → webhook delivery.

use std::sync::Arc;
use std::time::Duration;

use axum::{
    body::Body,
    http::{header, Request, StatusCode},
};
use chrono::Utc;
use http_body_util::BodyExt;
use serde_json::{json, Value};
use tl_core::{
    BudgetAlertThresholdType, BudgetAlertWindow, CreateBudgetAlertConfigRequest, SpendMeter,
};
use tl_engine::Engine;
use tl_server::budget_alerts::{process_spend, BudgetAlertRuntime, BudgetAlertStore, WindowSpend};
use tl_server::{
    memory_app_state, router, spawn_webhook_delivery_worker, AppState, MemoryBudgetAlertStore,
    RetryPolicy,
};
use tower::ServiceExt;
use uuid::Uuid;
use wiremock::matchers::{method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

const SEAL_KEY: [u8; 32] = [0u8; 32];

fn fast_retry() -> RetryPolicy {
    RetryPolicy { delays: vec![] }
}

fn delivery_tx() -> tokio::sync::mpsc::Sender<tl_server::WebhookDelivery> {
    #[cfg(feature = "postgres")]
    let (tx, _handle) = spawn_webhook_delivery_worker(fast_retry(), 16, None);
    #[cfg(not(feature = "postgres"))]
    let (tx, _handle) = spawn_webhook_delivery_worker(fast_retry(), 16);
    tx
}

/// Local-dev app (no bearer middleware) with the alert delivery worker
/// running and one workspace owned by the returned user. Admin writes
/// authenticate via the forwarded-user header, spends via the
/// workspace header alone.
async fn app_with_owner() -> (axum::Router, AppState, String, Uuid) {
    let mut state = memory_app_state(Arc::new(Engine::empty()));
    state.budget_alert_tx = Some(delivery_tx());
    let owner_id = Uuid::new_v4();
    let workspace = state
        .team_store
        .create_workspace(owner_id, "Budget Alerts Workspace")
        .await
        .unwrap();
    (
        router(state.clone(), None, SEAL_KEY),
        state,
        workspace.id,
        owner_id,
    )
}

async fn read_body(resp: axum::response::Response) -> Value {
    let bytes = resp.into_body().collect().await.unwrap().to_bytes();
    if bytes.is_empty() {
        Value::Null
    } else {
        serde_json::from_slice(&bytes).unwrap()
    }
}

fn workspace_request(method: &str, uri: &str, workspace_id: &str, body: &Value) -> Request<Body> {
    Request::builder()
        .method(method)
        .uri(uri)
        .header(header::CONTENT_TYPE, "application/json")
        .header("x-tlg-workspace-id", workspace_id)
        .body(Body::from(body.to_string()))
        .unwrap()
}

fn admin_request(
    method: &str,
    uri: &str,
    workspace_id: &str,
    user_id: Uuid,
    body: &Value,
) -> Request<Body> {
    Request::builder()
        .method(method)
        .uri(uri)
        .header(header::CONTENT_TYPE, "application/json")
        .header("x-tlg-workspace-id", workspace_id)
        .header("x-tlg-user-id", user_id.to_string())
        .body(Body::from(body.to_string()))
        .unwrap()
}

/// Weekly cap that matches every financial action in the workspace.
async fn create_weekly_cap(app: &axum::Router, workspace_id: &str, weekly_minor: i64) {
    let resp = app
        .clone()
        .oneshot(workspace_request(
            "POST",
            "/v1/financial/policies",
            workspace_id,
            &json!({
                "id": "weekly-spend-cap",
                "description": "Weekly spend cap",
                "when": { "agents": ["refund-bot"] },
                "weekly_minor": weekly_minor
            }),
        ))
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::CREATED);
}

async fn create_alert(
    app: &axum::Router,
    workspace_id: &str,
    owner_id: Uuid,
    body: Value,
) -> Value {
    let resp = app
        .clone()
        .oneshot(admin_request(
            "POST",
            "/v1/financial/budget-alerts",
            workspace_id,
            owner_id,
            &body,
        ))
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::CREATED, "alert create failed");
    read_body(resp).await
}

/// Create + auto-execute a refund so the spend lands in the ledger.
async fn spend(app: &axum::Router, workspace_id: &str, idempotency_key: &str, amount_minor: i64) {
    let resp = app
        .clone()
        .oneshot(workspace_request(
            "POST",
            "/v1/financial/actions",
            workspace_id,
            &json!({
                "idempotency_key": idempotency_key,
                "execute": true,
                "action": {
                    "kind": "refund",
                    "operation": "issue_refund",
                    "principal_id": "refund-bot",
                    "amount": { "amount_minor": amount_minor, "currency": "USD" },
                    "counterparty": {
                        "id": "cust_456",
                        "display_name": "Casey Customer",
                        "kind": "customer",
                        "country": "US",
                        "metadata": {}
                    },
                    "rail": "card",
                    "memo": "spend",
                    "metadata": {}
                },
                "evidence": []
            }),
        ))
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::CREATED);
    let body = read_body(resp).await;
    assert_eq!(body["status"], "executed", "spend did not execute: {body}");
}

async fn wait_for_requests(server: &MockServer, at_least: usize) -> Vec<wiremock::Request> {
    let deadline = std::time::Instant::now() + Duration::from_secs(2);
    loop {
        let received = server.received_requests().await.unwrap_or_default();
        if received.len() >= at_least {
            return received;
        }
        if std::time::Instant::now() > deadline {
            panic!("expected {at_least} webhook posts, got {}", received.len());
        }
        tokio::time::sleep(Duration::from_millis(5)).await;
    }
}

async fn list_firings(app: &axum::Router, workspace_id: &str, config_id: &str) -> Vec<Value> {
    let resp = app
        .clone()
        .oneshot(workspace_request(
            "GET",
            &format!("/v1/financial/budget-alerts/{config_id}/firings"),
            workspace_id,
            &json!({}),
        ))
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    read_body(resp).await["firings"]
        .as_array()
        .cloned()
        .unwrap_or_default()
}

#[tokio::test]
async fn percent_crossing_fires_webhook_once_with_correct_payload() {
    let receiver = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/alerts"))
        .respond_with(ResponseTemplate::new(200))
        .mount(&receiver)
        .await;

    let (app, _state, workspace_id, owner_id) = app_with_owner().await;
    create_weekly_cap(&app, &workspace_id, 5_000).await;
    let config = create_alert(
        &app,
        &workspace_id,
        owner_id,
        json!({
            "name": "weekly-80",
            "window": "week",
            "threshold_type": "percent",
            "threshold_value": 80,
            "webhook_url": format!("{}/alerts", receiver.uri())
        }),
    )
    .await;
    let config_id = config["id"].as_str().unwrap();

    // 3000 spent: below 80% of 5000 → silent.
    spend(&app, &workspace_id, "spend-1", 3_000).await;
    tokio::time::sleep(Duration::from_millis(100)).await;
    assert!(receiver.received_requests().await.unwrap().is_empty());
    assert!(list_firings(&app, &workspace_id, config_id)
        .await
        .is_empty());

    // 1000 more: 4000 = exactly 80% → fires.
    spend(&app, &workspace_id, "spend-2", 1_000).await;
    let received = wait_for_requests(&receiver, 1).await;
    let payload: Value = serde_json::from_slice(&received[0].body).unwrap();
    assert_eq!(payload["type"], "budget_alert");
    assert_eq!(payload["workspace_id"], workspace_id.as_str());
    assert_eq!(payload["config_id"], config_id);
    assert_eq!(payload["config_name"], "weekly-80");
    assert_eq!(payload["principal_id"], "refund-bot");
    assert_eq!(payload["window"], "week");
    assert_eq!(payload["threshold_type"], "percent");
    assert_eq!(payload["threshold_value"], 80);
    assert_eq!(payload["cap_minor"], 5_000);
    assert_eq!(payload["spent_minor"], 4_000);
    assert_eq!(payload["remaining_minor"], 1_000);
    assert_eq!(payload["percent_used"], 80);
    assert_eq!(payload["currency"], "USD");

    // Dedup: another spend in the same window stays silent.
    spend(&app, &workspace_id, "spend-3", 200).await;
    tokio::time::sleep(Duration::from_millis(200)).await;
    assert_eq!(receiver.received_requests().await.unwrap().len(), 1);

    let firings = list_firings(&app, &workspace_id, config_id).await;
    assert_eq!(firings.len(), 1);
    assert_eq!(firings[0]["spent_minor"], 4_000);
    assert_eq!(firings[0]["cap_minor"], 5_000);
    assert_eq!(firings[0]["principal_id"], "refund-bot");
}

#[tokio::test]
async fn absolute_threshold_fires_when_remaining_drops_to_value() {
    let receiver = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/alerts"))
        .respond_with(ResponseTemplate::new(200))
        .expect(1)
        .mount(&receiver)
        .await;

    let (app, _state, workspace_id, owner_id) = app_with_owner().await;
    create_weekly_cap(&app, &workspace_id, 5_000).await;
    create_alert(
        &app,
        &workspace_id,
        owner_id,
        json!({
            "name": "weekly-1000-left",
            "window": "week",
            "threshold_type": "absolute",
            "threshold_value": 1_000,
            "webhook_url": format!("{}/alerts", receiver.uri())
        }),
    )
    .await;

    // 4200 spent → 800 remaining ≤ 1000 → fires.
    spend(&app, &workspace_id, "spend-abs", 4_200).await;
    let received = wait_for_requests(&receiver, 1).await;
    let payload: Value = serde_json::from_slice(&received[0].body).unwrap();
    assert_eq!(payload["threshold_type"], "absolute");
    assert_eq!(payload["remaining_minor"], 800);
    assert_eq!(payload["spent_minor"], 4_200);
}

#[tokio::test]
async fn disabled_config_stays_silent() {
    let receiver = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/alerts"))
        .respond_with(ResponseTemplate::new(200))
        .expect(0)
        .mount(&receiver)
        .await;

    let (app, _state, workspace_id, owner_id) = app_with_owner().await;
    create_weekly_cap(&app, &workspace_id, 5_000).await;
    let config = create_alert(
        &app,
        &workspace_id,
        owner_id,
        json!({
            "name": "weekly-80-disabled",
            "window": "week",
            "threshold_type": "percent",
            "threshold_value": 80,
            "webhook_url": format!("{}/alerts", receiver.uri()),
            "enabled": false
        }),
    )
    .await;
    let config_id = config["id"].as_str().unwrap();

    spend(&app, &workspace_id, "spend-disabled", 4_500).await;
    tokio::time::sleep(Duration::from_millis(200)).await;
    assert!(receiver.received_requests().await.unwrap().is_empty());
    assert!(list_firings(&app, &workspace_id, config_id)
        .await
        .is_empty());
}

#[tokio::test]
async fn no_webhook_anywhere_records_firing_without_send_or_error() {
    let (app, _state, workspace_id, owner_id) = app_with_owner().await;
    create_weekly_cap(&app, &workspace_id, 5_000).await;
    // No per-config webhook_url; workspace escalation_webhook_url is
    // unset by default → dashboard-only alert.
    let config = create_alert(
        &app,
        &workspace_id,
        owner_id,
        json!({
            "name": "weekly-80-dashboard-only",
            "window": "week",
            "threshold_type": "percent",
            "threshold_value": 80
        }),
    )
    .await;
    let config_id = config["id"].as_str().unwrap();

    // The spend succeeds and the firing is recorded — no delivery, no
    // error surfaced to the spend path.
    spend(&app, &workspace_id, "spend-quiet", 4_000).await;
    tokio::time::sleep(Duration::from_millis(100)).await;
    let firings = list_firings(&app, &workspace_id, config_id).await;
    assert_eq!(firings.len(), 1);
    assert_eq!(firings[0]["spent_minor"], 4_000);
}

/// New window → fires again. Driven through `process_spend` directly
/// because a router test cannot time-travel across week boundaries.
#[tokio::test]
async fn new_window_fires_again_through_the_delivery_pipeline() {
    let receiver = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/alerts"))
        .respond_with(ResponseTemplate::new(200))
        .expect(2)
        .mount(&receiver)
        .await;

    let state = memory_app_state(Arc::new(Engine::empty()));
    let store: Arc<dyn BudgetAlertStore> = Arc::new(MemoryBudgetAlertStore::new());
    let config = store
        .create_config(
            "ws",
            CreateBudgetAlertConfigRequest {
                name: "weekly-80".into(),
                meter: SpendMeter::Actions,
                window: BudgetAlertWindow::Week,
                principal_id: None,
                threshold_type: BudgetAlertThresholdType::Percent,
                threshold_value: 80,
                webhook_url: Some(format!("{}/alerts", receiver.uri())),
                enabled: Some(true),
            },
        )
        .await
        .unwrap();
    let runtime = BudgetAlertRuntime {
        store: store.clone(),
        settings: state.settings_store.clone(),
        delivery_tx: Some(delivery_tx()),
    };
    let configs = vec![config];
    let this_week = Utc::now();
    let next_week = this_week + chrono::Duration::days(7);

    for (window_start, spent) in [(this_week, 4_000), (this_week, 4_500), (next_week, 4_200)] {
        process_spend(
            &runtime,
            "ws",
            "user:daniel",
            "USD",
            &configs,
            &[WindowSpend {
                window: BudgetAlertWindow::Week,
                window_start,
                cap_minor: 5_000,
                spent_minor: spent,
            }],
        )
        .await;
    }

    // First crossing fires, second in the same window dedups, the new
    // window fires again.
    let received = wait_for_requests(&receiver, 2).await;
    assert_eq!(received.len(), 2);
    assert_eq!(
        store
            .list_firings("ws", &configs[0].id)
            .await
            .unwrap()
            .len(),
        2
    );
}

#[tokio::test]
async fn validation_rejects_bad_thresholds_and_uncapped_scopes() {
    let (app, _state, workspace_id, owner_id) = app_with_owner().await;
    create_weekly_cap(&app, &workspace_id, 5_000).await;

    // Percent out of range.
    for value in [0, 101, -5] {
        let resp = app
            .clone()
            .oneshot(admin_request(
                "POST",
                "/v1/financial/budget-alerts",
                &workspace_id,
                owner_id,
                &json!({
                    "name": "bad-percent",
                    "window": "week",
                    "threshold_type": "percent",
                    "threshold_value": value
                }),
            ))
            .await
            .unwrap();
        assert_eq!(resp.status(), StatusCode::BAD_REQUEST);
    }

    // Window without a cap: only a weekly cap exists.
    let resp = app
        .clone()
        .oneshot(admin_request(
            "POST",
            "/v1/financial/budget-alerts",
            &workspace_id,
            owner_id,
            &json!({
                "name": "monthly-alert",
                "window": "month",
                "threshold_type": "percent",
                "threshold_value": 80
            }),
        ))
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::BAD_REQUEST);
    let body = read_body(resp).await;
    assert_eq!(body["message"], "no month cap configured for this scope");

    // An llm_usage cap scoped to non-USD currencies is unreachable —
    // LLM metering is USD-only — so it must not validate a scope on its
    // own. Only a monthly cap exists here, and it is EUR-scoped.
    let resp = app
        .clone()
        .oneshot(workspace_request(
            "POST",
            "/v1/financial/policies",
            &workspace_id,
            &json!({
                "id": "llm-monthly-eur",
                "meter": "llm_usage",
                "when": { "currencies": ["EUR"] },
                "monthly_minor": 10_000
            }),
        ))
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::CREATED);
    let resp = app
        .clone()
        .oneshot(admin_request(
            "POST",
            "/v1/financial/budget-alerts",
            &workspace_id,
            owner_id,
            &json!({
                "name": "monthly-alert-eur-llm",
                "window": "month",
                "threshold_type": "percent",
                "threshold_value": 80
            }),
        ))
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::BAD_REQUEST);
    let body = read_body(resp).await;
    assert_eq!(body["message"], "no month cap configured for this scope");

    // Bad webhook scheme.
    let resp = app
        .clone()
        .oneshot(admin_request(
            "POST",
            "/v1/financial/budget-alerts",
            &workspace_id,
            owner_id,
            &json!({
                "name": "bad-webhook",
                "window": "week",
                "threshold_type": "percent",
                "threshold_value": 80,
                "webhook_url": "ftp://example.com/hook"
            }),
        ))
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::BAD_REQUEST);

    // Mutations require an authenticated admin user.
    let resp = app
        .clone()
        .oneshot(workspace_request(
            "POST",
            "/v1/financial/budget-alerts",
            &workspace_id,
            &json!({
                "name": "no-user",
                "window": "week",
                "threshold_type": "percent",
                "threshold_value": 80
            }),
        ))
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn crud_round_trip_via_router() {
    let (app, _state, workspace_id, owner_id) = app_with_owner().await;
    create_weekly_cap(&app, &workspace_id, 5_000).await;
    let config = create_alert(
        &app,
        &workspace_id,
        owner_id,
        json!({
            "name": "weekly-80",
            "window": "week",
            "threshold_type": "percent",
            "threshold_value": 80
        }),
    )
    .await;
    let config_id = config["id"].as_str().unwrap();
    assert_eq!(config["enabled"], true);
    assert_eq!(config["principal_id"], Value::Null);

    // Duplicate name → conflict.
    let resp = app
        .clone()
        .oneshot(admin_request(
            "POST",
            "/v1/financial/budget-alerts",
            &workspace_id,
            owner_id,
            &json!({
                "name": "weekly-80",
                "window": "week",
                "threshold_type": "percent",
                "threshold_value": 50
            }),
        ))
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::CONFLICT);

    // List is readable with the workspace header alone.
    let resp = app
        .clone()
        .oneshot(workspace_request(
            "GET",
            "/v1/financial/budget-alerts",
            &workspace_id,
            &json!({}),
        ))
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    assert_eq!(
        read_body(resp).await["configs"].as_array().unwrap().len(),
        1
    );

    // Toggle off via PATCH.
    let resp = app
        .clone()
        .oneshot(admin_request(
            "PATCH",
            &format!("/v1/financial/budget-alerts/{config_id}"),
            &workspace_id,
            owner_id,
            &json!({ "enabled": false }),
        ))
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    assert_eq!(read_body(resp).await["enabled"], false);

    // Delete, then the list is empty and firings 404s.
    let resp = app
        .clone()
        .oneshot(admin_request(
            "DELETE",
            &format!("/v1/financial/budget-alerts/{config_id}"),
            &workspace_id,
            owner_id,
            &json!({}),
        ))
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::NO_CONTENT);
    let resp = app
        .clone()
        .oneshot(workspace_request(
            "GET",
            &format!("/v1/financial/budget-alerts/{config_id}/firings"),
            &workspace_id,
            &json!({}),
        ))
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::NOT_FOUND);
}
