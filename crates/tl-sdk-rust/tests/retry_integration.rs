//! End-to-end retry / auth tests for `Client`. Spins up a `wiremock`
//! server and asserts the SDK retries 503/429, honors `Retry-After`, sends
//! the bearer token, and surfaces the right typed error on terminal
//! failures.
//!
//! Pinned timing: `RetryConfig` is set to small `base_delay` /
//! `total_budget` values so these tests stay fast (<1s) while still
//! exercising the real `tokio::time::sleep` path.

use std::time::Duration;

use tl_sdk_rust::{
    Action, AuthorizationEffect, Client, EventKind, GuardEvent, Principal, ProvenanceMap,
    RetryConfig, SdkError,
};
use wiremock::matchers::{header, method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

fn event() -> GuardEvent {
    GuardEvent {
        kind: EventKind::OutputProposed,
        principal: Principal {
            workspace_id: String::new(),
            environment_id: String::new(),
            agent_id: "agent-a".into(),
            user_id: None,
            session_id: None,
            task_id: None,
            run_id: None,
            run_event_id: None,
        },
        action: Action {
            operation: "output".into(),
            parameters: serde_json::json!({ "text": "hello" }),
            side_effect: None,
            invocation_id: None,
            tool_identity: None,
            authorization: None,
        },
        sources: vec![],
        provenance: ProvenanceMap::default(),
        resolution: None,
        label_resolution: None,
        checks: vec![],
        signals: vec![],
        context: serde_json::json!({
            "channel": "chat",
            "domain": "customer_support",
        }),
    }
}
fn fast_retry() -> RetryConfig {
    // Tight enough to keep the test under a second even with 3 retries.
    RetryConfig {
        max_attempts: 4,
        total_budget: Duration::from_secs(2),
        base_delay: Duration::from_millis(5),
        max_delay: Duration::from_millis(50),
    }
}

fn ok_decision_body() -> serde_json::Value {
    serde_json::json!({
        "trace_id": "trace-1",
        "domain": "content",
        "effect": "permit",
        "reason": "no policies triggered",
        "findings": [],
        "latency_ms": 1,
    })
}

#[tokio::test]
async fn retries_503_until_success() {
    let server = MockServer::start().await;

    // First two calls 503, third succeeds.
    Mock::given(method("POST"))
        .and(path("/v1/events"))
        .respond_with(ResponseTemplate::new(503))
        .up_to_n_times(2)
        .mount(&server)
        .await;
    Mock::given(method("POST"))
        .and(path("/v1/events"))
        .respond_with(ResponseTemplate::new(200).set_body_json(ok_decision_body()))
        .mount(&server)
        .await;

    let client = Client::new(server.uri()).with_retry(fast_retry());
    let decision = client
        .submit_event(&event())
        .await
        .expect("retry should succeed");
    assert_eq!(decision.effect, AuthorizationEffect::Permit);

    let calls = server.received_requests().await.unwrap();
    assert_eq!(calls.len(), 3, "expected 1 initial + 2 retries");
}

#[tokio::test]
async fn does_not_retry_401() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/v1/events"))
        .respond_with(ResponseTemplate::new(401).set_body_json(serde_json::json!({
            "code": "unauthorized",
            "message": "bad token",
            "retriable": false,
        })))
        .mount(&server)
        .await;

    let client = Client::new(server.uri()).with_retry(fast_retry());
    let err = client.submit_event(&event()).await.unwrap_err();
    assert!(matches!(err, SdkError::Unauthorized(_)));

    let calls = server.received_requests().await.unwrap();
    assert_eq!(calls.len(), 1, "Unauthorized must not retry");
}

#[tokio::test]
async fn honors_retry_after_header() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/v1/events"))
        .respond_with(
            ResponseTemplate::new(429)
                .insert_header("retry-after", "0")
                .set_body_json(serde_json::json!({
                    "code": "rate_limited",
                    "message": "too many",
                    "retriable": true,
                })),
        )
        .up_to_n_times(1)
        .mount(&server)
        .await;
    Mock::given(method("POST"))
        .and(path("/v1/events"))
        .respond_with(ResponseTemplate::new(200).set_body_json(ok_decision_body()))
        .mount(&server)
        .await;

    let client = Client::new(server.uri()).with_retry(fast_retry());
    let decision = client.submit_event(&event()).await.unwrap();
    assert_eq!(decision.effect, AuthorizationEffect::Permit);
    assert_eq!(server.received_requests().await.unwrap().len(), 2);
}

#[tokio::test]
async fn sends_bearer_auth_header() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/v1/events"))
        .and(header("authorization", "Bearer sk-abc"))
        .respond_with(ResponseTemplate::new(200).set_body_json(ok_decision_body()))
        .mount(&server)
        .await;

    let client = Client::new(server.uri())
        .with_api_key("sk-abc")
        .with_retry(fast_retry());
    let decision = client.submit_event(&event()).await.unwrap();
    assert_eq!(decision.effect, AuthorizationEffect::Permit);
}

#[tokio::test]
async fn gives_up_after_max_attempts() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/v1/events"))
        .respond_with(ResponseTemplate::new(503))
        .mount(&server)
        .await;

    let cfg = RetryConfig {
        max_attempts: 3,
        ..fast_retry()
    };
    let client = Client::new(server.uri()).with_retry(cfg);
    let err = client.submit_event(&event()).await.unwrap_err();
    assert!(matches!(err, SdkError::Unavailable(_)));
    assert_eq!(
        server.received_requests().await.unwrap().len(),
        3,
        "max_attempts=3 should produce exactly 3 requests"
    );
}
