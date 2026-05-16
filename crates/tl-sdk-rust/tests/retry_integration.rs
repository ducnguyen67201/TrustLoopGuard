//! End-to-end retry / auth tests for `Client`. Spins up a `wiremock`
//! server and asserts the SDK retries 503/429, honors `Retry-After`, sends
//! the bearer token, and surfaces the right typed error on terminal
//! failures.
//!
//! Pinned timing: `RetryConfig` is set to small `base_delay` /
//! `total_budget` values so these tests stay fast (<1s) while still
//! exercising the real `tokio::time::sleep` path.

use std::time::Duration;

use tl_core::{Channel, CheckRequest, Verdict};
use tl_sdk_rust::{Client, RetryConfig, SdkError};
use wiremock::matchers::{header, method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

fn req() -> CheckRequest {
    CheckRequest {
        workspace_id: None,
        agent_id: "agent-a".into(),
        channel: Channel::Chat,
        input: "hi".into(),
        proposed_output: "hello".into(),
        domain: None,
        policies: vec![],
        context: serde_json::Value::Null,
        trace_id: None,
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
        "verdict": "allow",
        "reason": "no policies triggered",
        "triggered_policies": [],
        "safe_output": null,
        "latency_ms": 1,
        "tier_results": [],
    })
}

#[tokio::test]
async fn retries_503_until_success() {
    let server = MockServer::start().await;

    // First two calls 503, third succeeds.
    Mock::given(method("POST"))
        .and(path("/v1/check"))
        .respond_with(ResponseTemplate::new(503))
        .up_to_n_times(2)
        .mount(&server)
        .await;
    Mock::given(method("POST"))
        .and(path("/v1/check"))
        .respond_with(ResponseTemplate::new(200).set_body_json(ok_decision_body()))
        .mount(&server)
        .await;

    let client = Client::new(server.uri()).with_retry(fast_retry());
    let decision = client.check(&req()).await.expect("retry should succeed");
    assert_eq!(decision.verdict, Verdict::Allow);

    let calls = server.received_requests().await.unwrap();
    assert_eq!(calls.len(), 3, "expected 1 initial + 2 retries");
}

#[tokio::test]
async fn does_not_retry_401() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/v1/check"))
        .respond_with(ResponseTemplate::new(401).set_body_json(serde_json::json!({
            "code": "unauthorized",
            "message": "bad token",
            "retriable": false,
        })))
        .mount(&server)
        .await;

    let client = Client::new(server.uri()).with_retry(fast_retry());
    let err = client.check(&req()).await.unwrap_err();
    assert!(matches!(err, SdkError::Unauthorized(_)));

    let calls = server.received_requests().await.unwrap();
    assert_eq!(calls.len(), 1, "Unauthorized must not retry");
}

#[tokio::test]
async fn honors_retry_after_header() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/v1/check"))
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
        .and(path("/v1/check"))
        .respond_with(ResponseTemplate::new(200).set_body_json(ok_decision_body()))
        .mount(&server)
        .await;

    let client = Client::new(server.uri()).with_retry(fast_retry());
    let decision = client.check(&req()).await.unwrap();
    assert_eq!(decision.verdict, Verdict::Allow);
    assert_eq!(server.received_requests().await.unwrap().len(), 2);
}

#[tokio::test]
async fn sends_bearer_auth_header() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/v1/check"))
        .and(header("authorization", "Bearer sk-abc"))
        .respond_with(ResponseTemplate::new(200).set_body_json(ok_decision_body()))
        .mount(&server)
        .await;

    let client = Client::new(server.uri())
        .with_api_key("sk-abc")
        .with_retry(fast_retry());
    let decision = client.check(&req()).await.unwrap();
    assert_eq!(decision.verdict, Verdict::Allow);
}

#[tokio::test]
async fn gives_up_after_max_attempts() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/v1/check"))
        .respond_with(ResponseTemplate::new(503))
        .mount(&server)
        .await;

    let cfg = RetryConfig {
        max_attempts: 3,
        ..fast_retry()
    };
    let client = Client::new(server.uri()).with_retry(cfg);
    let err = client.check(&req()).await.unwrap_err();
    assert!(matches!(err, SdkError::Unavailable(_)));
    assert_eq!(
        server.received_requests().await.unwrap().len(),
        3,
        "max_attempts=3 should produce exactly 3 requests"
    );
}
