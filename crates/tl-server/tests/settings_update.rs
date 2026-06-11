//! Rollout-control write endpoints: `PATCH /v1/settings` and
//! `GET`/`PUT /v1/environments/{id}/checker-modes`.
//!
//! Phase 7 of the event engine: checker modes become operator-writable,
//! scoped by workspace (settings) and environment (overrides).

use std::sync::Arc;

use async_trait::async_trait;
use axum::{
    body::Body,
    http::{header, Request, StatusCode},
};
use http_body_util::BodyExt;
use serde_json::json;
use tl_core::{EnforcementMode, EnvironmentCheckerModes, WorkspaceSettings};
use tl_engine::Engine;
use tl_server::dashboard_admin::DashboardAdminStoreError;
use tl_server::{memory_app_state, router, SettingsStore};
use tower::ServiceExt;

fn app() -> axum::Router {
    router(memory_app_state(Arc::new(Engine::empty())), None, [0u8; 32])
}

async fn read_body(resp: axum::response::Response) -> serde_json::Value {
    let bytes = resp.into_body().collect().await.unwrap().to_bytes();
    serde_json::from_slice(&bytes).unwrap()
}

fn json_request(method: &str, uri: &str, body: &serde_json::Value) -> Request<Body> {
    Request::builder()
        .method(method)
        .uri(uri)
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::from(body.to_string()))
        .unwrap()
}

fn get_request(uri: &str) -> Request<Body> {
    Request::builder()
        .method("GET")
        .uri(uri)
        .body(Body::empty())
        .unwrap()
}

#[tokio::test]
async fn patch_settings_updates_only_provided_fields() {
    let app = app();

    let resp = app
        .clone()
        .oneshot(json_request(
            "PATCH",
            "/v1/settings",
            &json!({ "flow_checker_mode": "shadow" }),
        ))
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let settings: WorkspaceSettings = serde_json::from_value(read_body(resp).await).unwrap();

    assert_eq!(settings.flow_checker_mode, EnforcementMode::Shadow);
    // Untouched fields keep their defaults.
    assert_eq!(settings.memory_checker_mode, EnforcementMode::Off);
    assert_eq!(settings.default_action, "allow");
    assert!(settings.telemetry_enabled);
    assert!(settings.updated_at.is_some());

    // The update persisted: a follow-up read returns the new mode.
    let resp = app.oneshot(get_request("/v1/settings")).await.unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let settings: WorkspaceSettings = serde_json::from_value(read_body(resp).await).unwrap();
    assert_eq!(settings.flow_checker_mode, EnforcementMode::Shadow);
}

#[tokio::test]
async fn patch_settings_with_empty_body_is_a_noop() {
    let app = app();

    let resp = app
        .oneshot(json_request("PATCH", "/v1/settings", &json!({})))
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let settings: WorkspaceSettings = serde_json::from_value(read_body(resp).await).unwrap();
    assert_eq!(settings.flow_checker_mode, EnforcementMode::Off);
    assert_eq!(settings.default_action, "allow");
}

#[tokio::test]
async fn patch_settings_rejects_invalid_mode_string() {
    let resp = app()
        .oneshot(json_request(
            "PATCH",
            "/v1/settings",
            &json!({ "flow_checker_mode": "loud" }),
        ))
        .await
        .unwrap();
    // Serde rejects the unknown enum variant at the extractor.
    assert_eq!(resp.status(), StatusCode::UNPROCESSABLE_ENTITY);
}

#[tokio::test]
async fn patch_settings_is_scoped_by_workspace_header() {
    let app = app();

    let resp = app
        .clone()
        .oneshot(
            Request::builder()
                .method("PATCH")
                .uri("/v1/settings")
                .header(header::CONTENT_TYPE, "application/json")
                .header("x-tlg-workspace-id", "ws_other")
                .body(Body::from(
                    json!({ "memory_checker_mode": "enforce" }).to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);

    // The default workspace is untouched.
    let resp = app.oneshot(get_request("/v1/settings")).await.unwrap();
    let settings: WorkspaceSettings = serde_json::from_value(read_body(resp).await).unwrap();
    assert_eq!(settings.memory_checker_mode, EnforcementMode::Off);
}

#[tokio::test]
async fn environment_checker_modes_round_trip() {
    let app = app();

    let resp = app
        .clone()
        .oneshot(json_request(
            "PUT",
            "/v1/environments/staging/checker-modes",
            &json!({ "flow_checker_mode": "enforce" }),
        ))
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let modes: EnvironmentCheckerModes = serde_json::from_value(read_body(resp).await).unwrap();
    assert_eq!(modes.flow_checker_mode, Some(EnforcementMode::Enforce));
    assert_eq!(modes.memory_checker_mode, None);
    assert!(modes.updated_at.is_some());

    let resp = app
        .oneshot(get_request("/v1/environments/staging/checker-modes"))
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let modes: EnvironmentCheckerModes = serde_json::from_value(read_body(resp).await).unwrap();
    assert_eq!(modes.flow_checker_mode, Some(EnforcementMode::Enforce));
}

#[tokio::test]
async fn environment_checker_modes_get_without_override_returns_all_inherit() {
    let resp = app()
        .oneshot(get_request("/v1/environments/production/checker-modes"))
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);

    // All-None overrides serialize to an empty object (fields skipped),
    // meaning "inherit everything from the workspace".
    let body = read_body(resp).await;
    assert_eq!(body, json!({}));
}

/// Store that rejects unknown environments, mirroring the postgres
/// repo's existence check.
struct StrictEnvironmentStore;

#[async_trait]
impl SettingsStore for StrictEnvironmentStore {
    async fn get(
        &self,
        _workspace_id: &str,
    ) -> Result<WorkspaceSettings, DashboardAdminStoreError> {
        Ok(tl_server::dashboard_admin::default_settings())
    }

    async fn put_environment_modes(
        &self,
        _workspace_id: &str,
        _environment_id: &str,
        _modes: EnvironmentCheckerModes,
    ) -> Result<EnvironmentCheckerModes, DashboardAdminStoreError> {
        Err(DashboardAdminStoreError::NotFound)
    }
}

#[tokio::test]
async fn put_environment_checker_modes_unknown_environment_returns_404() {
    let mut state = memory_app_state(Arc::new(Engine::empty()));
    state.settings_store = Arc::new(StrictEnvironmentStore);
    let app = router(state, None, [0u8; 32]);

    let resp = app
        .oneshot(json_request(
            "PUT",
            "/v1/environments/missing/checker-modes",
            &json!({ "flow_checker_mode": "shadow" }),
        ))
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::NOT_FOUND);

    let body = read_body(resp).await;
    assert_eq!(body["code"], "not_found");
}
