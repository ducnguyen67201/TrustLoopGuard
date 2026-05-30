//! Integration coverage for Rust SDK run-grouping helpers.

use std::time::Duration;

use tl_sdk_rust::{
    Client, CreateRunEventRequest, CreateRunRequest, RetryConfig, RunEventKind, RunKind, RunStatus,
    UpdateRunRequest,
};
use wiremock::matchers::{header, method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

fn one_shot_retry() -> RetryConfig {
    RetryConfig {
        max_attempts: 1,
        total_budget: Duration::from_millis(50),
        base_delay: Duration::from_millis(1),
        max_delay: Duration::from_millis(2),
    }
}

fn run_body(status: &str) -> serde_json::Value {
    serde_json::json!({
        "id": "018f1111-1111-7111-8111-111111111111",
        "workspace_id": "default",
        "environment_id": "production",
        "environment": "production",
        "agent_id": "agent-a",
        "kind": "chat_session",
        "status": status,
        "external_id": "chat/123",
        "metadata": {},
        "started_at": "2026-05-17T00:00:00Z",
        "ended_at": null,
        "created_at": "2026-05-17T00:00:00Z",
        "updated_at": "2026-05-17T00:00:00Z",
        "trace_count": 0,
        "blocked_count": 0,
        "rewritten_count": 0,
        "escalated_count": 0,
        "p95_latency_ms": null,
    })
}

fn event_body() -> serde_json::Value {
    serde_json::json!({
        "id": "018f2222-2222-7222-8222-222222222222",
        "workspace_id": "default",
        "run_id": "018f1111-1111-7111-8111-111111111111",
        "sequence": 1,
        "kind": "assistant_turn",
        "label": "Turn 1",
        "input_summary": "Customer asks for a refund",
        "output_summary": "Agent drafts answer",
        "metadata": {},
        "occurred_at": "2026-05-17T00:00:01Z",
        "created_at": "2026-05-17T00:00:01Z",
    })
}

#[tokio::test]
async fn start_run_posts_typed_request_with_bearer_auth() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/v1/runs"))
        .and(header("authorization", "Bearer secret"))
        .respond_with(ResponseTemplate::new(201).set_body_json(run_body("running")))
        .mount(&server)
        .await;

    let client = Client::new(server.uri())
        .with_api_key("secret")
        .with_retry(one_shot_retry());
    let run = client
        .start_run(CreateRunRequest {
            agent_id: "agent-a".into(),
            kind: RunKind::ChatSession,
            status: None,
            external_id: Some("chat/123".into()),
            metadata: serde_json::json!({}),
        })
        .await
        .unwrap();

    assert_eq!(run.id, "018f1111-1111-7111-8111-111111111111");
    assert_eq!(run.status, RunStatus::Running);
}

#[tokio::test]
async fn run_helpers_encode_ids_and_parse_typed_responses() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/v1/runs/run%2Fone"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "run": run_body("running"),
            "events": [event_body()],
            "traces": [],
        })))
        .mount(&server)
        .await;
    Mock::given(method("PATCH"))
        .and(path("/v1/runs/run%2Fone"))
        .respond_with(ResponseTemplate::new(200).set_body_json(run_body("completed")))
        .mount(&server)
        .await;
    Mock::given(method("POST"))
        .and(path("/v1/runs/run%2Fone/events"))
        .respond_with(ResponseTemplate::new(201).set_body_json(event_body()))
        .mount(&server)
        .await;
    Mock::given(method("GET"))
        .and(path("/v1/runs/run%2Fone/events"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "events": [event_body()],
        })))
        .mount(&server)
        .await;
    Mock::given(method("GET"))
        .and(path("/v1/runs/run%2Fone/traces"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "traces": [],
        })))
        .mount(&server)
        .await;

    let client = Client::new(server.uri()).with_retry(one_shot_retry());

    let detail = client.get_run("run/one").await.unwrap();
    assert_eq!(detail.events.len(), 1);

    let event = client
        .create_run_event(
            "run/one",
            CreateRunEventRequest {
                kind: RunEventKind::AssistantTurn,
                sequence: None,
                label: Some("Turn 1".into()),
                input_summary: None,
                output_summary: None,
                metadata: serde_json::json!({}),
                occurred_at: None,
            },
        )
        .await
        .unwrap();
    assert_eq!(event.kind, RunEventKind::AssistantTurn);

    let events = client.list_run_events("run/one").await.unwrap();
    assert_eq!(events.events.len(), 1);

    let traces = client.list_run_traces("run/one").await.unwrap();
    assert!(traces.traces.is_empty());

    let updated = client
        .update_run(
            "run/one",
            UpdateRunRequest {
                status: Some(RunStatus::Completed),
                metadata: None,
                ended_at: None,
            },
        )
        .await
        .unwrap();
    assert_eq!(updated.status, RunStatus::Completed);

    let finished = client.finish_run("run/one").await.unwrap();
    assert_eq!(finished.status, RunStatus::Completed);
}
