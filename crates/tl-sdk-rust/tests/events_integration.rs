//! Integration coverage for the Rust SDK's observe-only event submission.

use std::time::Duration;

use tl_sdk_rust::{
    Action, Client, CreateRunEventRequest, CreateRunRequest, EventKind, GuardEvent, Labels, Origin,
    Principal, ProvenanceMap, RetryConfig, RunEventKind, RunKind, SdkError, Source, Verdict,
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

#[tokio::test]
async fn run_scoped_client_attaches_run_and_event_ids() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/v1/runs"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "id": "018f1111-1111-7111-8111-111111111111",
            "workspace_id": "ws_1",
            "environment_id": "production",
            "environment": "production",
            "agent_id": "agent-1",
            "kind": "chat_session",
            "status": "running",
            "external_id": null,
            "metadata": {},
            "started_at": "2026-01-01T00:00:00Z",
            "ended_at": null,
            "created_at": "2026-01-01T00:00:00Z",
            "updated_at": "2026-01-01T00:00:00Z",
            "trace_count": 0,
            "blocked_count": 0,
            "rewritten_count": 0,
            "escalated_count": 0,
            "p95_latency_ms": null
        })))
        .mount(&server)
        .await;
    Mock::given(method("POST"))
        .and(path("/v1/runs/018f1111-1111-7111-8111-111111111111/events"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "id": "018f2222-2222-7222-8222-222222222222",
            "workspace_id": "ws_1",
            "run_id": "018f1111-1111-7111-8111-111111111111",
            "sequence": 1,
            "kind": "user_turn",
            "label": null,
            "input_summary": null,
            "output_summary": null,
            "metadata": {},
            "occurred_at": "2026-01-01T00:00:00Z",
            "created_at": "2026-01-01T00:00:00Z"
        })))
        .mount(&server)
        .await;
    Mock::given(method("PATCH"))
        .and(path("/v1/runs/018f1111-1111-7111-8111-111111111111"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "id": "018f1111-1111-7111-8111-111111111111",
            "workspace_id": "ws_1",
            "environment_id": "production",
            "environment": "production",
            "agent_id": "agent-1",
            "kind": "chat_session",
            "status": "completed",
            "external_id": null,
            "metadata": {},
            "started_at": "2026-01-01T00:00:00Z",
            "ended_at": "2026-01-01T00:00:01Z",
            "created_at": "2026-01-01T00:00:00Z",
            "updated_at": "2026-01-01T00:00:01Z",
            "trace_count": 1,
            "blocked_count": 0,
            "rewritten_count": 0,
            "escalated_count": 0,
            "p95_latency_ms": 2
        })))
        .mount(&server)
        .await;
    Mock::given(method("POST"))
        .and(path("/v1/events"))
        .respond_with(ResponseTemplate::new(200).set_body_json(observe_only_decision()))
        .mount(&server)
        .await;

    let client = Client::new(server.uri()).with_retry(one_shot_retry());
    client
        .with_run(
            CreateRunRequest {
                agent_id: "agent-1".into(),
                kind: RunKind::ChatSession,
                status: None,
                external_id: None,
                metadata: serde_json::json!({}),
            },
            |run| async move {
                run.with_event(
                    CreateRunEventRequest {
                        kind: RunEventKind::UserTurn,
                        sequence: None,
                        label: None,
                        input_summary: None,
                        output_summary: None,
                        metadata: serde_json::json!({}),
                        occurred_at: None,
                    },
                    |event_run| async move {
                        event_run.submit_event(&send_email_event()).await?;
                        Ok(())
                    },
                )
                .await
            },
        )
        .await
        .unwrap();

    let requests = server.received_requests().await.unwrap();
    let event_request = requests.iter().find(|request| request.url.path() == "/v1/events").unwrap();
    let body: serde_json::Value = serde_json::from_slice(&event_request.body).unwrap();
    assert_eq!(
        body["principal"]["run_id"],
        "018f1111-1111-7111-8111-111111111111"
    );
    assert_eq!(
        body["principal"]["run_event_id"],
        "018f2222-2222-7222-8222-222222222222"
    );
}
