//! Integration coverage for the Rust SDK's opt-in monitoring sessions:
//! session-id tagging on `submit_event` and the fire-and-forget `record_event`
//! capture path.

use std::time::Duration;

use tl_sdk_rust::{Action, Client, EventKind, GuardEvent, Principal, ProvenanceMap, RetryConfig};
use wiremock::matchers::{method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

fn one_shot_retry() -> RetryConfig {
    RetryConfig {
        max_attempts: 1,
        total_budget: Duration::from_millis(50),
        base_delay: Duration::from_millis(1),
        max_delay: Duration::from_millis(2),
    }
}

fn event() -> GuardEvent {
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
        sources: vec![],
        provenance: ProvenanceMap::default(),
        resolution: None,
        label_resolution: None,
        checks: vec![],
        signals: vec![],
        context: serde_json::Value::Null,
    }
}

fn allow_decision() -> serde_json::Value {
    serde_json::json!({
        "trace_id": "018f1111-1111-7111-8111-111111111111",
        "verdict": "allow",
        "reason": "no policies triggered",
        "triggered_policies": [],
        "safe_output": null,
        "latency_ms": 2
    })
}

async fn mock_post(server: &MockServer, endpoint: &str) {
    Mock::given(method("POST"))
        .and(path(endpoint))
        .respond_with(ResponseTemplate::new(200).set_body_json(allow_decision()))
        .mount(server)
        .await;
}

/// Poll the mock server until `record_event`'s spawned task delivers,
/// instead of sleeping a fixed amount.
async fn wait_for_requests(server: &MockServer, count: usize) -> Vec<wiremock::Request> {
    for _ in 0..100 {
        let requests = server.received_requests().await.unwrap_or_default();
        if requests.len() >= count {
            return requests;
        }
        tokio::time::sleep(Duration::from_millis(10)).await;
    }
    panic!("mock server never received {count} request(s)");
}

#[tokio::test]
async fn monitoring_client_tags_submitted_events_with_session() {
    let server = MockServer::start().await;
    mock_post(&server, "/v1/events").await;

    let client = Client::new(server.uri())
        .with_retry(one_shot_retry())
        .with_monitoring();
    let session = client.session_id().expect("monitoring session").to_string();
    assert!(session.starts_with("sess_"), "got {session}");

    client.submit_event(&event()).await.unwrap();

    let requests = server.received_requests().await.unwrap();
    let body: serde_json::Value = serde_json::from_slice(&requests[0].body).unwrap();
    assert_eq!(body["principal"]["session_id"], session.as_str());
}

#[tokio::test]
async fn caller_explicit_session_is_never_overwritten() {
    let server = MockServer::start().await;
    mock_post(&server, "/v1/events").await;

    let client = Client::new(server.uri())
        .with_retry(one_shot_retry())
        .with_monitoring();

    let mut explicit = event();
    explicit.principal.session_id = Some("sess_mine".into());
    client.submit_event(&explicit).await.unwrap();

    let requests = server.received_requests().await.unwrap();
    let body: serde_json::Value = serde_json::from_slice(&requests[0].body).unwrap();
    assert_eq!(body["principal"]["session_id"], "sess_mine");
}

#[tokio::test]
async fn client_without_monitoring_sends_no_session_id() {
    let server = MockServer::start().await;
    mock_post(&server, "/v1/events").await;

    let client = Client::new(server.uri()).with_retry(one_shot_retry());
    assert_eq!(client.session_id(), None);

    client.submit_event(&event()).await.unwrap();

    let requests = server.received_requests().await.unwrap();
    for request in &requests {
        let body: serde_json::Value = serde_json::from_slice(&request.body).unwrap();
        // `session_id` is skip_serializing_if = None — the key must be
        // absent so untagged traffic stays byte-identical to before.
        assert!(body.get("session_id").is_none());
        assert!(body
            .get("principal")
            .is_none_or(|p| p.get("session_id").is_none()));
    }
}

#[tokio::test]
async fn record_event_delivers_without_blocking() {
    let server = MockServer::start().await;
    mock_post(&server, "/v1/events").await;

    let client = Client::new(server.uri())
        .with_retry(one_shot_retry())
        .with_monitoring();
    let session = client.session_id().expect("monitoring session").to_string();

    client.record_event(event());

    let requests = wait_for_requests(&server, 1).await;
    let body: serde_json::Value = serde_json::from_slice(&requests[0].body).unwrap();
    assert_eq!(body["action"]["operation"], "send_email");
    assert_eq!(body["principal"]["session_id"], session.as_str());
}

#[tokio::test]
async fn record_event_swallows_server_errors() {
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

    let client = Client::new(server.uri())
        .with_retry(one_shot_retry())
        .with_monitoring();

    // Must not panic or surface the failure; the request still goes out.
    client.record_event(event());

    wait_for_requests(&server, 1).await;
}
