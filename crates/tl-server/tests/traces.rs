use std::sync::{Arc, Mutex};

use async_trait::async_trait;
use axum::{
    body::Body,
    http::{Request, StatusCode},
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

fn get(uri: &str, workspace_id: &str) -> Request<Body> {
    Request::builder()
        .method("GET")
        .uri(uri)
        .header("x-tlg-workspace-id", workspace_id)
        .header("x-tlg-environment-id", "production")
        .body(Body::empty())
        .unwrap()
}

fn trace(trace_id: &str, environment_id: &str) -> tl_core::TraceSummary {
    tl_core::TraceSummary {
        trace_id: trace_id.to_string(),
        run_id: Some("018f0000-0000-7000-8000-000000000001".to_string()),
        run_event_id: None,
        session_id: Some("sess_1".to_string()),
        environment_id: environment_id.to_string(),
        environment: environment_id.to_string(),
        domain: "payments".to_string(),
        decision: "escalate".to_string(),
        elapsed_ms: 42,
        latest_review_outcome: None,
        latest_reviewed_at: None,
        payload: serde_json::json!({
            "reason": "wire transfer requires human approval",
            "event": {
                "action": {
                    "operation": "send_wire",
                    "parameters": { "amount": 15000 }
                }
            }
        }),
        created_at: "2026-06-30T12:00:00Z".to_string(),
    }
}

#[derive(Default)]
struct LookupTraceStore {
    rows: Vec<(String, tl_core::TraceSummary)>,
    calls: Mutex<Vec<(String, String, String)>>,
}

#[async_trait]
impl TraceStore for LookupTraceStore {
    async fn list_recent(
        &self,
        _workspace_id: &str,
        _environment_id: &str,
        _session_id: Option<&str>,
        _limit: usize,
    ) -> Result<Vec<tl_core::TraceSummary>, TraceStoreError> {
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

    async fn get(
        &self,
        workspace_id: &str,
        environment_id: &str,
        trace_id: &str,
    ) -> Result<Option<tl_core::TraceSummary>, TraceStoreError> {
        self.calls.lock().unwrap().push((
            workspace_id.to_string(),
            environment_id.to_string(),
            trace_id.to_string(),
        ));
        Ok(self
            .rows
            .iter()
            .find(|(row_workspace_id, row)| {
                row_workspace_id == workspace_id
                    && row.environment_id == environment_id
                    && row.trace_id == trace_id
            })
            .map(|(_, row)| row.clone()))
    }
}

#[tokio::test]
async fn traces_lookup_returns_trace_from_resolved_workspace_and_environment() {
    let store = Arc::new(LookupTraceStore {
        rows: vec![("ws_1".to_string(), trace("trace_found", "production"))],
        calls: Mutex::default(),
    });
    let mut state = memory_app_state(Arc::new(Engine::empty()));
    state.trace_store = store.clone();
    let app = router(state, None, [0u8; 32]);

    let resp = app
        .oneshot(get("/v1/traces/trace_found", "ws_1"))
        .await
        .unwrap();

    assert_eq!(resp.status(), StatusCode::OK);
    let body = read_body(resp).await;
    assert_eq!(body["trace_id"], "trace_found");
    assert_eq!(
        body["payload"]["reason"],
        "wire transfer requires human approval"
    );
    assert_eq!(
        store.calls.lock().unwrap().as_slice(),
        &[(
            "ws_1".to_string(),
            "production".to_string(),
            "trace_found".to_string()
        )]
    );
}

#[tokio::test]
async fn traces_lookup_returns_not_found_for_unknown_or_other_workspace_trace() {
    let store = Arc::new(LookupTraceStore {
        rows: vec![("ws_other".to_string(), trace("trace_private", "production"))],
        calls: Mutex::default(),
    });
    let mut state = memory_app_state(Arc::new(Engine::empty()));
    state.trace_store = store;
    let app = router(state, None, [0u8; 32]);

    for trace_id in ["trace_missing", "trace_private"] {
        let resp = app
            .clone()
            .oneshot(get(&format!("/v1/traces/{trace_id}"), "ws_1"))
            .await
            .unwrap();

        assert_eq!(resp.status(), StatusCode::NOT_FOUND, "{trace_id}");
        let body = read_body(resp).await;
        assert_eq!(body["code"], "not_found");
    }
}
