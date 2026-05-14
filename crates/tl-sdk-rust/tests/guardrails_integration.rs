//! Integration coverage for the agent-bound guardrail SDK methods.
//!
//! Uses `wiremock` to spin up an HTTP server that mimics
//! `POST /v1/agents/{id}/guardrails/generate` and
//! `GET  /v1/agents/{id}/guardrails`. Verifies happy-path JSON parsing,
//! URL encoding, bearer auth, and that the 404 / 422 / 503 error
//! statuses surface as typed `SdkError` variants.

use std::time::Duration;

use tl_sdk_rust::{Client, RetryConfig, SdkError};
use wiremock::matchers::{header, method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

fn one_shot_retry() -> RetryConfig {
    // No retries — tests assert specific status mappings without bouncing.
    RetryConfig {
        max_attempts: 1,
        total_budget: Duration::from_millis(50),
        base_delay: Duration::from_millis(1),
        max_delay: Duration::from_millis(2),
    }
}

fn generate_response_body() -> serde_json::Value {
    serde_json::json!({
        "generated": [
            {
                "id": "no-pii-leak",
                "description": "Block email addresses in responses.",
                "severity": "high",
                "enabled": false,
                "source_yaml": "id: no-pii-leak\nmatch:\n  regex: ...\n",
            },
            {
                "id": "no-medical-claims",
                "description": "Block medical safety claims.",
                "severity": "high",
                "enabled": false,
                "source_yaml": "id: no-medical-claims\n...\n",
            }
        ]
    })
}

fn list_response_body() -> serde_json::Value {
    serde_json::json!({
        "policies": [
            {
                "id": "no-pii-leak",
                "description": "Block email addresses in responses.",
                "severity": "high",
                "enabled": false,
            }
        ]
    })
}

#[tokio::test]
async fn generate_returns_typed_response() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/v1/agents/baker-9000/guardrails/generate"))
        .and(header("authorization", "Bearer secret"))
        .respond_with(ResponseTemplate::new(200).set_body_json(generate_response_body()))
        .mount(&server)
        .await;

    let client = Client::new(server.uri())
        .with_api_key("secret")
        .with_retry(one_shot_retry());

    let out = client.generate_guardrails("baker-9000").await.unwrap();
    assert_eq!(out.generated.len(), 2);
    assert!(out.generated.iter().all(|p| !p.enabled));
    assert!(out.generated.iter().any(|p| p.id == "no-pii-leak"));
}

#[tokio::test]
async fn generate_url_encodes_agent_id() {
    let server = MockServer::start().await;
    // The encoded path must contain `%2F` for the slash and `%20` for the space.
    Mock::given(method("POST"))
        .and(path("/v1/agents/team%2Fbaker%20one/guardrails/generate"))
        .respond_with(ResponseTemplate::new(200).set_body_json(generate_response_body()))
        .mount(&server)
        .await;

    let client = Client::new(server.uri()).with_retry(one_shot_retry());
    let out = client.generate_guardrails("team/baker one").await.unwrap();
    assert_eq!(out.generated.len(), 2);
}

#[tokio::test]
async fn generate_404_maps_to_not_found() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/v1/agents/ghost/guardrails/generate"))
        .respond_with(ResponseTemplate::new(404).set_body_json(serde_json::json!({
            "code": "not_found",
            "message": "agent `ghost` not found",
            "retriable": false,
            "details": null,
        })))
        .mount(&server)
        .await;

    let client = Client::new(server.uri()).with_retry(one_shot_retry());
    let err = client.generate_guardrails("ghost").await.unwrap_err();
    match err {
        SdkError::NotFound(api) => assert_eq!(api.code, tl_sdk_rust::ApiErrorCode::NotFound),
        other => panic!("expected NotFound, got {other:?}"),
    }
}

#[tokio::test]
async fn generate_422_maps_to_unprocessable() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/v1/agents/silent/guardrails/generate"))
        .respond_with(ResponseTemplate::new(422).set_body_json(serde_json::json!({
            "code": "unprocessable",
            "message": "agent has no system_prompt",
            "retriable": false,
            "details": null,
        })))
        .mount(&server)
        .await;

    let client = Client::new(server.uri()).with_retry(one_shot_retry());
    let err = client.generate_guardrails("silent").await.unwrap_err();
    match err {
        SdkError::Unprocessable(api) => {
            assert_eq!(api.code, tl_sdk_rust::ApiErrorCode::Unprocessable)
        }
        other => panic!("expected Unprocessable, got {other:?}"),
    }
}

#[tokio::test]
async fn generate_503_maps_to_unavailable_and_is_retriable() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/v1/agents/a/guardrails/generate"))
        .respond_with(ResponseTemplate::new(503).set_body_json(serde_json::json!({
            "code": "unavailable",
            "message": "no LLM configured",
            "retriable": true,
            "details": null,
        })))
        .mount(&server)
        .await;

    let client = Client::new(server.uri()).with_retry(one_shot_retry());
    let err = client.generate_guardrails("a").await.unwrap_err();
    match err {
        SdkError::Unavailable(api) => {
            assert_eq!(api.code, tl_sdk_rust::ApiErrorCode::Unavailable);
            assert!(api.retriable);
        }
        other => panic!("expected Unavailable, got {other:?}"),
    }
}

#[tokio::test]
async fn list_returns_owned_policies() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/v1/agents/baker-9000/guardrails"))
        .respond_with(ResponseTemplate::new(200).set_body_json(list_response_body()))
        .mount(&server)
        .await;

    let client = Client::new(server.uri()).with_retry(one_shot_retry());
    let out = client.list_guardrails("baker-9000").await.unwrap();
    assert_eq!(out.policies.len(), 1);
    assert_eq!(out.policies[0].id, "no-pii-leak");
}

#[tokio::test]
async fn list_for_unknown_agent_returns_empty() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/v1/agents/ghost/guardrails"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "policies": [],
        })))
        .mount(&server)
        .await;

    let client = Client::new(server.uri()).with_retry(one_shot_retry());
    let out = client.list_guardrails("ghost").await.unwrap();
    assert!(out.policies.is_empty());
}
