//! Rollout-control write endpoints: `PATCH /v1/settings` and
//! `GET`/`PUT /v1/environments/{id}/checker-modes`.
//!
//! Phase 7 of the event engine: checker modes become operator-writable,
//! scoped by workspace (settings) and environment (overrides). Writes
//! require an Owner/Admin user; workspace runtime keys are rejected so a
//! running agent can never weaken the controls that govern it.

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
use tl_server::{memory_app_state, router, AuthConfig, MemoryUserStore, SettingsStore};
use tower::ServiceExt;
use uuid::Uuid;

/// Local-dev app (no bearer middleware) with one workspace owned by the
/// returned user. Writes authenticate via the forwarded-user headers.
async fn app_with_owner() -> (axum::Router, String, Uuid) {
    let state = memory_app_state(Arc::new(Engine::empty()));
    let owner_id = Uuid::new_v4();
    let workspace = state
        .team_store
        .create_workspace(owner_id, "Settings Workspace")
        .await
        .unwrap();
    (router(state, None, [0u8; 32]), workspace.id, owner_id)
}

async fn read_body(resp: axum::response::Response) -> serde_json::Value {
    let bytes = resp.into_body().collect().await.unwrap().to_bytes();
    serde_json::from_slice(&bytes).unwrap()
}

fn write_request(
    method: &str,
    uri: &str,
    workspace_id: &str,
    user_id: Uuid,
    body: &serde_json::Value,
) -> Request<Body> {
    Request::builder()
        .method(method)
        .uri(uri)
        .header(header::CONTENT_TYPE, "application/json")
        .header("x-tlg-workspace-id", workspace_id)
        .header("x-tlg-user-id", user_id.to_string())
        .body(Body::from(body.to_string()))
        .unwrap()
}

fn get_request(uri: &str, workspace_id: &str) -> Request<Body> {
    Request::builder()
        .method("GET")
        .uri(uri)
        .header("x-tlg-workspace-id", workspace_id)
        .body(Body::empty())
        .unwrap()
}

#[tokio::test]
async fn patch_settings_updates_only_provided_fields() {
    let (app, workspace_id, owner_id) = app_with_owner().await;

    let resp = app
        .clone()
        .oneshot(write_request(
            "PATCH",
            "/v1/settings",
            &workspace_id,
            owner_id,
            &json!({ "flow_checker_mode": "shadow" }),
        ))
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let settings: WorkspaceSettings = serde_json::from_value(read_body(resp).await).unwrap();

    assert_eq!(settings.flow_checker_mode, EnforcementMode::Shadow);
    // Untouched fields keep their defaults.
    assert_eq!(settings.memory_checker_mode, EnforcementMode::Off);
    assert_eq!(settings.default_action, "permit");
    assert!(settings.telemetry_enabled);
    assert!(settings.updated_at.is_some());

    // The update persisted: a follow-up read returns the new mode.
    let resp = app
        .oneshot(get_request("/v1/settings", &workspace_id))
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let settings: WorkspaceSettings = serde_json::from_value(read_body(resp).await).unwrap();
    assert_eq!(settings.flow_checker_mode, EnforcementMode::Shadow);
}

#[tokio::test]
async fn patch_settings_with_empty_body_is_a_noop() {
    let (app, workspace_id, owner_id) = app_with_owner().await;

    let resp = app
        .oneshot(write_request(
            "PATCH",
            "/v1/settings",
            &workspace_id,
            owner_id,
            &json!({}),
        ))
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let settings: WorkspaceSettings = serde_json::from_value(read_body(resp).await).unwrap();
    assert_eq!(settings.flow_checker_mode, EnforcementMode::Off);
    assert_eq!(settings.default_action, "permit");
}

#[tokio::test]
async fn patch_settings_rejects_invalid_mode_string() {
    let (app, workspace_id, owner_id) = app_with_owner().await;

    let resp = app
        .oneshot(write_request(
            "PATCH",
            "/v1/settings",
            &workspace_id,
            owner_id,
            &json!({ "flow_checker_mode": "loud" }),
        ))
        .await
        .unwrap();
    // Serde rejects the unknown enum variant at the extractor.
    assert_eq!(resp.status(), StatusCode::UNPROCESSABLE_ENTITY);
}

#[tokio::test]
async fn patch_settings_is_scoped_by_workspace_header() {
    let (app, workspace_a, owner_id) = app_with_owner().await;

    let resp = app
        .clone()
        .oneshot(write_request(
            "PATCH",
            "/v1/settings",
            &workspace_a,
            owner_id,
            &json!({ "memory_checker_mode": "enforce" }),
        ))
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);

    // Another workspace is untouched.
    let resp = app
        .oneshot(get_request("/v1/settings", "ws_other"))
        .await
        .unwrap();
    let settings: WorkspaceSettings = serde_json::from_value(read_body(resp).await).unwrap();
    assert_eq!(settings.memory_checker_mode, EnforcementMode::Off);
}

#[tokio::test]
async fn settings_write_without_user_is_unauthorized() {
    let (app, workspace_id, _) = app_with_owner().await;

    let resp = app
        .oneshot(
            Request::builder()
                .method("PATCH")
                .uri("/v1/settings")
                .header(header::CONTENT_TYPE, "application/json")
                .header("x-tlg-workspace-id", workspace_id)
                .body(Body::from(
                    json!({ "flow_checker_mode": "enforce" }).to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn settings_write_requires_owner_or_admin_role() {
    let (app, workspace_id, _) = app_with_owner().await;
    let outsider = Uuid::new_v4();

    let resp = app
        .oneshot(write_request(
            "PATCH",
            "/v1/settings",
            &workspace_id,
            outsider,
            &json!({ "flow_checker_mode": "off" }),
        ))
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::FORBIDDEN);
}

#[tokio::test]
async fn workspace_runtime_key_cannot_modify_settings() {
    // Full bearer-auth flow: mint a runtime key as the workspace owner,
    // then try to weaken enforcement with it. A running agent must never
    // be able to change the controls that govern it.
    let mut state = memory_app_state(Arc::new(Engine::empty()));
    let user_store = Arc::new(MemoryUserStore::new());
    let owner_id = user_store
        .create_approved_for_tests("runtime-settings-owner@example.com")
        .await
        .unwrap()
        .id;
    state.user_store = user_store;
    let workspace = state
        .team_store
        .create_workspace(owner_id, "Runtime Workspace")
        .await
        .unwrap();
    let app = router(state, Some(AuthConfig::new("sk-internal")), [0u8; 32]);

    let create_resp = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/v1/api-keys")
                .header(header::CONTENT_TYPE, "application/json")
                .header(header::AUTHORIZATION, "Bearer sk-internal")
                .header("x-tlg-workspace-id", workspace.id.as_str())
                .header("x-tlg-user-id", owner_id.to_string())
                .body(Body::from(json!({ "name": "agent key" }).to_string()))
                .unwrap(),
        )
        .await
        .unwrap();
    let create_status = create_resp.status();
    let created = read_body(create_resp).await;
    assert_eq!(
        create_status,
        StatusCode::CREATED,
        "runtime key creation failed: {created}"
    );
    let runtime_key = created["plaintext_key"].as_str().unwrap();

    let patch_resp = app
        .clone()
        .oneshot(
            Request::builder()
                .method("PATCH")
                .uri("/v1/settings")
                .header(header::CONTENT_TYPE, "application/json")
                .header(header::AUTHORIZATION, format!("Bearer {runtime_key}"))
                .body(Body::from(
                    json!({ "flow_checker_mode": "off" }).to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(patch_resp.status(), StatusCode::FORBIDDEN);

    let put_resp = app
        .oneshot(
            Request::builder()
                .method("PUT")
                .uri("/v1/environments/default/checker-modes")
                .header(header::CONTENT_TYPE, "application/json")
                .header(header::AUTHORIZATION, format!("Bearer {runtime_key}"))
                .body(Body::from(
                    json!({ "flow_checker_mode": "off" }).to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(put_resp.status(), StatusCode::FORBIDDEN);
}

#[tokio::test]
async fn environment_checker_modes_round_trip() {
    let (app, workspace_id, owner_id) = app_with_owner().await;

    let resp = app
        .clone()
        .oneshot(write_request(
            "PUT",
            "/v1/environments/staging/checker-modes",
            &workspace_id,
            owner_id,
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
        .oneshot(get_request(
            "/v1/environments/staging/checker-modes",
            &workspace_id,
        ))
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let modes: EnvironmentCheckerModes = serde_json::from_value(read_body(resp).await).unwrap();
    assert_eq!(modes.flow_checker_mode, Some(EnforcementMode::Enforce));
}

#[tokio::test]
async fn environment_checker_modes_get_without_override_returns_all_inherit() {
    let (app, workspace_id, _) = app_with_owner().await;

    let resp = app
        .oneshot(get_request(
            "/v1/environments/production/checker-modes",
            &workspace_id,
        ))
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
    let owner_id = Uuid::new_v4();
    let workspace = state
        .team_store
        .create_workspace(owner_id, "Strict Workspace")
        .await
        .unwrap();
    state.settings_store = Arc::new(StrictEnvironmentStore);
    let app = router(state, None, [0u8; 32]);

    let resp = app
        .oneshot(write_request(
            "PUT",
            "/v1/environments/missing/checker-modes",
            &workspace.id,
            owner_id,
            &json!({ "flow_checker_mode": "shadow" }),
        ))
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::NOT_FOUND);

    let body = read_body(resp).await;
    assert_eq!(body["code"], "not_found");
}

#[tokio::test]
async fn patch_settings_rejects_unknown_default_action() {
    let (app, workspace_id, owner_id) = app_with_owner().await;

    let resp = app
        .oneshot(write_request(
            "PATCH",
            "/v1/settings",
            &workspace_id,
            owner_id,
            &json!({ "default_action": "allow_everything" }),
        ))
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::BAD_REQUEST);
    let body = read_body(resp).await;
    assert_eq!(body["code"], "invalid");
}

#[tokio::test]
async fn patch_settings_rejects_non_numeric_retention_days() {
    let (app, workspace_id, owner_id) = app_with_owner().await;

    let resp = app
        .oneshot(write_request(
            "PATCH",
            "/v1/settings",
            &workspace_id,
            owner_id,
            &json!({ "retention_days": "forever" }),
        ))
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::BAD_REQUEST);
    let body = read_body(resp).await;
    assert_eq!(body["code"], "invalid");
}
