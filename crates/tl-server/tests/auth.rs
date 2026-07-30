//! E2E tests for the bearer-auth middleware. We invoke the router as a
//! tower service via `oneshot` so the tests don't need a real TCP
//! listener — pure in-process Service::call.

use std::sync::Arc;

use axum::{
    body::Body,
    http::{header, Request, StatusCode},
};
use http_body_util::BodyExt;
use tl_core::{ApiError, AuthorizationEffect};
use tl_engine::Engine;
use tl_server::{jwt::JwtSigner, memory_app_state, router, AuthConfig, MemoryUserStore};
use tower::ServiceExt;
use uuid::Uuid;

fn build_app(auth: Option<Arc<AuthConfig>>) -> axum::Router {
    let state = memory_app_state(Arc::new(Engine::empty()));
    router(state, auth, [0u8; 32])
}

fn build_app_with_jwt() -> (axum::Router, Arc<JwtSigner>) {
    let mut state = memory_app_state(Arc::new(Engine::empty()));
    let signer = JwtSigner::new("test-secret-test-secret-test-secret-12");
    state.jwt_signer = Some(signer.clone());
    (
        router(state, Some(AuthConfig::new("sk-internal")), [0u8; 32]),
        signer,
    )
}

async fn build_app_with_approved_user(auth: Option<Arc<AuthConfig>>) -> (axum::Router, Uuid) {
    let mut state = memory_app_state(Arc::new(Engine::empty()));
    let user_store = Arc::new(MemoryUserStore::new());
    let user = user_store
        .create_approved_for_tests("approved@example.com")
        .await
        .unwrap();
    state.user_store = user_store;
    (router(state, auth, [0u8; 32]), user.id)
}

async fn build_app_with_jwt_and_approved_users() -> (axum::Router, Arc<JwtSigner>, Uuid, Uuid) {
    let mut state = memory_app_state(Arc::new(Engine::empty()));
    let user_store = Arc::new(MemoryUserStore::new());
    let owner = user_store
        .create_approved_for_tests("owner@example.com")
        .await
        .unwrap();
    let outsider = user_store
        .create_approved_for_tests("outsider@example.com")
        .await
        .unwrap();
    let signer = JwtSigner::new("test-secret-test-secret-test-secret-12");
    state.user_store = user_store;
    state.jwt_signer = Some(signer.clone());
    (
        router(state, Some(AuthConfig::new("sk-internal")), [0u8; 32]),
        signer,
        owner.id,
        outsider.id,
    )
}

async fn build_app_with_unapproved_user() -> (axum::Router, Uuid) {
    let mut state = memory_app_state(Arc::new(Engine::empty()));
    state.workspace_self_service_enabled = false;
    let user = state
        .user_store
        .create("pending@example.com", "oauth:external-provider")
        .await
        .unwrap();
    (
        router(state, Some(AuthConfig::new("sk-internal")), [0u8; 32]),
        user.id,
    )
}

fn event_request(token: Option<&str>) -> Request<Body> {
    let body = serde_json::json!({
        "kind": "output.proposed",
        "principal": {
            "workspace_id": "default",
            "environment_id": "production",
            "agent_id": "a"
        },
        "action": {
            "operation": "output",
            "parameters": { "text": "hello" },
            "side_effect": "none"
        },
        "sources": [{ "id": "input", "origin": "user", "labels": {} }],
        "provenance": { "text": ["input"] },
        "context": { "channel": "chat", "domain": "customer_support" }
    });
    let mut b = Request::builder()
        .method("POST")
        .uri("/v1/events")
        .header(header::CONTENT_TYPE, "application/json")
        .header("x-tlg-workspace-id", "ws");
    if let Some(t) = token {
        b = b.header(header::AUTHORIZATION, format!("Bearer {t}"));
    }
    b.body(Body::from(body.to_string())).unwrap()
}

fn policy_validate_request(token: Option<&str>) -> Request<Body> {
    let body = r#"
id: pii-block
description: Block obvious PII
when:
  channels: [chat]
match:
  regex: "\\b\\d{3}-\\d{2}-\\d{4}\\b"
action:
  effect: block
"#;
    let mut b = Request::builder()
        .method("POST")
        .uri("/v1/policies/validate")
        .header(header::CONTENT_TYPE, "application/x-yaml");
    if let Some(t) = token {
        b = b.header(header::AUTHORIZATION, format!("Bearer {t}"));
    }
    b.body(Body::from(body)).unwrap()
}

fn create_api_key_request(token: &str, workspace_id: &str, name: &str) -> Request<Body> {
    let body = serde_json::json!({ "name": name });
    Request::builder()
        .method("POST")
        .uri("/v1/api-keys")
        .header(header::CONTENT_TYPE, "application/json")
        .header(header::AUTHORIZATION, format!("Bearer {token}"))
        .header("x-tlg-workspace-id", workspace_id)
        .body(Body::from(body.to_string()))
        .unwrap()
}

fn create_api_key_request_with_user(
    token: &str,
    workspace_id: &str,
    name: &str,
    user_id: Uuid,
) -> Request<Body> {
    let body = serde_json::json!({ "name": name });
    Request::builder()
        .method("POST")
        .uri("/v1/api-keys")
        .header(header::CONTENT_TYPE, "application/json")
        .header(header::AUTHORIZATION, format!("Bearer {token}"))
        .header("x-tlg-workspace-id", workspace_id)
        .header("x-tlg-user-id", user_id.to_string())
        .body(Body::from(body.to_string()))
        .unwrap()
}

fn list_api_keys_request(token: &str, workspace_id: &str) -> Request<Body> {
    Request::builder()
        .method("GET")
        .uri("/v1/api-keys")
        .header(header::AUTHORIZATION, format!("Bearer {token}"))
        .header("x-tlg-workspace-id", workspace_id)
        .body(Body::empty())
        .unwrap()
}

fn list_api_keys_request_with_user(
    token: &str,
    workspace_id: &str,
    user_id: Uuid,
) -> Request<Body> {
    Request::builder()
        .method("GET")
        .uri("/v1/api-keys")
        .header(header::AUTHORIZATION, format!("Bearer {token}"))
        .header("x-tlg-workspace-id", workspace_id)
        .header("x-tlg-user-id", user_id.to_string())
        .body(Body::empty())
        .unwrap()
}

fn revoke_api_keys_request(token: &str, workspace_id: &str, ids: &[&str]) -> Request<Body> {
    let body = serde_json::json!({ "ids": ids });
    Request::builder()
        .method("PATCH")
        .uri("/v1/api-keys/batch/revoke")
        .header(header::CONTENT_TYPE, "application/json")
        .header(header::AUTHORIZATION, format!("Bearer {token}"))
        .header("x-tlg-workspace-id", workspace_id)
        .body(Body::from(body.to_string()))
        .unwrap()
}

fn revoke_api_keys_request_with_user(
    token: &str,
    workspace_id: &str,
    ids: &[&str],
    user_id: Uuid,
) -> Request<Body> {
    let body = serde_json::json!({ "ids": ids });
    Request::builder()
        .method("PATCH")
        .uri("/v1/api-keys/batch/revoke")
        .header(header::CONTENT_TYPE, "application/json")
        .header(header::AUTHORIZATION, format!("Bearer {token}"))
        .header("x-tlg-workspace-id", workspace_id)
        .header("x-tlg-user-id", user_id.to_string())
        .body(Body::from(body.to_string()))
        .unwrap()
}

fn oauth_session_request(
    token: Option<&str>,
    provider: &str,
    provider_subject: &str,
    email: &str,
) -> Request<Body> {
    let body = serde_json::json!({
        "provider": provider,
        "provider_subject": provider_subject,
        "email": email,
        "name": "Test User"
    });
    let mut b = Request::builder()
        .method("POST")
        .uri("/v1/identity/oauth-session")
        .header(header::CONTENT_TYPE, "application/json");
    if let Some(t) = token {
        b = b.header(header::AUTHORIZATION, format!("Bearer {t}"));
    }
    b.body(Body::from(body.to_string())).unwrap()
}

fn oauth_authorize_request(token: &str, workspace_id: &str, user_id: Uuid) -> Request<Body> {
    Request::builder()
        .method("POST")
        .uri("/v1/oauth/authorize")
        .header(header::CONTENT_TYPE, "application/json")
        .header(header::AUTHORIZATION, format!("Bearer {token}"))
        .header("x-tlg-workspace-id", workspace_id)
        .header("x-tlg-user-id", user_id.to_string())
        .body(Body::from(
            serde_json::json!({
                "client_id": "attacker-client",
                "redirect_uri": "https://attacker.example/callback",
                "code_challenge": "attacker-challenge",
                "code_challenge_method": "S256",
                "agent_id": "attacker-agent",
            })
            .to_string(),
        ))
        .unwrap()
}

fn my_workspaces_request(token: &str, user_id: Uuid) -> Request<Body> {
    Request::builder()
        .method("GET")
        .uri("/v1/team/my-workspaces")
        .header(header::AUTHORIZATION, format!("Bearer {token}"))
        .header("x-tlg-user-id", user_id.to_string())
        .body(Body::empty())
        .unwrap()
}

fn create_workspace_request(token: &str) -> Request<Body> {
    let body = serde_json::json!({ "name": "Acme Support" });
    Request::builder()
        .method("POST")
        .uri("/v1/team/my-workspaces")
        .header(header::CONTENT_TYPE, "application/json")
        .header(header::AUTHORIZATION, format!("Bearer {token}"))
        .body(Body::from(body.to_string()))
        .unwrap()
}

fn create_workspace_request_for_user(token: &str, user_id: Uuid, name: &str) -> Request<Body> {
    let body = serde_json::json!({ "name": name });
    Request::builder()
        .method("POST")
        .uri("/v1/team/my-workspaces")
        .header(header::CONTENT_TYPE, "application/json")
        .header(header::AUTHORIZATION, format!("Bearer {token}"))
        .header("x-tlg-user-id", user_id.to_string())
        .body(Body::from(body.to_string()))
        .unwrap()
}

async fn create_workspace_for_user(app: axum::Router, user_id: Uuid, name: &str) -> String {
    let resp = app
        .oneshot(create_workspace_request_for_user(
            "sk-internal",
            user_id,
            name,
        ))
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::CREATED);
    let body = read_body(resp).await;
    body["id"].as_str().unwrap().to_string()
}

async fn read_body(resp: axum::response::Response) -> serde_json::Value {
    let bytes = resp.into_body().collect().await.unwrap().to_bytes();
    if bytes.is_empty() {
        serde_json::Value::Null
    } else {
        serde_json::from_slice(&bytes).unwrap_or(serde_json::Value::Null)
    }
}

#[tokio::test]
async fn missing_bearer_returns_401_with_api_error_envelope() {
    let app = build_app(Some(AuthConfig::new("sk-correct")));
    let resp = app.oneshot(event_request(None)).await.unwrap();
    assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);

    let body: ApiError = serde_json::from_value(read_body(resp).await).expect("ApiError");
    assert!(matches!(body.code, tl_core::ApiErrorCode::Unauthorized));
    assert!(body.message.contains("missing"));
    assert!(!body.retriable);
}

#[tokio::test]
async fn wrong_bearer_returns_401() {
    let app = build_app(Some(AuthConfig::new("sk-correct")));
    let resp = app.oneshot(event_request(Some("sk-wrong"))).await.unwrap();
    assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);

    let body: ApiError = serde_json::from_value(read_body(resp).await).expect("ApiError");
    assert!(body.message.contains("invalid"));
}

#[tokio::test]
async fn correct_bearer_returns_200() {
    let app = build_app(Some(AuthConfig::new("sk-correct")));
    let resp = app
        .oneshot(event_request(Some("sk-correct")))
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let body = read_body(resp).await;
    assert_eq!(body["effect"], "permit");
}

#[tokio::test]
async fn correct_bearer_can_call_policy_authoring_routes() {
    let app = build_app(Some(AuthConfig::new("sk-correct")));

    let resp = app
        .oneshot(policy_validate_request(Some("sk-correct")))
        .await
        .unwrap();

    assert_eq!(resp.status(), StatusCode::OK);
}

#[tokio::test]
async fn approval_gate_blocks_unapproved_forwarded_user() {
    let (app, user_id) = build_app_with_unapproved_user().await;

    let resp = app
        .oneshot(my_workspaces_request("sk-internal", user_id))
        .await
        .unwrap();

    assert_eq!(resp.status(), StatusCode::FORBIDDEN);
    let body: ApiError = serde_json::from_value(read_body(resp).await).expect("ApiError");
    assert!(matches!(body.code, tl_core::ApiErrorCode::Forbidden));
    assert!(body.message.contains("not approved"));
}

#[tokio::test]
async fn disabled_self_service_blocks_workspace_creation() {
    let (app, _) = build_app_with_unapproved_user().await;

    let resp = app
        .oneshot(create_workspace_request("sk-internal"))
        .await
        .unwrap();

    assert_eq!(resp.status(), StatusCode::FORBIDDEN);
    let body: ApiError = serde_json::from_value(read_body(resp).await).expect("ApiError");
    assert!(body.message.contains("self-service"));
}

include!("auth/oauth.rs");

include!("auth/api_keys.rs");

#[tokio::test]
async fn health_endpoint_works_without_token() {
    let app = build_app(Some(AuthConfig::new("sk-correct")));
    let req = Request::builder()
        .method("GET")
        .uri("/health")
        .body(Body::empty())
        .unwrap();
    let resp = app.oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
}

#[tokio::test]
async fn health_endpoint_works_with_random_token_too() {
    // Health bypasses auth entirely — extra credential doesn't break it.
    let app = build_app(Some(AuthConfig::new("sk-correct")));
    let req = Request::builder()
        .method("GET")
        .uri("/health")
        .header(header::AUTHORIZATION, "Bearer literally-anything")
        .body(Body::empty())
        .unwrap();
    let resp = app.oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
}

#[tokio::test]
async fn no_auth_config_disables_middleware() {
    // Local-dev / test mode: auth=None means /v1/events accepts any
    // request (or no Authorization header at all).
    let app = build_app(None);
    let resp = app.oneshot(event_request(None)).await.unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
}

#[tokio::test]
async fn malformed_authorization_header_yields_401() {
    // Bare "Bearer" with no token, or a non-Bearer scheme — both should
    // be rejected as missing.
    let app = build_app(Some(AuthConfig::new("sk-correct")));

    for header_value in ["", "Bearer", "Basic Zm9vOmJhcg==", "Token sk-correct"] {
        let req = Request::builder()
            .method("POST")
            .uri("/v1/events")
            .header(header::CONTENT_TYPE, "application/json")
            .header(header::AUTHORIZATION, header_value)
            .body(Body::from(
                serde_json::json!({
                    "kind": "output.proposed",
                    "principal": {
                        "workspace_id": "default",
                        "environment_id": "production",
                        "agent_id": "a"
                    },
                    "action": {
                        "operation": "output",
                        "parameters": { "text": "y" },
                        "side_effect": "none"
                    }
                })
                .to_string(),
            ))
            .unwrap();
        let resp = app.clone().oneshot(req).await.unwrap();
        assert_eq!(
            resp.status(),
            StatusCode::UNAUTHORIZED,
            "expected 401 for header `{header_value}`"
        );
    }
}
