//! Observe-only event collection at the `/v1/check` route.
//!
//! Phase 1 wires the event pipeline into the check path: raw inputs
//! normalize into `GuardEvent` trace evidence while legacy decision
//! behavior stays byte-identical and trace writes stay fire-and-forget.

use std::sync::Arc;

use axum::{
    body::Body,
    http::{header, Request, StatusCode},
};
use http_body_util::BodyExt;
use tl_core::{Decision, Verdict};
use tl_engine::Engine;
use tl_server::{memory_app_state, router};
use tower::ServiceExt;

async fn read_body(resp: axum::response::Response) -> serde_json::Value {
    let bytes = resp.into_body().collect().await.unwrap().to_bytes();
    serde_json::from_slice(&bytes).unwrap()
}

fn check_request(body: &serde_json::Value) -> Request<Body> {
    Request::builder()
        .method("POST")
        .uri("/v1/check")
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::from(body.to_string()))
        .unwrap()
}

fn legacy_body() -> serde_json::Value {
    serde_json::json!({
        "agent_id": "anon",
        "channel": "chat",
        "input": "hi",
        "proposed_output": "hello there"
    })
}

/// Route-level golden replay: the event pipeline must not change the
/// legacy response shape — no event evidence keys, same allow verdict.
#[tokio::test]
async fn legacy_check_response_unchanged_by_event_pipeline() {
    let state = memory_app_state(Arc::new(Engine::empty()));
    let app = router(state, None, [0u8; 32]);

    let resp = app.oneshot(check_request(&legacy_body())).await.unwrap();
    assert_eq!(resp.status(), StatusCode::OK);

    let value = read_body(resp).await;
    for key in [
        "event",
        "violated_rule",
        "remediation",
        "source_chain",
        "risk_source",
        "failure_mode",
        "harm_class",
        "constraints",
    ] {
        assert!(
            value.get(key).is_none(),
            "{key} should not appear in the legacy response"
        );
    }

    let decision: Decision = serde_json::from_value(value).unwrap();
    assert_eq!(decision.verdict, Verdict::Allow);
}

/// Gateway-context checks gain low-fidelity trace evidence but the
/// decision itself is untouched — observe-only means no enforcement.
#[tokio::test]
async fn check_with_gateway_context_returns_same_decision() {
    let state = memory_app_state(Arc::new(Engine::empty()));
    let app = router(state, None, [0u8; 32]);

    let body = serde_json::json!({
        "agent_id": "anon",
        "channel": "chat",
        "input": "hi",
        "proposed_output": "hello there",
        "context": { "integration_mode": "gateway" }
    });
    let resp = app.oneshot(check_request(&body)).await.unwrap();
    assert_eq!(resp.status(), StatusCode::OK);

    let value = read_body(resp).await;
    assert!(value.get("event").is_none(), "evidence is trace-side only");

    let decision: Decision = serde_json::from_value(value).unwrap();
    assert_eq!(decision.verdict, Verdict::Allow);
}

#[cfg(feature = "postgres")]
mod trace_evidence {
    use super::*;
    use tl_core::EventKind;
    use tl_storage::TraceWrite;
    use tokio::sync::mpsc;

    fn dummy_trace_write() -> TraceWrite {
        TraceWrite {
            decision: Decision::allow("018f0000-0000-7000-8000-000000000000"),
            event: None,
            workspace_id: "ws_test".into(),
            environment_id: "production".into(),
            run_id: None,
            run_event_id: None,
            session_id: None,
            domain: "customer_support".into(),
        }
    }

    /// The enqueued trace carries the normalized event evidence.
    #[tokio::test]
    async fn trace_write_carries_event_evidence() {
        let mut state = memory_app_state(Arc::new(Engine::empty()));
        let (tx, mut rx) = mpsc::channel::<TraceWrite>(8);
        state.trace_tx = Some(tx);
        let app = router(state, None, [0u8; 32]);

        let resp = app
            .clone()
            .oneshot(check_request(&legacy_body()))
            .await
            .unwrap();
        assert_eq!(resp.status(), StatusCode::OK);

        let trace = rx.recv().await.expect("trace enqueued");
        let event = trace.event.expect("event evidence attached");
        assert_eq!(event.kind, EventKind::OutputProposed);
        assert_eq!(event.principal.agent_id, "anon");
        assert_eq!(event.action.operation, "output");
        assert_eq!(event.sources[0].id, "legacy.input");

        // Gateway-context check records low-fidelity gateway sources.
        let body = serde_json::json!({
            "agent_id": "anon",
            "channel": "chat",
            "input": "hi",
            "proposed_output": "hello there",
            "context": { "integration_mode": "gateway" }
        });
        let resp = app.oneshot(check_request(&body)).await.unwrap();
        assert_eq!(resp.status(), StatusCode::OK);

        let trace = rx.recv().await.expect("gateway trace enqueued");
        let event = trace.event.expect("event evidence attached");
        let ids: Vec<&str> = event.sources.iter().map(|s| s.id.as_str()).collect();
        assert_eq!(ids, vec!["input.observed", "model.output"]);
    }

    /// A full trace queue must never block or fail the request —
    /// the trace is dropped with a warning and the caller still gets
    /// its decision.
    #[tokio::test]
    async fn full_trace_queue_does_not_block_check() {
        let mut state = memory_app_state(Arc::new(Engine::empty()));
        let (tx, _rx) = mpsc::channel::<TraceWrite>(1);
        tx.try_send(dummy_trace_write()).unwrap();
        state.trace_tx = Some(tx);
        let app = router(state, None, [0u8; 32]);

        let resp = app.oneshot(check_request(&legacy_body())).await.unwrap();
        assert_eq!(resp.status(), StatusCode::OK);

        let decision: Decision = serde_json::from_value(read_body(resp).await).unwrap();
        assert_eq!(decision.verdict, Verdict::Allow);
    }
}
