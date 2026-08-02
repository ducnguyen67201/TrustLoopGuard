//! Tool metadata registry: CRUD surface + action-resolution evidence at
//! `/v1/events`.
//!
//! Phase 2 of the event engine: workspace-scoped tool metadata can be
//! managed, cached, and attached to events without changing decisions.

use std::sync::Arc;

use axum::{
    body::Body,
    http::{header, Request, StatusCode},
};
use http_body_util::BodyExt;
use tl_core::{
    AuthorizationDecision, AuthorizationEffect, ToolMetadataEntry, ToolMetadataListResponse,
};
use tl_engine::Engine;
use tl_server::{memory_app_state, router};
use tower::ServiceExt;
use uuid::Uuid;

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
        builder = builder.header("x-featherlane-ai-workspace-id", ws);
    }
    builder
}

fn metadata_body(tool: &str) -> serde_json::Value {
    serde_json::json!({
        "tool": tool,
        "side_effect": "external_communication",
        "reversible": false,
        "params": [{
            "path": "recipient",
            "role": "authority_bearing",
            "allowed_sources": [{ "origin": "user" }]
        }]
    })
}

fn upsert_request(body: &serde_json::Value, workspace_id: &str, owner_id: Uuid) -> Request<Body> {
    json_request("POST", "/v1/tool-metadata", Some(workspace_id))
        .header("x-featherlane-ai-user-id", owner_id.to_string())
        .body(Body::from(body.to_string()))
        .unwrap()
}

async fn app() -> (axum::Router, String, Uuid) {
    let state = memory_app_state(Arc::new(Engine::empty()));
    let owner_id = Uuid::new_v4();
    let workspace = state
        .team_store
        .create_workspace(owner_id, "Tool Metadata Tests")
        .await
        .unwrap();
    (router(state, None, [0u8; 32]), workspace.id, owner_id)
}

#[tokio::test]
async fn upsert_then_get_round_trips() {
    let (app, workspace_id, owner_id) = app().await;

    let resp = app
        .clone()
        .oneshot(upsert_request(
            &metadata_body("send_email"),
            &workspace_id,
            owner_id,
        ))
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::CREATED);

    let resp = app
        .oneshot(
            json_request("GET", "/v1/tool-metadata/send_email", Some(&workspace_id))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);

    let entry: ToolMetadataEntry = serde_json::from_value(read_body(resp).await).unwrap();
    assert_eq!(entry.metadata.tool, "send_email");
    assert_eq!(entry.metadata.params[0].path, "recipient");
    assert!(entry.enabled, "enabled defaults to true");
}

#[tokio::test]
async fn list_scopes_to_workspace() {
    let state = memory_app_state(Arc::new(Engine::empty()));
    let owner_id = Uuid::new_v4();
    let workspace_a = state
        .team_store
        .create_workspace(owner_id, "Tool Metadata A")
        .await
        .unwrap();
    let workspace_b = state
        .team_store
        .create_workspace(owner_id, "Tool Metadata B")
        .await
        .unwrap();
    let app = router(state, None, [0u8; 32]);

    for (tool, ws) in [
        ("send_email", workspace_a.id.as_str()),
        ("run_query", workspace_a.id.as_str()),
        ("send_email", workspace_b.id.as_str()),
    ] {
        let resp = app
            .clone()
            .oneshot(upsert_request(&metadata_body(tool), ws, owner_id))
            .await
            .unwrap();
        assert_eq!(resp.status(), StatusCode::CREATED);
    }

    let resp = app
        .oneshot(
            json_request("GET", "/v1/tool-metadata", Some(&workspace_a.id))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);

    let list: ToolMetadataListResponse = serde_json::from_value(read_body(resp).await).unwrap();
    let tools: Vec<&str> = list
        .tools
        .iter()
        .map(|t| t.metadata.tool.as_str())
        .collect();
    assert_eq!(tools, vec!["run_query", "send_email"]);
}

#[tokio::test]
async fn invalid_param_path_returns_422() {
    let (app, workspace_id, owner_id) = app().await;
    let mut body = metadata_body("send_email");
    body["params"][0]["path"] = serde_json::json!("");

    let resp = app
        .oneshot(upsert_request(&body, &workspace_id, owner_id))
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::UNPROCESSABLE_ENTITY);

    let error = read_body(resp).await;
    assert!(error["message"].as_str().unwrap().contains("param path"));
}

#[tokio::test]
async fn duplicate_param_path_returns_422() {
    let (app, workspace_id, owner_id) = app().await;
    let mut body = metadata_body("send_email");
    let param = body["params"][0].clone();
    body["params"].as_array_mut().unwrap().push(param);

    let resp = app
        .oneshot(upsert_request(&body, &workspace_id, owner_id))
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::UNPROCESSABLE_ENTITY);

    let error = read_body(resp).await;
    assert!(error["message"].as_str().unwrap().contains("duplicate"));
}

/// Serde rejects unknown enum variants at the extractor: axum maps
/// well-formed-but-invalid JSON to 422 (400 is reserved for syntax errors).
#[tokio::test]
async fn unknown_side_effect_is_rejected_at_parse() {
    let (app, workspace_id, owner_id) = app().await;
    let mut body = metadata_body("send_email");
    body["side_effect"] = serde_json::json!("nonsense");

    let resp = app
        .oneshot(upsert_request(&body, &workspace_id, owner_id))
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::UNPROCESSABLE_ENTITY);
}

#[tokio::test]
async fn delete_then_get_returns_404() {
    let (app, workspace_id, owner_id) = app().await;
    let resp = app
        .clone()
        .oneshot(upsert_request(
            &metadata_body("send_email"),
            &workspace_id,
            owner_id,
        ))
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::CREATED);

    let resp = app
        .clone()
        .oneshot(
            json_request(
                "DELETE",
                "/v1/tool-metadata/send_email",
                Some(&workspace_id),
            )
            .header("x-featherlane-ai-user-id", owner_id.to_string())
            .body(Body::empty())
            .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::NO_CONTENT);

    let resp = app
        .clone()
        .oneshot(
            json_request("GET", "/v1/tool-metadata/send_email", Some(&workspace_id))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::NOT_FOUND);

    let resp = app
        .oneshot(
            json_request(
                "DELETE",
                "/v1/tool-metadata/send_email",
                Some(&workspace_id),
            )
            .header("x-featherlane-ai-user-id", owner_id.to_string())
            .body(Body::empty())
            .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::NOT_FOUND);
}

/// Tool names commonly contain dots (`gmail.send`); they must route
/// as a single path segment.
#[tokio::test]
async fn dotted_tool_name_routes_in_path() {
    let (app, workspace_id, owner_id) = app().await;
    let resp = app
        .clone()
        .oneshot(upsert_request(
            &metadata_body("gmail.send"),
            &workspace_id,
            owner_id,
        ))
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::CREATED);

    let resp = app
        .oneshot(
            json_request("GET", "/v1/tool-metadata/gmail.send", Some(&workspace_id))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let entry: ToolMetadataEntry = serde_json::from_value(read_body(resp).await).unwrap();
    assert_eq!(entry.metadata.tool, "gmail.send");
}

#[tokio::test]
async fn get_is_isolated_by_workspace_header() {
    let state = memory_app_state(Arc::new(Engine::empty()));
    let owner_id = Uuid::new_v4();
    let workspace_a = state
        .team_store
        .create_workspace(owner_id, "Tool Metadata A")
        .await
        .unwrap();
    let workspace_b = state
        .team_store
        .create_workspace(owner_id, "Tool Metadata B")
        .await
        .unwrap();
    let app = router(state, None, [0u8; 32]);
    let resp = app
        .clone()
        .oneshot(upsert_request(
            &metadata_body("send_email"),
            &workspace_a.id,
            owner_id,
        ))
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::CREATED);

    let resp = app
        .oneshot(
            json_request("GET", "/v1/tool-metadata/send_email", Some(&workspace_b.id))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::NOT_FOUND);
}

/// Resolution evidence rides on the trace event; resolution alone does
/// not change the decision. Trace plumbing requires the
/// postgres feature (captured through the unified `TraceStore::record` seam).
#[cfg(feature = "postgres")]
mod resolution_evidence {
    use super::*;
    use tl_core::{SideEffectClass, ToolResolution};
    use tl_server::traces::ChannelTraceStore;

    fn event_request(body: &serde_json::Value, workspace_id: &str) -> Request<Body> {
        json_request("POST", "/v1/events", Some(workspace_id))
            .body(Body::from(body.to_string()))
            .unwrap()
    }

    fn output_event_body(workspace_id: &str) -> serde_json::Value {
        serde_json::json!({
            "kind": "output.proposed",
            "principal": {
                "workspace_id": workspace_id,
                "environment_id": "production",
                "agent_id": "anon"
            },
            "action": {
                "operation": "output",
                "parameters": { "text": "hello there" },
                "side_effect": "none"
            },
            "sources": [{ "id": "input", "origin": "user", "labels": {} }],
            "provenance": { "text": ["input"] },
            "context": { "channel": "chat", "domain": "customer_support" }
        })
    }

    /// The `output` operation has no registry entry by default: the trace
    /// event carries conservative `unregistered` evidence and the verdict
    /// is unchanged.
    #[tokio::test]
    async fn event_trace_carries_unregistered_resolution() {
        let mut state = memory_app_state(Arc::new(Engine::empty()));
        let owner_id = Uuid::new_v4();
        let workspace = state
            .team_store
            .create_workspace(owner_id, "Tool Metadata Trace")
            .await
            .unwrap();
        let (capture, mut rx) = ChannelTraceStore::channel(8);
        state.trace_store = capture;
        let app = router(state, None, [0u8; 32]);

        let resp = app
            .oneshot(event_request(
                &output_event_body(&workspace.id),
                &workspace.id,
            ))
            .await
            .unwrap();
        assert_eq!(resp.status(), StatusCode::OK);
        let decision: AuthorizationDecision =
            serde_json::from_value(read_body(resp).await).unwrap();
        assert_eq!(decision.effect, AuthorizationEffect::Permit);

        let trace = rx.recv().await.expect("trace enqueued");
        let event = trace.event.expect("event evidence attached");
        assert_eq!(event.resolution, Some(ToolResolution::Unregistered));
    }

    /// Registered metadata resolves through the shared memory store:
    /// the trace event carries the metadata and the registry-authoritative
    /// side effect, while the verdict stays Allow.
    #[tokio::test]
    async fn event_trace_carries_resolved_metadata() {
        let mut state = memory_app_state(Arc::new(Engine::empty()));
        let owner_id = Uuid::new_v4();
        let workspace = state
            .team_store
            .create_workspace(owner_id, "Tool Metadata Trace")
            .await
            .unwrap();
        let (capture, mut rx) = ChannelTraceStore::channel(8);
        state.trace_store = capture;
        let app = router(state.clone(), None, [0u8; 32]);

        let mut body = metadata_body("output");
        body["side_effect"] = serde_json::json!("read");
        let resp = app
            .clone()
            .oneshot(upsert_request(&body, &workspace.id, owner_id))
            .await
            .unwrap();
        assert_eq!(resp.status(), StatusCode::CREATED);

        let resp = app
            .oneshot(event_request(
                &output_event_body(&workspace.id),
                &workspace.id,
            ))
            .await
            .unwrap();
        assert_eq!(resp.status(), StatusCode::OK);
        let decision: AuthorizationDecision =
            serde_json::from_value(read_body(resp).await).unwrap();
        assert_eq!(decision.effect, AuthorizationEffect::Permit);

        let trace = rx.recv().await.expect("trace enqueued");
        let event = trace.event.expect("event evidence attached");
        assert_eq!(event.action.side_effect, Some(SideEffectClass::Read));
        match event.resolution {
            Some(ToolResolution::Resolved { metadata }) => {
                assert_eq!(metadata.tool, "output");
                assert_eq!(metadata.params[0].path, "recipient");
            }
            other => panic!("expected resolved metadata, got {other:?}"),
        }
    }

    /// Disabled registry rows resolve as unregistered at runtime.
    #[tokio::test]
    async fn disabled_tool_resolves_as_unregistered() {
        let mut state = memory_app_state(Arc::new(Engine::empty()));
        let owner_id = Uuid::new_v4();
        let workspace = state
            .team_store
            .create_workspace(owner_id, "Tool Metadata Trace")
            .await
            .unwrap();
        let (capture, mut rx) = ChannelTraceStore::channel(8);
        state.trace_store = capture;
        let app = router(state, None, [0u8; 32]);

        let mut body = metadata_body("output");
        body["enabled"] = serde_json::json!(false);
        let resp = app
            .clone()
            .oneshot(upsert_request(&body, &workspace.id, owner_id))
            .await
            .unwrap();
        assert_eq!(resp.status(), StatusCode::CREATED);

        let resp = app
            .oneshot(event_request(
                &output_event_body(&workspace.id),
                &workspace.id,
            ))
            .await
            .unwrap();
        assert_eq!(resp.status(), StatusCode::OK);

        let trace = rx.recv().await.expect("trace enqueued");
        let event = trace.event.expect("event evidence attached");
        assert_eq!(event.resolution, Some(ToolResolution::Unregistered));
    }
}
