//! E2E tests for the agent-bound guardrail endpoints.
//!
//! Both endpoints run against in-memory stores. A stub `LlmClient`
//! returns a fixture policy set so the test is deterministic and
//! offline. Verifies:
//!
//! - `POST /v1/agents/{id}/guardrails/generate` persists each draft
//!   with `enabled=false`, `owner_agent_id`, and `when.agents=[id]`.
//! - `GET /v1/agents/{id}/guardrails` lists those policies.
//! - `DELETE /v1/agents/{id}` cascades to soft-delete the owned policies.
//! - Generating without a `system_prompt` returns 422.
//! - Generating for a missing agent returns 404.

use std::sync::Arc;
use std::time::Duration;

use async_trait::async_trait;
use axum::{
    body::Body,
    http::{header, Request, StatusCode},
    Router,
};
use http_body_util::BodyExt;
use tl_core::{GuardrailGenerateResponse, GuardrailListResponse};
use tl_llm::{JsonSchema, LlmClient, LlmError, LlmOutput};
use tl_server::{
    agents::{self, AgentState},
    environments::MemoryEnvironmentStore,
    policies::{self, GuardrailState, MemoryPolicyStore},
    MemoryAgentStore,
};
use tower::ServiceExt;

const AGENT_YAML: &str = r#"
agent_id: baker-9000
display_name: Baker 9000
system_prompt: |
  You are a baking assistant for Sweet Loaf. Help customers with
  bread, pastries, and cake recipes. Stay strictly in scope.
scope:
  in_scope:
    - baking recipes
    - ingredient substitutions
  out_of_scope:
    - medical advice
authority:
  cannot_promise:
    - dietary safety
tone:
  target: warm-helpful
  forbidden:
    - condescending
"#;

const AGENT_YAML_NO_PROMPT: &str = r#"
agent_id: silent-agent
display_name: Silent Agent
scope:
  in_scope:
    - help
authority: {}
tone:
  target: neutral
"#;

/// Stub `LlmClient` that always returns the same canned policy set —
/// shape matches `policy_set_draft_json_schema()`.
struct StubLlm;

#[async_trait]
impl LlmClient for StubLlm {
    async fn complete(
        &self,
        _model: &str,
        _prompt: &str,
        _schema: &JsonSchema,
        _deadline: Duration,
    ) -> Result<LlmOutput, LlmError> {
        let json = serde_json::json!({
            "policies": [
                {
                    "id": "no-customer-email-leak",
                    "description": "Block customer email patterns in responses.",
                    "match_type": "regex",
                    "match_value": "[a-zA-Z0-9._%+-]+@[a-zA-Z0-9.-]+\\.[A-Za-z]{2,}",
                    "action": "deny",
                    "severity": "high",
                    "rewrite": null
                },
                {
                    "id": "no-medical-claims",
                    "description": "Block claims about medical or dietary safety.",
                    "match_type": "literal",
                    "match_value": "is safe for diabetics",
                    "action": "deny",
                    "severity": "high",
                    "rewrite": null
                },
                {
                    "id": "no-refund-promises",
                    "description": "Refuse refund guarantees.",
                    "match_type": "literal",
                    "match_value": "guaranteed refund",
                    "action": "deny",
                    "severity": "critical",
                    "rewrite": null
                }
            ]
        });
        Ok(LlmOutput {
            json,
            prompt_tokens: 0,
            completion_tokens: 0,
        })
    }
}

fn build_app() -> Router {
    let agent_store = Arc::new(MemoryAgentStore::new());
    let policy_store = Arc::new(MemoryPolicyStore::new());

    let agent_state = AgentState {
        store: agent_store.clone(),
        policy_store: Some(policy_store.clone()),
    };
    let agent_routes = Router::new()
        .route(
            "/v1/agents",
            axum::routing::post(agents::upsert_agent).get(agents::list_agents),
        )
        .route(
            "/v1/agents/:id",
            axum::routing::get(agents::get_agent).delete(agents::delete_agent),
        )
        .with_state(agent_state);

    let guardrail_state = GuardrailState {
        agent_store,
        policy_store: policy_store.clone(),
        environment_store: Arc::new(MemoryEnvironmentStore::new()),
        draft_llm: Some(Arc::new(StubLlm) as Arc<dyn LlmClient>),
        draft_model: "stub-model".to_string(),
    };
    let guardrail_routes = Router::new()
        .route(
            "/v1/agents/:id/guardrails/generate",
            axum::routing::post(policies::generate_guardrails),
        )
        .route(
            "/v1/agents/:id/guardrails",
            axum::routing::get(policies::list_guardrails),
        )
        .with_state(guardrail_state);

    Router::new().merge(agent_routes).merge(guardrail_routes)
}

async fn read_body(resp: axum::response::Response) -> serde_json::Value {
    let bytes = resp.into_body().collect().await.unwrap().to_bytes();
    if bytes.is_empty() {
        serde_json::Value::Null
    } else {
        serde_json::from_slice(&bytes).unwrap_or(serde_json::Value::Null)
    }
}

fn workspace_request() -> axum::http::request::Builder {
    Request::builder().header("x-tlg-workspace-id", "ws")
}

async fn upsert_agent(app: &Router, yaml: &str) {
    let resp = app
        .clone()
        .oneshot(
            workspace_request()
                .method("POST")
                .uri("/v1/agents")
                .header(header::CONTENT_TYPE, "application/yaml")
                .body(Body::from(yaml.to_string()))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::CREATED);
}

#[tokio::test]
async fn generate_persists_each_draft_disabled_and_returns_them() {
    let app = build_app();
    upsert_agent(&app, AGENT_YAML).await;

    let resp = app
        .clone()
        .oneshot(
            workspace_request()
                .method("POST")
                .uri("/v1/agents/baker-9000/guardrails/generate")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);

    let body: GuardrailGenerateResponse = serde_json::from_value(read_body(resp).await).unwrap();
    assert_eq!(body.generated.len(), 3);
    for doc in &body.generated {
        assert!(!doc.enabled, "generated policies must start disabled");
    }
    let ids: Vec<&str> = body.generated.iter().map(|d| d.id.as_str()).collect();
    assert!(ids.contains(&"no-customer-email-leak"));
    assert!(ids.contains(&"no-medical-claims"));
    assert!(ids.contains(&"no-refund-promises"));
}

#[tokio::test]
async fn list_returns_policies_scoped_to_agent() {
    let app = build_app();
    upsert_agent(&app, AGENT_YAML).await;

    // Trigger generation, then list.
    let _ = app
        .clone()
        .oneshot(
            workspace_request()
                .method("POST")
                .uri("/v1/agents/baker-9000/guardrails/generate")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    let resp = app
        .clone()
        .oneshot(
            workspace_request()
                .method("GET")
                .uri("/v1/agents/baker-9000/guardrails")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let body: GuardrailListResponse = serde_json::from_value(read_body(resp).await).unwrap();
    assert_eq!(body.policies.len(), 3);
    for p in &body.policies {
        assert!(!p.enabled);
    }
}

#[tokio::test]
async fn list_for_unknown_agent_returns_empty() {
    let app = build_app();
    let resp = app
        .clone()
        .oneshot(
            workspace_request()
                .method("GET")
                .uri("/v1/agents/ghost/guardrails")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let body: GuardrailListResponse = serde_json::from_value(read_body(resp).await).unwrap();
    assert!(body.policies.is_empty());
}

#[tokio::test]
async fn delete_agent_cascades_to_owned_policies() {
    let app = build_app();
    upsert_agent(&app, AGENT_YAML).await;
    let _ = app
        .clone()
        .oneshot(
            workspace_request()
                .method("POST")
                .uri("/v1/agents/baker-9000/guardrails/generate")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    // Delete the agent.
    let resp = app
        .clone()
        .oneshot(
            workspace_request()
                .method("DELETE")
                .uri("/v1/agents/baker-9000")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::NO_CONTENT);

    // Owned policies must be gone from the per-agent list.
    let resp = app
        .clone()
        .oneshot(
            workspace_request()
                .method("GET")
                .uri("/v1/agents/baker-9000/guardrails")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    let body: GuardrailListResponse = serde_json::from_value(read_body(resp).await).unwrap();
    assert!(
        body.policies.is_empty(),
        "cascade must remove owned policies"
    );
}

#[tokio::test]
async fn generate_without_system_prompt_is_422() {
    let app = build_app();
    upsert_agent(&app, AGENT_YAML_NO_PROMPT).await;
    let resp = app
        .clone()
        .oneshot(
            workspace_request()
                .method("POST")
                .uri("/v1/agents/silent-agent/guardrails/generate")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::UNPROCESSABLE_ENTITY);
}

#[tokio::test]
async fn generate_for_missing_agent_is_404() {
    let app = build_app();
    let resp = app
        .clone()
        .oneshot(
            workspace_request()
                .method("POST")
                .uri("/v1/agents/ghost/guardrails/generate")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::NOT_FOUND);
}
