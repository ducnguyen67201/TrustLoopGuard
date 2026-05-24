//! E2E tests for the bearer-auth middleware. We invoke the router as a
//! tower service via `oneshot` so the tests don't need a real TCP
//! listener — pure in-process Service::call.

use std::sync::Arc;

use axum::{
    body::Body,
    http::{header, Request, StatusCode},
};
use http_body_util::BodyExt;
use tl_core::{ApiError, Verdict};
use tl_engine::Engine;
use tl_server::{jwt::JwtSigner, memory_app_state, router, AuthConfig};
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

async fn build_hosted_app_with_unapproved_user() -> (axum::Router, Uuid) {
    let mut state = memory_app_state(Arc::new(Engine::empty()));
    state.hosted_user_approval_required = true;
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

fn check_request(token: Option<&str>) -> Request<Body> {
    let body = serde_json::json!({
        "agent_id": "a",
        "channel": "chat",
        "input": "hi",
        "proposed_output": "hello"
    });
    let mut b = Request::builder()
        .method("POST")
        .uri("/v1/check")
        .header(header::CONTENT_TYPE, "application/json");
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
  verdict: block
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
    let resp = app.oneshot(check_request(None)).await.unwrap();
    assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);

    let body: ApiError = serde_json::from_value(read_body(resp).await).expect("ApiError");
    assert!(matches!(body.code, tl_core::ApiErrorCode::Unauthorized));
    assert!(body.message.contains("missing"));
    assert!(!body.retriable);
}

#[tokio::test]
async fn wrong_bearer_returns_401() {
    let app = build_app(Some(AuthConfig::new("sk-correct")));
    let resp = app.oneshot(check_request(Some("sk-wrong"))).await.unwrap();
    assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);

    let body: ApiError = serde_json::from_value(read_body(resp).await).expect("ApiError");
    assert!(body.message.contains("invalid"));
}

#[tokio::test]
async fn correct_bearer_returns_200() {
    let app = build_app(Some(AuthConfig::new("sk-correct")));
    let resp = app
        .oneshot(check_request(Some("sk-correct")))
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let body = read_body(resp).await;
    assert_eq!(body["verdict"], "allow");
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
async fn hosted_gate_blocks_unapproved_forwarded_user() {
    let (app, user_id) = build_hosted_app_with_unapproved_user().await;

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
async fn hosted_gate_blocks_workspace_self_service_creation() {
    let (app, _) = build_hosted_app_with_unapproved_user().await;

    let resp = app
        .oneshot(create_workspace_request("sk-internal"))
        .await
        .unwrap();

    assert_eq!(resp.status(), StatusCode::FORBIDDEN);
    let body: ApiError = serde_json::from_value(read_body(resp).await).expect("ApiError");
    assert!(body.message.contains("self-service"));
}

#[tokio::test]
async fn oauth_session_requires_internal_bearer() {
    let app = build_app(Some(AuthConfig::new("sk-internal")));

    let resp = app
        .oneshot(oauth_session_request(
            None,
            "google",
            "google-subject",
            "user@example.com",
        ))
        .await
        .unwrap();

    assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn oauth_session_links_google_and_github_to_same_local_user_by_email() {
    let app = build_app(Some(AuthConfig::new("sk-internal")));

    let google_resp = app
        .clone()
        .oneshot(oauth_session_request(
            Some("sk-internal"),
            "google",
            "google-subject",
            "user@example.com",
        ))
        .await
        .unwrap();
    assert_eq!(google_resp.status(), StatusCode::OK);
    let google = read_body(google_resp).await;

    let github_resp = app
        .oneshot(oauth_session_request(
            Some("sk-internal"),
            "github",
            "github-subject",
            "user@example.com",
        ))
        .await
        .unwrap();
    assert_eq!(github_resp.status(), StatusCode::OK);
    let github = read_body(github_resp).await;

    assert_eq!(google["user_id"], github["user_id"]);
    assert_eq!(github["username"], "user@example.com");
}

#[tokio::test]
async fn internal_bearer_with_forwarded_user_can_issue_workspace_key_used_by_sdk_runtime() {
    let app = build_app(Some(AuthConfig::new("sk-internal")));
    let user_id = Uuid::new_v4();
    let workspace_id = create_workspace_for_user(app.clone(), user_id, "Runtime Workspace").await;

    let create_resp = app
        .clone()
        .oneshot(create_api_key_request_with_user(
            "sk-internal",
            &workspace_id,
            "SDK integration",
            user_id,
        ))
        .await
        .unwrap();
    assert_eq!(create_resp.status(), StatusCode::CREATED);
    let created = read_body(create_resp).await;
    let plaintext = created["plaintext_key"]
        .as_str()
        .expect("plaintext key is returned once");
    assert!(plaintext.starts_with("tl_live_"));
    assert_eq!(created["api_key"]["name"], "SDK integration");
    assert_eq!(created["api_key"]["status"], "active");
    assert_eq!(created["api_key"]["last_used_at"], serde_json::Value::Null);
    assert!(created["api_key"]["prefix"]
        .as_str()
        .unwrap()
        .starts_with("tl_live_"));
    assert!(created.get("key_hash").is_none());

    let list_resp = app
        .clone()
        .oneshot(list_api_keys_request_with_user(
            "sk-internal",
            &workspace_id,
            user_id,
        ))
        .await
        .unwrap();
    assert_eq!(list_resp.status(), StatusCode::OK);
    let listed = read_body(list_resp).await;
    assert_eq!(listed["api_keys"].as_array().unwrap().len(), 1);
    assert_eq!(
        listed["api_keys"][0]["prefix"],
        created["api_key"]["prefix"]
    );
    assert!(!listed.to_string().contains(plaintext));

    let other_workspace_policy = r#"
id: wrong-workspace-block
description: Would block if caller-controlled workspace won
when:
  channels: [chat]
match:
  literal: deny me
action: block
"#;
    let upsert_resp = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/v1/policies")
                .header(header::CONTENT_TYPE, "application/x-yaml")
                .header(header::AUTHORIZATION, "Bearer sk-internal")
                .header("x-tlg-workspace-id", "ws_wrong")
                .body(Body::from(other_workspace_policy))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(upsert_resp.status(), StatusCode::CREATED);

    let check_body = serde_json::json!({
        "agent_id": "a",
        "channel": "chat",
        "input": "deny me",
        "proposed_output": "deny me",
        "workspace_id": "ws_wrong"
    });
    let check_resp = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/v1/check")
                .header(header::CONTENT_TYPE, "application/json")
                .header(header::AUTHORIZATION, format!("Bearer {plaintext}"))
                .header("x-tlg-workspace-id", "ws_wrong")
                .body(Body::from(check_body.to_string()))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(check_resp.status(), StatusCode::OK);
    let decision = read_body(check_resp).await;
    assert_eq!(decision["verdict"], serde_json::json!(Verdict::Allow));
}

#[tokio::test]
async fn local_dev_without_auth_config_can_manage_api_keys_with_forwarded_user() {
    let app = build_app(None);
    let user_id = Uuid::new_v4();
    let workspace_id = create_workspace_for_user(app.clone(), user_id, "Local Dev Workspace").await;

    let create_resp = app
        .clone()
        .oneshot(create_api_key_request_with_user(
            "unused-local-token",
            &workspace_id,
            "SDK integration",
            user_id,
        ))
        .await
        .unwrap();
    assert_eq!(create_resp.status(), StatusCode::CREATED);

    let list_resp = app
        .oneshot(list_api_keys_request_with_user(
            "unused-local-token",
            &workspace_id,
            user_id,
        ))
        .await
        .unwrap();
    assert_eq!(list_resp.status(), StatusCode::OK);
}

#[tokio::test]
async fn local_dev_missing_forwarded_user_is_unauthorized() {
    let app = build_app(None);
    let user_id = Uuid::new_v4();
    let workspace_id = create_workspace_for_user(app.clone(), user_id, "Local Dev Workspace").await;

    let create_resp = app
        .clone()
        .oneshot(create_api_key_request(
            "unused-local-token",
            &workspace_id,
            "SDK integration",
        ))
        .await
        .unwrap();
    assert_eq!(create_resp.status(), StatusCode::UNAUTHORIZED);

    let list_resp = app
        .oneshot(list_api_keys_request("unused-local-token", &workspace_id))
        .await
        .unwrap();
    assert_eq!(list_resp.status(), StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn internal_bearer_can_revoke_workspace_keys() {
    let app = build_app(Some(AuthConfig::new("sk-internal")));
    let user_id = Uuid::new_v4();
    let workspace_id = create_workspace_for_user(app.clone(), user_id, "Runtime Workspace").await;
    let create_resp = app
        .clone()
        .oneshot(create_api_key_request_with_user(
            "sk-internal",
            &workspace_id,
            "SDK integration",
            user_id,
        ))
        .await
        .unwrap();
    assert_eq!(create_resp.status(), StatusCode::CREATED);
    let created = read_body(create_resp).await;
    let key_id = created["api_key"]["id"].as_str().unwrap();
    let plaintext = created["plaintext_key"].as_str().unwrap();

    let revoke_resp = app
        .clone()
        .oneshot(revoke_api_keys_request_with_user(
            "sk-internal",
            &workspace_id,
            &[key_id],
            user_id,
        ))
        .await
        .unwrap();
    assert_eq!(revoke_resp.status(), StatusCode::OK);
    let revoked = read_body(revoke_resp).await;
    assert_eq!(revoked["api_keys"][0]["status"], "revoked");

    let check_body = serde_json::json!({
        "agent_id": "a",
        "channel": "chat",
        "input": "hi",
        "proposed_output": "hello",
    });
    let check_resp = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/v1/check")
                .header(header::CONTENT_TYPE, "application/json")
                .header(header::AUTHORIZATION, format!("Bearer {plaintext}"))
                .body(Body::from(check_body.to_string()))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(check_resp.status(), StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn revoke_missing_workspace_key_returns_404() {
    let app = build_app(Some(AuthConfig::new("sk-internal")));
    let user_id = Uuid::new_v4();
    let workspace_id = create_workspace_for_user(app.clone(), user_id, "Runtime Workspace").await;
    let resp = app
        .oneshot(revoke_api_keys_request_with_user(
            "sk-internal",
            &workspace_id,
            &["apk_missing"],
            user_id,
        ))
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn internal_bearer_without_forwarded_user_cannot_issue_workspace_key() {
    let app = build_app(Some(AuthConfig::new("sk-internal")));
    let resp = app
        .oneshot(create_api_key_request(
            "sk-internal",
            "ws_runtime",
            "SDK integration",
        ))
        .await
        .unwrap();

    assert_eq!(resp.status(), StatusCode::FORBIDDEN);
}

#[tokio::test]
async fn user_jwt_can_manage_keys_only_for_owned_workspace() {
    let (app, signer) = build_app_with_jwt();
    let owner_id = Uuid::new_v4();
    let workspace_id = create_workspace_for_user(app.clone(), owner_id, "JWT Workspace").await;
    let owner_token = signer.mint(owner_id, "owner@example.com").unwrap();

    let create_resp = app
        .clone()
        .oneshot(create_api_key_request(
            &owner_token,
            &workspace_id,
            "SDK integration",
        ))
        .await
        .unwrap();
    assert_eq!(create_resp.status(), StatusCode::CREATED);
    let created = read_body(create_resp).await;
    let key_id = created["api_key"]["id"].as_str().unwrap();

    let list_resp = app
        .clone()
        .oneshot(list_api_keys_request(&owner_token, &workspace_id))
        .await
        .unwrap();
    assert_eq!(list_resp.status(), StatusCode::OK);

    let revoke_resp = app
        .clone()
        .oneshot(revoke_api_keys_request(
            &owner_token,
            &workspace_id,
            &[key_id],
        ))
        .await
        .unwrap();
    assert_eq!(revoke_resp.status(), StatusCode::OK);

    let outsider_id = Uuid::new_v4();
    let outsider_token = signer.mint(outsider_id, "outsider@example.com").unwrap();
    let denied_resp = app
        .oneshot(create_api_key_request(
            &outsider_token,
            &workspace_id,
            "stolen workspace",
        ))
        .await
        .unwrap();
    assert_eq!(denied_resp.status(), StatusCode::FORBIDDEN);
}

#[tokio::test]
async fn workspace_runtime_key_cannot_manage_api_keys() {
    let app = build_app(Some(AuthConfig::new("sk-internal")));
    let user_id = Uuid::new_v4();
    let workspace_id = create_workspace_for_user(app.clone(), user_id, "Runtime Workspace").await;

    let create_resp = app
        .clone()
        .oneshot(create_api_key_request_with_user(
            "sk-internal",
            &workspace_id,
            "SDK integration",
            user_id,
        ))
        .await
        .unwrap();
    assert_eq!(create_resp.status(), StatusCode::CREATED);
    let created = read_body(create_resp).await;
    let plaintext = created["plaintext_key"].as_str().unwrap();
    let key_id = created["api_key"]["id"].as_str().unwrap();

    let create_with_runtime_key = app
        .clone()
        .oneshot(create_api_key_request(
            plaintext,
            &workspace_id,
            "recursive key",
        ))
        .await
        .unwrap();
    assert_eq!(create_with_runtime_key.status(), StatusCode::FORBIDDEN);

    let list_with_runtime_key = app
        .clone()
        .oneshot(list_api_keys_request(plaintext, &workspace_id))
        .await
        .unwrap();
    assert_eq!(list_with_runtime_key.status(), StatusCode::FORBIDDEN);

    let revoke_with_runtime_key = app
        .oneshot(revoke_api_keys_request(plaintext, &workspace_id, &[key_id]))
        .await
        .unwrap();
    assert_eq!(revoke_with_runtime_key.status(), StatusCode::FORBIDDEN);
}

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
    // Local-dev / test mode: auth=None means /v1/check accepts any
    // request (or no Authorization header at all).
    let app = build_app(None);
    let resp = app.oneshot(check_request(None)).await.unwrap();
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
            .uri("/v1/check")
            .header(header::CONTENT_TYPE, "application/json")
            .header(header::AUTHORIZATION, header_value)
            .body(Body::from(
                r#"{"agent_id":"a","channel":"chat","input":"x","proposed_output":"y"}"#,
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
