//! Integration coverage for the Rust SDK's observe-only event submission.

use std::time::Duration;

use tl_sdk_rust::{
    Action, Client, EventKind, GuardEvent, Labels, Origin, Principal, ProvenanceMap, RetryConfig,
    SdkError, Source, Verdict,
};
use wiremock::matchers::{header, method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

const OBSERVE_ONLY_REASON: &str = "observe-only: event recorded; checkers not yet enforcing";

fn one_shot_retry() -> RetryConfig {
    RetryConfig {
        max_attempts: 1,
        total_budget: Duration::from_millis(50),
        base_delay: Duration::from_millis(1),
        max_delay: Duration::from_millis(2),
    }
}

fn send_email_event() -> GuardEvent {
    let mut provenance = ProvenanceMap::default();
    provenance.insert("recipient", vec!["src.web".into()]);

    GuardEvent {
        kind: EventKind::ToolCallProposed,
        principal: Principal {
            workspace_id: "ws_1".into(),
            environment_id: "production".into(),
            agent_id: "agent-1".into(),
            user_id: None,
            session_id: None,
            task_id: None,
            run_id: None,
            run_event_id: None,
        },
        action: Action {
            operation: "send_email".into(),
            parameters: serde_json::json!({ "recipient": "a@b.c" }),
            side_effect: None,
        },
        sources: vec![Source {
            id: "src.web".into(),
            origin: Origin::Web,
            labels: Labels::default(),
            kind: Some("web_page".into()),
        }],
        provenance,
        resolution: None,
        label_resolution: None,
        checks: vec![],
        signals: vec![],
        context: serde_json::Value::Null,
    }
}

fn observe_only_decision() -> serde_json::Value {
    serde_json::json!({
        "trace_id": "018f1111-1111-7111-8111-111111111111",
        "verdict": "allow",
        "reason": OBSERVE_ONLY_REASON,
        "triggered_policies": [],
        "safe_output": null,
        "latency_ms": 2
    })
}

#[tokio::test]
async fn submit_event_posts_typed_event_with_bearer_auth() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/v1/events"))
        .and(header("authorization", "Bearer secret"))
        .respond_with(ResponseTemplate::new(200).set_body_json(observe_only_decision()))
        .mount(&server)
        .await;

    let client = Client::new(server.uri())
        .with_api_key("secret")
        .with_retry(one_shot_retry());

    let decision = client.submit_event(&send_email_event()).await.unwrap();

    assert_eq!(decision.verdict, Verdict::Allow);
    assert_eq!(decision.reason, OBSERVE_ONLY_REASON);

    let requests = server.received_requests().await.unwrap();
    let body: serde_json::Value = serde_json::from_slice(&requests[0].body).unwrap();
    assert_eq!(body["action"]["operation"], "send_email");
    assert_eq!(body["provenance"]["recipient"][0], "src.web");
}

#[tokio::test]
async fn submit_event_maps_server_error() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/v1/events"))
        .respond_with(ResponseTemplate::new(500).set_body_json(serde_json::json!({
            "code": "internal",
            "message": "boom",
            "retriable": false
        })))
        .mount(&server)
        .await;

    let client = Client::new(server.uri()).with_retry(one_shot_retry());

    let err = client.submit_event(&send_email_event()).await.unwrap_err();
    assert!(matches!(err, SdkError::Internal(_)), "got {err:?}");
}
