//! Source label policies: CRUD surface + event label-resolution evidence
//! at `/v1/events`.
//!
//! Phase 3 of the event engine: labels are resolved and propagated into
//! trace evidence without changing any decisions.

use std::sync::Arc;

use axum::{
    body::Body,
    http::{header, Request, StatusCode},
};
use http_body_util::BodyExt;
use tl_core::{
    AuthorizationDecision, AuthorizationEffect, SourceLabelPolicyEntry,
    SourceLabelPolicyListResponse,
};
use tl_engine::Engine;
use tl_server::{memory_app_state, router};
use tower::ServiceExt;

async fn read_body(resp: axum::response::Response) -> serde_json::Value {
    let bytes = resp.into_body().collect().await.unwrap().to_bytes();
    serde_json::from_slice(&bytes).unwrap()
}

fn json_request(
    method: &str,
    uri: &str,
    workspace_id: Option<&str>,
) -> axum::http::request::Builder {
    let builder = Request::builder()
        .method(method)
        .uri(uri)
        .header(header::CONTENT_TYPE, "application/json");
    builder.header("x-tlg-workspace-id", workspace_id.unwrap_or("ws"))
}

fn policy_body(origin: &str) -> serde_json::Value {
    serde_json::json!({
        "origin": origin,
        "trust": "untrusted",
        "confidentiality": "private"
    })
}

fn upsert_request(body: &serde_json::Value, workspace_id: Option<&str>) -> Request<Body> {
    json_request("POST", "/v1/label-policies", workspace_id)
        .body(Body::from(body.to_string()))
        .unwrap()
}

fn app() -> axum::Router {
    router(memory_app_state(Arc::new(Engine::empty())), None, [0u8; 32])
}

#[tokio::test]
async fn upsert_then_get_round_trips() {
    let app = app();

    let resp = app
        .clone()
        .oneshot(upsert_request(&policy_body("web"), None))
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::CREATED);

    let resp = app
        .oneshot(
            json_request("GET", "/v1/label-policies/web", None)
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);

    let entry: SourceLabelPolicyEntry = serde_json::from_value(read_body(resp).await).unwrap();
    assert_eq!(entry.policy.origin, tl_core::Origin::Web);
    assert_eq!(entry.policy.trust, Some(tl_core::Trust::Untrusted));
    assert_eq!(
        entry.policy.confidentiality,
        Some(tl_core::Confidentiality::Private)
    );
    assert!(entry.policy.integrity.is_none());
    assert!(entry.enabled, "enabled defaults to true");
}

#[tokio::test]
async fn list_scopes_to_workspace() {
    let app = app();

    for (origin, ws) in [("web", "ws_a"), ("email", "ws_a"), ("web", "ws_b")] {
        let resp = app
            .clone()
            .oneshot(upsert_request(&policy_body(origin), Some(ws)))
            .await
            .unwrap();
        assert_eq!(resp.status(), StatusCode::CREATED);
    }

    let resp = app
        .oneshot(
            json_request("GET", "/v1/label-policies", Some("ws_a"))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);

    let list: SourceLabelPolicyListResponse =
        serde_json::from_value(read_body(resp).await).unwrap();
    let origins: Vec<tl_core::Origin> = list.policies.iter().map(|p| p.policy.origin).collect();
    assert_eq!(origins, vec![tl_core::Origin::Email, tl_core::Origin::Web]);
}

#[tokio::test]
async fn upsert_rejects_empty_override() {
    let app = app();

    let resp = app
        .oneshot(upsert_request(
            &serde_json::json!({ "origin": "web" }),
            None,
        ))
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::UNPROCESSABLE_ENTITY);

    let body = read_body(resp).await;
    assert!(body["message"]
        .as_str()
        .unwrap()
        .contains("at least one of trust, confidentiality, integrity"));
}

#[tokio::test]
async fn invalid_origin_path_rejected() {
    let app = app();

    let resp = app
        .oneshot(
            json_request("GET", "/v1/label-policies/banana", None)
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::UNPROCESSABLE_ENTITY);

    let body = read_body(resp).await;
    assert!(body["message"].as_str().unwrap().contains("unknown origin"));
}

#[tokio::test]
async fn delete_then_get_returns_not_found() {
    let app = app();

    let resp = app
        .clone()
        .oneshot(upsert_request(&policy_body("web"), None))
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::CREATED);

    let resp = app
        .clone()
        .oneshot(
            json_request("DELETE", "/v1/label-policies/web", None)
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::NO_CONTENT);

    let resp = app
        .oneshot(
            json_request("GET", "/v1/label-policies/web", None)
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn disabled_policy_listed_but_not_resolved() {
    let app = app();

    let mut body = policy_body("web");
    body["enabled"] = serde_json::json!(false);
    let resp = app
        .clone()
        .oneshot(upsert_request(&body, None))
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::CREATED);

    // The control plane still sees the disabled row.
    let resp = app
        .oneshot(
            json_request("GET", "/v1/label-policies", None)
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    let list: SourceLabelPolicyListResponse =
        serde_json::from_value(read_body(resp).await).unwrap();
    assert_eq!(list.policies.len(), 1);
    assert!(!list.policies[0].enabled);
    // Runtime resolution skipping the disabled row is asserted on the
    // enqueued trace evidence in `trace_evidence` below.
}

fn event_request(body: &serde_json::Value) -> Request<Body> {
    Request::builder()
        .method("POST")
        .uri("/v1/events")
        .header(header::CONTENT_TYPE, "application/json")
        .header("x-tlg-workspace-id", "ws")
        .body(Body::from(body.to_string()))
        .unwrap()
}

fn output_event_body() -> serde_json::Value {
    serde_json::json!({
        "kind": "output.proposed",
        "principal": {
            "workspace_id": "default",
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

/// Label policy configuration resolves evidence; it must not change an
/// event decision when no checker or content policy matches.
#[tokio::test]
async fn event_path_decision_unchanged_with_label_policies_configured() {
    let baseline_app = app();
    let resp = baseline_app
        .oneshot(event_request(&output_event_body()))
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let baseline = read_body(resp).await;

    let configured_app = app();
    let resp = configured_app
        .clone()
        .oneshot(upsert_request(
            &serde_json::json!({ "origin": "user", "trust": "untrusted" }),
            None,
        ))
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::CREATED);

    let resp = configured_app
        .oneshot(event_request(&output_event_body()))
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let mut configured = read_body(resp).await;

    // Trace ids and measured latency differ per request; everything
    // else must be identical.
    configured["trace_id"] = baseline["trace_id"].clone();
    configured["latency_ms"] = baseline["latency_ms"].clone();
    configured["receipt_id"] = baseline["receipt_id"].clone();
    assert_eq!(configured, baseline);

    let decision: AuthorizationDecision = serde_json::from_value(configured).unwrap();
    assert_eq!(decision.effect, AuthorizationEffect::Permit);
}

#[cfg(feature = "postgres")]
mod trace_evidence {
    use super::*;
    use tl_core::{Confidentiality, LabelBasis, LabelPolicyStatus, Trust};
    use tl_server::traces::ChannelTraceStore;

    /// Full end-to-end flow: configure a label policy through the API,
    /// submit an event, and observe Phase 2 + Phase 3 evidence on the
    /// enqueued trace.
    #[tokio::test]
    async fn trace_write_carries_label_resolution_evidence() {
        let mut state = memory_app_state(Arc::new(Engine::empty()));
        let (capture, mut rx) = ChannelTraceStore::channel(8);
        state.trace_store = capture;
        let app = router(state, None, [0u8; 32]);

        // The event carries a user-origin `input` source; override the
        // user origin so the workspace policy is visible in the resolved
        // evidence.
        let resp = app
            .clone()
            .oneshot(upsert_request(
                &serde_json::json!({ "origin": "user", "confidentiality": "secret" }),
                None,
            ))
            .await
            .unwrap();
        assert_eq!(resp.status(), StatusCode::CREATED);

        let resp = app
            .oneshot(event_request(&output_event_body()))
            .await
            .unwrap();
        assert_eq!(resp.status(), StatusCode::OK);

        let trace = rx.recv().await.expect("trace enqueued");
        let event = trace.event.expect("event evidence attached");

        // Phase 2 evidence: the `output` operation is unregistered.
        assert_eq!(
            event.resolution,
            Some(tl_core::ToolResolution::Unregistered)
        );

        // Phase 3 evidence: resolved source labels + basis + status.
        let resolution = event.label_resolution.expect("label evidence");
        assert_eq!(resolution.policy_status, LabelPolicyStatus::Applied);
        let source = &resolution.sources[0];
        assert_eq!(source.source_id, "input");
        assert_eq!(source.labels.trust, Trust::Trusted);
        assert_eq!(source.basis.trust, LabelBasis::OriginDefault);
        assert_eq!(source.labels.confidentiality, Confidentiality::Secret);
        assert_eq!(source.basis.confidentiality, LabelBasis::WorkspaceOverride);
        // Resolved labels are written back onto the event source.
        assert_eq!(
            event.sources[0].labels.confidentiality,
            Confidentiality::Secret
        );
    }

    /// Disabled policies stay manageable but are skipped at runtime.
    #[tokio::test]
    async fn disabled_policy_not_applied_at_runtime() {
        let mut state = memory_app_state(Arc::new(Engine::empty()));
        let (capture, mut rx) = ChannelTraceStore::channel(8);
        state.trace_store = capture;
        let app = router(state, None, [0u8; 32]);

        let resp = app
            .clone()
            .oneshot(upsert_request(
                &serde_json::json!({
                    "origin": "user",
                    "confidentiality": "secret",
                    "enabled": false
                }),
                None,
            ))
            .await
            .unwrap();
        assert_eq!(resp.status(), StatusCode::CREATED);

        let resp = app
            .oneshot(event_request(&output_event_body()))
            .await
            .unwrap();
        assert_eq!(resp.status(), StatusCode::OK);

        let trace = rx.recv().await.expect("trace enqueued");
        let event = trace.event.expect("event evidence attached");
        let resolution = event.label_resolution.expect("label evidence");

        // No enabled rows -> defaults applied, status not_configured.
        assert_eq!(resolution.policy_status, LabelPolicyStatus::NotConfigured);
        let source = &resolution.sources[0];
        assert_eq!(source.labels.confidentiality, Confidentiality::Private);
        assert_eq!(source.basis.confidentiality, LabelBasis::OriginDefault);
    }
}
