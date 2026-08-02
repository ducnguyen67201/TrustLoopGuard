//! Mocked-HTTP integration tests for `OpenAiClient` and `OpenRouterClient`.
//! Verifies request body shape, auth header, provider-specific headers
//! (e.g. `HTTP-Referer` for OpenRouter), and end-to-end response parsing.
//! Uses `wiremock` so the actual reqwest client makes a real local call.

use std::time::Duration;

use serde_json::{json, Value};
use tl_llm::{JsonSchema, LlmClient, OpenAiClient, OpenRouterClient};
use wiremock::matchers::{header, method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

fn ok_response() -> ResponseTemplate {
    ResponseTemplate::new(200).set_body_json(json!({
        "choices": [{
            "message": {
                "content": "{\"verdict\":\"allow\",\"score\":0.92}"
            }
        }],
        "usage": { "prompt_tokens": 11, "completion_tokens": 4 }
    }))
}

fn schema() -> JsonSchema {
    JsonSchema {
        name: "AuthorizationEffect".into(),
        schema: json!({
            "type": "object",
            "properties": {
                "verdict": { "type": "string" },
                "score":   { "type": "number" }
            },
            "required": ["verdict", "score"],
            "additionalProperties": false
        }),
    }
}

#[tokio::test]
async fn openai_sends_bearer_auth_and_json_schema_body() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/v1/chat/completions"))
        .and(header("authorization", "Bearer test-key"))
        .and(header("content-type", "application/json"))
        .respond_with(ok_response())
        .expect(1)
        .mount(&server)
        .await;

    let client = OpenAiClient::new("test-key")
        .unwrap()
        .with_base_url(server.uri());
    let out = client
        .complete(
            "gpt-4o-mini",
            "judge this",
            &schema(),
            Duration::from_secs(5),
        )
        .await
        .expect("response");
    assert_eq!(out.prompt_tokens, 11);
    assert_eq!(out.completion_tokens, 4);
    assert_eq!(out.json["verdict"], "allow");

    // Inspect the request body that was actually sent.
    let received = &server.received_requests().await.unwrap()[0];
    let body: Value = serde_json::from_slice(&received.body).unwrap();
    assert_eq!(body["model"], "gpt-4o-mini");
    assert_eq!(body["messages"][0]["role"], "user");
    assert_eq!(body["messages"][0]["content"], "judge this");
    assert_eq!(body["response_format"]["type"], "json_schema");
    assert_eq!(body["response_format"]["json_schema"]["strict"], true);
    assert_eq!(
        body["response_format"]["json_schema"]["name"],
        "AuthorizationEffect"
    );
}

#[tokio::test]
async fn openrouter_adds_http_referer() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/v1/chat/completions"))
        .and(header("authorization", "Bearer or-key"))
        .and(header("http-referer", "https://example.test/featherlane"))
        .respond_with(ok_response())
        .expect(1)
        .mount(&server)
        .await;

    let client = OpenRouterClient::new("or-key")
        .unwrap()
        .with_base_url(server.uri())
        .with_referer("https://example.test/featherlane");
    let out = client
        .complete(
            "openai/gpt-4o-mini",
            "judge this",
            &schema(),
            Duration::from_secs(5),
        )
        .await
        .expect("response");
    assert_eq!(out.json["verdict"], "allow");
}

#[tokio::test]
async fn non_2xx_yields_status_error() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/v1/chat/completions"))
        .respond_with(ResponseTemplate::new(429).set_body_string("rate limited"))
        .mount(&server)
        .await;

    let client = OpenAiClient::new("k").unwrap().with_base_url(server.uri());
    let err = client
        .complete("m", "p", &schema(), Duration::from_secs(5))
        .await
        .unwrap_err();
    match err {
        tl_llm::LlmError::Status(code, body) => {
            assert_eq!(code, 429);
            assert!(body.contains("rate limited"));
        }
        other => panic!("expected Status error, got {other:?}"),
    }
}

#[tokio::test]
async fn deadline_exceeded_yields_timeout() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/v1/chat/completions"))
        // Server delays well past the deadline.
        .respond_with(ok_response().set_delay(Duration::from_millis(500)))
        .mount(&server)
        .await;

    let client = OpenAiClient::new("k").unwrap().with_base_url(server.uri());
    let err = client
        .complete("m", "p", &schema(), Duration::from_millis(50))
        .await
        .unwrap_err();
    assert!(matches!(err, tl_llm::LlmError::Timeout(_)), "got {err:?}");
}

#[tokio::test]
async fn malformed_inner_json_yields_parse_error() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/v1/chat/completions"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "choices": [{ "message": { "content": "not valid json" } }]
        })))
        .mount(&server)
        .await;

    let client = OpenAiClient::new("k").unwrap().with_base_url(server.uri());
    let err = client
        .complete("m", "p", &schema(), Duration::from_secs(5))
        .await
        .unwrap_err();
    assert!(matches!(err, tl_llm::LlmError::Parse(_)), "got {err:?}");
}
