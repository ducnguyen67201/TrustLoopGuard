//! Monitoring sessions: session ids flow from `/v1/events` into trace
//! writes, are length-bounded, and `GET /v1/traces?session_id=` plumbs
//! the filter to the trace store.

use std::sync::{Arc, Mutex};

use async_trait::async_trait;
use axum::{
    body::Body,
    http::{header, Request, StatusCode},
};
use http_body_util::BodyExt;
use tl_engine::Engine;
use tl_server::traces::{TraceStore, TraceStoreError};
use tl_server::{memory_app_state, router};
use tower::ServiceExt;

async fn read_body(resp: axum::response::Response) -> serde_json::Value {
    let bytes = resp.into_body().collect().await.unwrap().to_bytes();
    serde_json::from_slice(&bytes).unwrap()
}

fn post_json(uri: &str, body: &serde_json::Value) -> Request<Body> {
    Request::builder()
        .method("POST")
        .uri(uri)
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::from(body.to_string()))
        .unwrap()
}

fn event_body(session_id: Option<&str>) -> serde_json::Value {
    let mut body = serde_json::json!({
        "kind": "tool.call.proposed",
        "principal": {
            "workspace_id": "ws_claimed",
            "environment_id": "env_claimed",
            "agent_id": "agent-1"
        },
        "action": {
            "operation": "send_email",
            "parameters": { "recipient": "a@b.c" }
        }
    });
    if let Some(session_id) = session_id {
        body["principal"]["session_id"] = serde_json::json!(session_id);
    }
    body
}

fn oversized_session() -> String {
    "s".repeat(257)
}

#[tokio::test]
async fn event_rejects_oversized_session_id() {
    let app = router(memory_app_state(Arc::new(Engine::empty())), None, [0u8; 32]);

    let resp = app
        .oneshot(post_json(
            "/v1/events",
            &event_body(Some(&oversized_session())),
        ))
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::UNPROCESSABLE_ENTITY);

    let value = read_body(resp).await;
    assert!(value["message"].as_str().unwrap().contains("session_id"));
}

/// Records the arguments `list_recent` was called with so the handler's
/// query-string plumbing can be asserted without Postgres.
#[derive(Default)]
struct RecordingTraceStore {
    calls: Mutex<Vec<Option<String>>>,
}

#[async_trait]
impl TraceStore for RecordingTraceStore {
    async fn list_recent(
        &self,
        _workspace_id: &str,
        _environment_id: &str,
        session_id: Option<&str>,
        _limit: usize,
    ) -> Result<Vec<tl_core::TraceSummary>, TraceStoreError> {
        self.calls
            .lock()
            .unwrap()
            .push(session_id.map(str::to_string));
        Ok(vec![])
    }

    async fn sum_payment_minor_since(
        &self,
        _workspace_id: &str,
        _owner: &str,
        _operations: &[String],
        _since: chrono::DateTime<chrono::Utc>,
    ) -> Result<i64, TraceStoreError> {
        Ok(0)
    }

    async fn record(
        &self,
        _write: tl_server::traces::TraceWriteRequest,
    ) -> Result<(), TraceStoreError> {
        Ok(())
    }
}

#[tokio::test]
async fn traces_endpoint_plumbs_session_filter_to_store() {
    let store = Arc::new(RecordingTraceStore::default());
    let mut state = memory_app_state(Arc::new(Engine::empty()));
    state.trace_store = store.clone();
    let app = router(state, None, [0u8; 32]);

    for uri in [
        "/v1/traces?session_id=sess_a&limit=5",
        "/v1/traces",
        "/v1/traces?session_id=",
        // Percent-encoded values must decode (`%5F` is `_`), matching
        // the other query parsers in the crate.
        "/v1/traces?session_id=sess%5Fb",
    ] {
        let resp = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("GET")
                    .uri(uri)
                    .header("x-tlg-workspace-id", "ws_x")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(resp.status(), StatusCode::OK, "{uri}");
    }

    let calls = store.calls.lock().unwrap();
    assert_eq!(calls[0].as_deref(), Some("sess_a"));
    assert_eq!(calls[1], None);
    // An empty `session_id=` value means "no filter", not "match empty".
    assert_eq!(calls[2], None);
    assert_eq!(calls[3].as_deref(), Some("sess_b"));
}

#[cfg(feature = "postgres")]
mod trace_writes {
    use super::*;
    use tl_server::traces::ChannelTraceStore;

    #[tokio::test]
    async fn event_trace_write_carries_session_id() {
        let mut state = memory_app_state(Arc::new(Engine::empty()));
        let (capture, mut rx) = ChannelTraceStore::channel(8);
        state.trace_store = capture;
        let app = router(state, None, [0u8; 32]);

        let resp = app
            .oneshot(post_json("/v1/events", &event_body(Some("sess_event"))))
            .await
            .unwrap();
        assert_eq!(resp.status(), StatusCode::OK);

        let trace = rx.recv().await.expect("trace enqueued");
        assert_eq!(trace.session_id.as_deref(), Some("sess_event"));
        assert_eq!(trace.domain, "event");
    }
}
