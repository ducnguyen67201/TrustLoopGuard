use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::time::Duration;

use async_trait::async_trait;
use axum::{body::Body, http::Request, routing::post, Router};
use http_body_util::BodyExt;
use serde_json::{json, Value};
use tl_llm::{
    JsonSchema, LlmClient, LlmError, LlmOutput, LlmRouteKind, LlmRouter, ProviderTarget,
    ResolvedRoute, TokenBudget,
};
use tl_server::{
    environments::MemoryEnvironmentStore,
    policies::{self, MemoryPolicyStore, PolicyState},
    MemoryTeamStore,
};
use tower::ServiceExt;

#[derive(Debug, Clone, PartialEq, Eq)]
struct RecordedCall {
    model: String,
    deadline: Duration,
    schema_name: String,
}

struct RecordingLlm {
    calls: Arc<Mutex<Vec<RecordedCall>>>,
    fail: bool,
}

#[async_trait]
impl LlmClient for RecordingLlm {
    async fn complete(
        &self,
        model: &str,
        _prompt: &str,
        schema: &JsonSchema,
        deadline: Duration,
    ) -> Result<LlmOutput, LlmError> {
        self.calls
            .lock()
            .expect("calls lock poisoned")
            .push(RecordedCall {
                model: model.to_string(),
                deadline,
                schema_name: schema.name.clone(),
            });
        if self.fail {
            return Err(LlmError::Http("fixture provider failed".into()));
        }

        let json = match schema.name.as_str() {
            "policy_draft" => json!({
                "id": "block-secret-sharing",
                "description": "Block secret sharing",
                "match_type": "literal",
                "match_value": "secret",
                "action": "deny",
                "severity": "high",
                "rewrite": null
            }),
            "yaml_edit_result" => json!({ "yaml": "id: edited-policy\n" }),
            other => panic!("unexpected schema {other}"),
        };
        Ok(LlmOutput {
            json,
            prompt_tokens: 8,
            completion_tokens: 4,
        })
    }
}

fn policy_router(llm: Arc<LlmRouter>) -> Router {
    Router::new()
        .route("/v1/policies/draft", post(policies::draft_policy))
        .route("/v1/policies/ai-edit", post(policies::ai_edit_policy))
        .with_state(PolicyState {
            store: Arc::new(MemoryPolicyStore::new()),
            environment_store: Arc::new(MemoryEnvironmentStore::new()),
            team_store: Arc::new(MemoryTeamStore::new()),
            llm,
        })
}

fn configured_router(calls: Arc<Mutex<Vec<RecordedCall>>>, fail: bool) -> Arc<LlmRouter> {
    let mut providers: HashMap<String, Arc<dyn LlmClient>> = HashMap::new();
    providers.insert("openai".into(), Arc::new(RecordingLlm { calls, fail }));
    let mut routes = HashMap::new();
    routes.insert(
        LlmRouteKind::PolicyDraft,
        ResolvedRoute {
            primary: target("draft-model", 30_000),
            fallback: None,
        },
    );
    routes.insert(
        LlmRouteKind::PolicyAiEdit,
        ResolvedRoute {
            primary: target("edit-model", 31_000),
            fallback: None,
        },
    );
    Arc::new(LlmRouter::new(
        providers,
        routes,
        Arc::new(TokenBudget::new(1)),
    ))
}

fn target(model: &str, deadline_ms: u32) -> ProviderTarget {
    ProviderTarget {
        provider: "openai".into(),
        model: model.into(),
        deadline_ms,
        reasoning_effort: None,
    }
}

async fn post_json(app: Router, path: &str, body: Value) -> (u16, Value) {
    let response = app
        .oneshot(
            Request::post(path)
                .header("content-type", "application/json")
                .body(Body::from(body.to_string()))
                .expect("request"),
        )
        .await
        .expect("response");
    let status = response.status().as_u16();
    let bytes = response
        .into_body()
        .collect()
        .await
        .expect("body")
        .to_bytes();
    (
        status,
        serde_json::from_slice(&bytes).expect("json response"),
    )
}

#[tokio::test]
async fn policy_endpoints_use_their_configured_routes() {
    let calls = Arc::new(Mutex::new(Vec::new()));
    let app = policy_router(configured_router(calls.clone(), false));

    let (draft_status, draft_body) = post_json(
        app.clone(),
        "/v1/policies/draft",
        json!({ "prompt": "block secret sharing" }),
    )
    .await;
    assert_eq!(draft_status, 200);
    assert_eq!(draft_body["draft"]["id"], "block-secret-sharing");

    let (edit_status, edit_body) = post_json(
        app,
        "/v1/policies/ai-edit",
        json!({ "yaml": "id: old", "instruction": "rename it" }),
    )
    .await;
    assert_eq!(edit_status, 200);
    assert_eq!(edit_body["yaml"], "id: edited-policy");

    assert_eq!(
        *calls.lock().expect("calls lock poisoned"),
        vec![
            RecordedCall {
                model: "draft-model".into(),
                deadline: Duration::from_secs(30),
                schema_name: "policy_draft".into(),
            },
            RecordedCall {
                model: "edit-model".into(),
                deadline: Duration::from_secs(31),
                schema_name: "yaml_edit_result".into(),
            },
        ]
    );
}

#[tokio::test]
async fn missing_policy_route_preserves_service_unavailable() {
    let (status, body) = post_json(
        policy_router(Arc::new(LlmRouter::empty())),
        "/v1/policies/draft",
        json!({ "prompt": "block secret sharing" }),
    )
    .await;

    assert_eq!(status, 503);
    assert_eq!(body["code"], "unavailable");
}

#[tokio::test]
async fn provider_failure_preserves_bad_gateway() {
    let calls = Arc::new(Mutex::new(Vec::new()));
    let (status, body) = post_json(
        policy_router(configured_router(calls, true)),
        "/v1/policies/ai-edit",
        json!({ "yaml": "id: old", "instruction": "rename it" }),
    )
    .await;

    assert_eq!(status, 502);
    assert_eq!(body["code"], "unavailable");
}
