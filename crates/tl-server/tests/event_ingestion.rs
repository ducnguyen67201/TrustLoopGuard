//! Direct `GuardEvent` ingestion at `/v1/events`.
//!
//! A full event (sources + provenance) can be submitted directly; its
//! evidence persists in traces; enabled policies and enforced checkers
//! compose into the returned decision. The retired `/v1/check` route is
//! intentionally absent.

use std::{collections::HashMap, sync::Arc, time::Duration};

use async_trait::async_trait;
use axum::{
    body::Body,
    http::{header, Request, StatusCode},
};
use http_body_util::BodyExt;
use tl_core::{AuthorizationDecision, AuthorizationEffect, DEFAULT_WORKSPACE_ID};
use tl_engine::Engine;
use tl_llm::{
    JsonSchema, JudgeKind, LlmClient, LlmError, LlmOutput, LlmRouter, ProviderTarget,
    ResolvedRoute, TokenBudget,
};
use tl_policy::load_str;
use tl_server::{memory_app_state, router, AppState};
use tower::ServiceExt;
use uuid::Uuid;

const DEFAULT_EVENT_ALLOW_REASON: &str = "current policy and authority permit the subject";

async fn read_body(resp: axum::response::Response) -> serde_json::Value {
    let bytes = resp.into_body().collect().await.unwrap().to_bytes();
    serde_json::from_slice(&bytes).unwrap()
}

fn json_request(
    method: &str,
    uri: &str,
    workspace_id: Option<&str>,
) -> axum::http::request::Builder {
    let mut builder = Request::builder()
        .method(method)
        .uri(uri)
        .header(header::CONTENT_TYPE, "application/json");
    if let Some(ws) = workspace_id {
        builder = builder.header("x-tlg-workspace-id", ws);
    }
    builder
}

fn submit_request(body: &serde_json::Value, workspace_id: Option<&str>) -> Request<Body> {
    json_request("POST", "/v1/events", workspace_id)
        .body(Body::from(body.to_string()))
        .unwrap()
}

/// `tool.call.proposed` for send_email: recipient sourced from the web,
/// body from both the user and the web — the canonical phase 2-3 fixture.
fn send_email_event() -> serde_json::Value {
    serde_json::json!({
        "kind": "tool.call.proposed",
        "principal": {
            "workspace_id": "ws_claimed",
            "environment_id": "env_claimed",
            "agent_id": "agent-1"
        },
        "action": {
            "invocation_id": Uuid::new_v4().to_string(),
            "operation": "send_email",
            "parameters": { "recipient": "a@b.c", "body": "hi" },
            "side_effect": "external_communication",
            "tool_identity": {
                "server_id": "mail",
                "tool_name": "send_email",
                "schema_hash": "sha256:v1:test-schema"
            }
        },
        "sources": [
            { "id": "src.user", "origin": "user", "labels": {} },
            { "id": "src.web", "origin": "web", "labels": {}, "kind": "web_page" }
        ],
        "provenance": {
            "recipient": ["src.web"],
            "body": ["src.user", "src.web"]
        }
    })
}

fn app() -> axum::Router {
    router(memory_app_state(Arc::new(Engine::empty())), None, [0u8; 32])
}

enum CannedLlmResponse {
    Ok(serde_json::Value),
    Error,
}

struct CannedLlmClient {
    response: CannedLlmResponse,
}

#[async_trait]
impl LlmClient for CannedLlmClient {
    async fn complete(
        &self,
        _model: &str,
        _prompt: &str,
        _schema: &JsonSchema,
        _deadline: Duration,
    ) -> Result<LlmOutput, LlmError> {
        match &self.response {
            CannedLlmResponse::Ok(json) => Ok(LlmOutput {
                json: json.clone(),
                prompt_tokens: 8,
                completion_tokens: 4,
            }),
            CannedLlmResponse::Error => Err(LlmError::Timeout(Duration::from_millis(1))),
        }
    }
}

fn semantic_router(response: CannedLlmResponse) -> Arc<LlmRouter> {
    let mut providers: HashMap<String, Arc<dyn LlmClient>> = HashMap::new();
    providers.insert("test".into(), Arc::new(CannedLlmClient { response }));
    let mut routes = HashMap::new();
    routes.insert(
        JudgeKind::SemanticPolicy,
        ResolvedRoute {
            primary: ProviderTarget {
                provider: "test".into(),
                model: "semantic".into(),
                deadline_ms: 1_000,
            },
            fallback: None,
        },
    );
    Arc::new(LlmRouter::new(
        providers,
        routes,
        Arc::new(TokenBudget::new(0)),
    ))
}

fn state_with_policies(policies: Vec<tl_policy::Policy>) -> AppState {
    memory_app_state(Arc::new(Engine::new(policies)))
}

fn output_event_body(text: &str) -> serde_json::Value {
    serde_json::json!({
        "kind": "output.proposed",
        "principal": {
            "workspace_id": "ws_claimed",
            "environment_id": "env_claimed",
            "agent_id": "agent-1"
        },
        "action": {
            "operation": "output",
            "parameters": { "text": text },
            "side_effect": "none"
        },
        "sources": [
            { "id": "input", "origin": "user", "labels": {} }
        ],
        "provenance": {
            "text": ["input"]
        },
        "context": {
            "channel": "chat",
            "domain": "customer_support"
        }
    })
}

#[tokio::test]
async fn legacy_check_route_is_removed() {
    let resp = app()
        .oneshot(
            json_request("POST", "/v1/check", None)
                .body(Body::from(legacy_check_body().to_string()))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(resp.status(), StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn submit_event_returns_default_allow_when_nothing_matches() {
    let app = app();

    let resp = app
        .oneshot(submit_request(&send_email_event(), None))
        .await
        .unwrap();
    let status = resp.status();
    let body = read_body(resp).await;
    assert_eq!(status, StatusCode::OK, "unexpected response: {body}");

    let decision: AuthorizationDecision = serde_json::from_value(body).unwrap();
    assert_eq!(decision.effect, AuthorizationEffect::Permit);
    assert_eq!(decision.reason, DEFAULT_EVENT_ALLOW_REASON);
    assert!(!decision.trace_id.is_empty());
    assert!(decision.findings.is_empty());
}

#[tokio::test]
async fn direct_event_with_run_updates_run_stats() {
    let app = app();
    let run_resp = app
        .clone()
        .oneshot(
            json_request("POST", "/v1/runs", Some(DEFAULT_WORKSPACE_ID))
                .body(Body::from(
                    serde_json::json!({
                        "agent_id": "agent-1",
                        "kind": "chat_session",
                        "metadata": {}
                    })
                    .to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(run_resp.status(), StatusCode::CREATED);
    let run = read_body(run_resp).await;
    let run_id = run["id"].as_str().unwrap();

    let mut body = send_email_event();
    body["principal"]["run_id"] = serde_json::json!(run_id);
    let resp = app
        .clone()
        .oneshot(submit_request(&body, Some(DEFAULT_WORKSPACE_ID)))
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);

    let run_resp = app
        .oneshot(
            json_request(
                "GET",
                &format!("/v1/runs/{run_id}"),
                Some(DEFAULT_WORKSPACE_ID),
            )
            .body(Body::empty())
            .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(run_resp.status(), StatusCode::OK);
    let run = read_body(run_resp).await;
    assert_eq!(run["run"]["trace_count"], 1);
}

#[tokio::test]
async fn direct_event_cannot_spoof_gateway_to_skip_run_stats() {
    let app = app();
    let run_resp = app
        .clone()
        .oneshot(
            json_request("POST", "/v1/runs", Some(DEFAULT_WORKSPACE_ID))
                .body(Body::from(
                    serde_json::json!({
                        "agent_id": "agent-1",
                        "kind": "chat_session",
                        "metadata": {}
                    })
                    .to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(run_resp.status(), StatusCode::CREATED);
    let run = read_body(run_resp).await;
    let run_id = run["id"].as_str().unwrap();

    let mut body = send_email_event();
    body["principal"]["run_id"] = serde_json::json!(run_id);
    body["context"] = serde_json::json!({ "integration_mode": "gateway" });
    let resp = app
        .clone()
        .oneshot(submit_request(&body, Some(DEFAULT_WORKSPACE_ID)))
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);

    let run_resp = app
        .oneshot(
            json_request(
                "GET",
                &format!("/v1/runs/{run_id}"),
                Some(DEFAULT_WORKSPACE_ID),
            )
            .body(Body::empty())
            .unwrap(),
        )
        .await
        .unwrap();
    let run = read_body(run_resp).await;
    assert_eq!(run["run"]["trace_count"], 1);
}

#[tokio::test]
async fn direct_event_rejects_run_event_from_another_run() {
    let app = app();
    let first_run_resp = app
        .clone()
        .oneshot(
            json_request("POST", "/v1/runs", Some(DEFAULT_WORKSPACE_ID))
                .body(Body::from(
                    serde_json::json!({
                        "agent_id": "agent-1",
                        "kind": "chat_session",
                        "metadata": {}
                    })
                    .to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(first_run_resp.status(), StatusCode::CREATED);
    let first_run = read_body(first_run_resp).await;
    let first_run_id = first_run["id"].as_str().unwrap();
    let event_resp = app
        .clone()
        .oneshot(
            json_request(
                "POST",
                &format!("/v1/runs/{first_run_id}/events"),
                Some(DEFAULT_WORKSPACE_ID),
            )
            .body(Body::from(
                serde_json::json!({
                    "kind": "user_turn",
                    "metadata": {}
                })
                .to_string(),
            ))
            .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(event_resp.status(), StatusCode::CREATED);
    let run_event = read_body(event_resp).await;
    let run_event_id = run_event["id"].as_str().unwrap();

    let second_run_resp = app
        .clone()
        .oneshot(
            json_request("POST", "/v1/runs", Some(DEFAULT_WORKSPACE_ID))
                .body(Body::from(
                    serde_json::json!({
                        "agent_id": "agent-1",
                        "kind": "chat_session",
                        "metadata": {}
                    })
                    .to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(second_run_resp.status(), StatusCode::CREATED);
    let second_run = read_body(second_run_resp).await;
    let second_run_id = second_run["id"].as_str().unwrap();

    let mut body = send_email_event();
    body["principal"]["run_id"] = serde_json::json!(second_run_id);
    body["principal"]["run_event_id"] = serde_json::json!(run_event_id);
    let resp = app
        .oneshot(submit_request(&body, Some(DEFAULT_WORKSPACE_ID)))
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn output_event_evaluates_enabled_content_policies() {
    let policy = load_str(
        r#"
id: refund-guarantee
when:
  channels: [chat]
  domains: [customer_support]
match:
  literal: guaranteed refund
action: deny
severity: high
"#,
    )
    .unwrap();
    let state = state_with_policies(vec![policy]);
    let app = router(state, None, [0u8; 32]);
    let body = output_event_body("we can offer a guaranteed refund today");

    let resp = app
        .oneshot(submit_request(&body, Some(DEFAULT_WORKSPACE_ID)))
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);

    let decision: AuthorizationDecision = serde_json::from_value(read_body(resp).await).unwrap();
    assert_eq!(decision.effect, AuthorizationEffect::Deny);
    assert_eq!(decision.findings.len(), 1);
    assert_eq!(
        decision.findings[0].policy_id.as_deref(),
        Some("refund-guarantee")
    );
}

#[tokio::test]
async fn output_event_evaluates_enabled_semantic_policy_with_llm_judge() {
    let policy = load_str(
        r#"
id: respectful-tone
when:
  channels: [chat]
  domains: [customer_support]
match:
  semantic: "the agent insults or demeans the user"
action: deny
severity: high
"#,
    )
    .unwrap();
    let mut state = state_with_policies(vec![policy]);
    state.handler_ctx.llm = semantic_router(CannedLlmResponse::Ok(serde_json::json!({
        "decisions": [{
            "policy_id": "respectful-tone",
            "matched": true,
            "confidence": 0.94,
            "reason": "direct insult",
            "evidence": ["you are dumb"]
        }]
    })));
    state
        .llm_pricing_store
        .upsert_price(
            DEFAULT_WORKSPACE_ID,
            "semantic",
            10,
            20,
            100_000_000,
            200_000_000,
        )
        .await
        .unwrap();
    let app = router(state, None, [0u8; 32]);

    let resp = app
        .clone()
        .oneshot(submit_request(
            &output_event_body("you are dumb"),
            Some(DEFAULT_WORKSPACE_ID),
        ))
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);

    let decision: AuthorizationDecision = serde_json::from_value(read_body(resp).await).unwrap();
    assert_eq!(decision.effect, AuthorizationEffect::Deny);
    assert_eq!(decision.findings.len(), 1);
    assert_eq!(
        decision.findings[0].policy_id.as_deref(),
        Some("respectful-tone")
    );
    assert!(decision.findings[0].reason.contains("confidence=0.94"));

    let usage = app
        .oneshot(
            json_request(
                "GET",
                "/v1/llm-usage?kind=guardrail",
                Some(DEFAULT_WORKSPACE_ID),
            )
            .body(Body::empty())
            .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(usage.status(), StatusCode::OK);
    let body = read_body(usage).await;
    assert_eq!(body["events"][0]["kind"], "guardrail");
    assert_eq!(body["events"][0]["prompt_tokens"], 8);
    assert_eq!(body["events"][0]["completion_tokens"], 4);
}

#[tokio::test]
async fn semantic_policy_without_llm_route_preserves_allow() {
    let policy = load_str(
        r#"
id: respectful-tone
match:
  semantic: "the agent insults or demeans the user"
action: deny
severity: high
"#,
    )
    .unwrap();
    let state = state_with_policies(vec![policy]);
    let app = router(state, None, [0u8; 32]);

    let resp = app
        .oneshot(submit_request(
            &output_event_body("you are dumb"),
            Some(DEFAULT_WORKSPACE_ID),
        ))
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);

    let decision: AuthorizationDecision = serde_json::from_value(read_body(resp).await).unwrap();
    assert_eq!(decision.effect, AuthorizationEffect::Permit);
    assert!(decision.findings.is_empty());
}

#[tokio::test]
async fn semantic_judge_failure_escalates_high_severity_policy() {
    let policy = load_str(
        r#"
id: legal-advice
match:
  semantic: "the agent gives legal advice"
action: deny
severity: critical
"#,
    )
    .unwrap();
    let mut state = state_with_policies(vec![policy]);
    state.handler_ctx.llm = semantic_router(CannedLlmResponse::Error);
    let app = router(state, None, [0u8; 32]);

    let resp = app
        .oneshot(submit_request(
            &output_event_body("you should sue them"),
            Some(DEFAULT_WORKSPACE_ID),
        ))
        .await
        .unwrap();
    let status = resp.status();
    let body = read_body(resp).await;
    assert_eq!(status, StatusCode::OK, "unexpected response: {body}");

    let decision: AuthorizationDecision = serde_json::from_value(body).unwrap();
    assert_eq!(decision.effect, AuthorizationEffect::Defer);
    assert_eq!(
        decision.findings[0].policy_id.as_deref(),
        Some("legal-advice")
    );
    assert!(decision.reason.contains("judge unavailable"));
}

#[tokio::test]
async fn malformed_json_is_rejected() {
    let app = app();

    let resp = app
        .oneshot(
            json_request("POST", "/v1/events", None)
                .body(Body::from("{ not json"))
                .unwrap(),
        )
        .await
        .unwrap();
    assert!(resp.status().is_client_error());
}

#[tokio::test]
async fn validation_rejections() {
    let cases: Vec<(serde_json::Value, &str)> = vec![
        (
            {
                let mut e = send_email_event();
                e["sources"] = serde_json::Value::Array(
                    (0..65)
                        .map(|i| serde_json::json!({ "id": format!("s{i}"), "origin": "web" }))
                        .collect(),
                );
                e
            },
            "sources",
        ),
        (
            {
                let mut e = send_email_event();
                e["action"]["operation"] = serde_json::json!("");
                e
            },
            "operation",
        ),
        (
            {
                let mut e = send_email_event();
                e["sources"] = serde_json::json!([
                    { "id": "dup", "origin": "web" },
                    { "id": "dup", "origin": "user" }
                ]);
                e
            },
            "duplicate source id at index 1",
        ),
        (
            {
                let mut e = send_email_event();
                e["action"]["parameters"] = serde_json::json!({ "blob": "x".repeat(70_000) });
                e
            },
            "parameters",
        ),
    ];

    for (body, expected_fragment) in cases {
        let resp = app().oneshot(submit_request(&body, None)).await.unwrap();
        assert_eq!(
            resp.status(),
            StatusCode::UNPROCESSABLE_ENTITY,
            "case `{expected_fragment}`"
        );
        let value = read_body(resp).await;
        assert!(
            value["message"]
                .as_str()
                .unwrap()
                .contains(expected_fragment),
            "case `{expected_fragment}`: got {}",
            value["message"]
        );
    }
}

#[tokio::test]
async fn run_id_must_be_uuid_and_exist() {
    let app = app();

    let mut body = send_email_event();
    body["principal"]["run_id"] = serde_json::json!("not-a-uuid");
    let resp = app
        .clone()
        .oneshot(submit_request(&body, None))
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::BAD_REQUEST);

    let mut body = send_email_event();
    body["principal"]["run_id"] = serde_json::json!("018f9999-9999-7999-8999-999999999999");
    let resp = app
        .clone()
        .oneshot(submit_request(&body, None))
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::NOT_FOUND);

    // run_event_id requires run_id.
    let mut body = send_email_event();
    body["principal"]["run_event_id"] = serde_json::json!("018f9999-9999-7999-8999-999999999999");
    let resp = app
        .clone()
        .oneshot(submit_request(&body, None))
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::BAD_REQUEST);

    // run_event_id must itself be a UUID.
    let mut body = send_email_event();
    body["principal"]["run_event_id"] = serde_json::json!("not-a-uuid");
    let resp = app.oneshot(submit_request(&body, None)).await.unwrap();
    assert_eq!(resp.status(), StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn oversized_body_rejected_before_deserialization() {
    let app = app();

    // 600 KiB body exceeds the route's 512 KiB cap.
    let mut body = send_email_event();
    body["context"] = serde_json::json!({ "blob": "x".repeat(600 * 1024) });
    let resp = app.oneshot(submit_request(&body, None)).await.unwrap();
    assert_eq!(resp.status(), StatusCode::PAYLOAD_TOO_LARGE);
}

#[tokio::test]
async fn non_raw_allowed_workspace_rejected() {
    use async_trait::async_trait;
    use tl_server::dashboard_admin::DashboardAdminStoreError;
    use tl_server::SettingsStore;

    struct RedactedOnlySettings;

    #[async_trait]
    impl SettingsStore for RedactedOnlySettings {
        async fn get(
            &self,
            _workspace_id: &str,
        ) -> Result<tl_core::WorkspaceSettings, DashboardAdminStoreError> {
            Ok(tl_core::WorkspaceSettings {
                default_action: "block".into(),
                escalation_webhook_url: None,
                telemetry_enabled: false,
                retention_days: "30".into(),
                data_handling_mode: tl_core::DataHandlingMode::RedactedOnly,
                flow_checker_mode: tl_core::EnforcementMode::Off,
                memory_checker_mode: tl_core::EnforcementMode::Off,
                param_checker_mode: tl_core::EnforcementMode::Off,
                approval_checker_mode: tl_core::EnforcementMode::Off,
                config: serde_json::Value::Null,
                updated_at: None,
            })
        }
    }

    let mut state = memory_app_state(Arc::new(Engine::empty()));
    state.settings_store = Arc::new(RedactedOnlySettings);
    let app = router(state, None, [0u8; 32]);

    let resp = app
        .oneshot(submit_request(&send_email_event(), None))
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::BAD_REQUEST);

    let value = read_body(resp).await;
    assert!(value["message"].as_str().unwrap().contains("raw_allowed"));
}

fn legacy_check_body() -> serde_json::Value {
    serde_json::json!({
        "agent_id": "anon",
        "channel": "chat",
        "input": "hi",
        "proposed_output": "hello there"
    })
}

#[cfg(feature = "postgres")]
mod trace_evidence {
    use super::*;
    use tl_core::{LabelBasis, LabelPolicyStatus, SideEffectClass, ToolResolution, Trust};
    use tl_server::traces::ChannelTraceStore;

    fn metadata_body() -> serde_json::Value {
        serde_json::json!({
            "tool": "send_email",
            "side_effect": "external_communication",
            "reversible": false,
            "params": [{
                "path": "recipient",
                "role": "authority_bearing",
                "allowed_sources": [{ "origin": "user" }]
            }]
        })
    }

    /// The headline end-to-end flow: register a tool and a label policy
    /// through their APIs, submit a full event, and observe phase 2 + 3
    /// evidence on the enqueued trace.
    #[tokio::test]
    async fn full_evidence_flows_to_trace() {
        let mut state = memory_app_state(Arc::new(Engine::empty()));
        let (capture, mut rx) = ChannelTraceStore::channel(8);
        state.trace_store = capture;
        let owner_id = uuid::Uuid::new_v4();
        let workspace = state
            .team_store
            .create_workspace(owner_id, "Trace Evidence")
            .await
            .unwrap();
        let app = router(state, None, [0u8; 32]);

        let resp = app
            .clone()
            .oneshot(
                json_request("POST", "/v1/tool-metadata", Some(&workspace.id))
                    .header("x-tlg-user-id", owner_id.to_string())
                    .body(Body::from(metadata_body().to_string()))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(resp.status(), StatusCode::CREATED);

        let resp = app
            .clone()
            .oneshot(
                json_request("POST", "/v1/label-policies", Some(&workspace.id))
                    .body(Body::from(
                        serde_json::json!({ "origin": "web", "confidentiality": "private" })
                            .to_string(),
                    ))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(resp.status(), StatusCode::CREATED);

        let resp = app
            .oneshot(submit_request(&send_email_event(), Some(&workspace.id)))
            .await
            .unwrap();
        assert_eq!(resp.status(), StatusCode::OK);

        let trace = rx.recv().await.expect("trace enqueued");
        assert_eq!(trace.domain, "event");
        assert!(trace.run_id.is_none());

        let event = trace.event.expect("event evidence attached");
        // Phase 2: registry side effect is authoritative.
        assert!(matches!(
            event.resolution,
            Some(ToolResolution::Resolved { .. })
        ));
        assert_eq!(
            event.action.side_effect,
            Some(SideEffectClass::ExternalCommunication)
        );
        // Phase 3: labels resolved with basis; derived over provenance.
        let resolution = event.label_resolution.expect("label evidence");
        assert_eq!(resolution.policy_status, LabelPolicyStatus::Applied);
        let web = resolution
            .sources
            .iter()
            .find(|s| s.source_id == "src.web")
            .expect("web evidence");
        assert_eq!(web.basis.confidentiality, LabelBasis::WorkspaceOverride);
        assert_eq!(resolution.derived["recipient"].trust, Trust::Untrusted);
        assert_eq!(resolution.derived["body"].trust, Trust::Untrusted);
    }

    /// The caller-claimed principal identity never survives ingestion.
    #[tokio::test]
    async fn workspace_identity_cannot_be_spoofed() {
        let mut state = memory_app_state(Arc::new(Engine::empty()));
        let (capture, mut rx) = ChannelTraceStore::channel(8);
        state.trace_store = capture;
        let app = router(state, None, [0u8; 32]);

        // Body claims ws_claimed; the header says ws_a.
        let resp = app
            .oneshot(submit_request(&send_email_event(), Some("ws_a")))
            .await
            .unwrap();
        assert_eq!(resp.status(), StatusCode::OK);

        let trace = rx.recv().await.expect("trace enqueued");
        assert_eq!(trace.workspace_id, "ws_a");
        let event = trace.event.expect("event evidence attached");
        assert_eq!(event.principal.workspace_id, "ws_a");
        assert_ne!(event.principal.environment_id, "env_claimed");
    }
}
