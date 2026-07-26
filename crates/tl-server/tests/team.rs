//! Component tests for user-scoped workspace team endpoints.

use std::sync::Arc;

use axum::{
    body::Body,
    http::{header, Request, StatusCode},
};
use http_body_util::BodyExt;
use tl_core::{ApiError, WorkspaceRole};
use tl_engine::Engine;
use tl_server::{
    jwt::JwtSigner, memory_app_state, router, AuthConfig, MemoryTeamStore, MemoryUserStore,
    TeamStore,
};
use tower::ServiceExt;
use uuid::Uuid;

struct TeamFixture {
    app: axum::Router,
    store: Arc<dyn TeamStore>,
    workspace_id: String,
    owner_id: Uuid,
    owner_token: String,
    admin_token: String,
    editor_token: String,
    viewer_token: String,
    outsider_token: String,
    platform_admin_token: String,
}

async fn team_fixture() -> TeamFixture {
    let mut state = memory_app_state(Arc::new(Engine::empty()));
    let user_store = Arc::new(MemoryUserStore::new());
    let owner = user_store
        .create_approved_for_tests("owner@example.com")
        .await
        .expect("owner");
    let admin = user_store
        .create_approved_for_tests("admin@example.com")
        .await
        .expect("admin");
    let editor = user_store
        .create_approved_for_tests("editor@example.com")
        .await
        .expect("editor");
    let viewer = user_store
        .create_approved_for_tests("viewer@example.com")
        .await
        .expect("viewer");
    let outsider = user_store
        .create_approved_for_tests("outsider@example.com")
        .await
        .expect("outsider");
    let platform_admin = user_store
        .create_approved_for_tests("platform-admin@example.com")
        .await
        .expect("platform admin");

    let store = Arc::new(MemoryTeamStore::new());
    state.team_store = store.clone();
    let workspace = store
        .create_workspace(owner.id, "Delete Team")
        .await
        .expect("create workspace");
    store
        .set_platform_admin_for_tests(platform_admin.id, true)
        .await;
    for (user_id, email, role) in [
        (admin.id, admin.username.as_str(), WorkspaceRole::Admin),
        (editor.id, editor.username.as_str(), WorkspaceRole::Editor),
        (viewer.id, viewer.username.as_str(), WorkspaceRole::Viewer),
    ] {
        store
            .add_member_or_invite(&workspace.id, email, role, Some(owner.id))
            .await
            .expect("invite member");
        assert_eq!(
            store
                .accept_pending_invites_for_email(email, user_id)
                .await
                .expect("accept member invite"),
            1
        );
    }
    store
        .add_member_or_invite(
            &workspace.id,
            "pending@example.com",
            WorkspaceRole::Viewer,
            Some(owner.id),
        )
        .await
        .expect("pending invite");

    let signer = JwtSigner::new("test-secret-test-secret-test-secret-12");
    let owner_token = signer.mint(owner.id, &owner.username).expect("owner token");
    let admin_token = signer.mint(admin.id, &admin.username).expect("admin token");
    let editor_token = signer
        .mint(editor.id, &editor.username)
        .expect("editor token");
    let viewer_token = signer
        .mint(viewer.id, &viewer.username)
        .expect("viewer token");
    let outsider_token = signer
        .mint(outsider.id, &outsider.username)
        .expect("outsider token");
    let platform_admin_token = signer
        .mint(platform_admin.id, &platform_admin.username)
        .expect("platform admin token");

    state.user_store = user_store;
    state.jwt_signer = Some(signer);
    let app = router(state, Some(AuthConfig::new("sk-internal")), [0u8; 32]);

    TeamFixture {
        app,
        store,
        workspace_id: workspace.id,
        owner_id: owner.id,
        owner_token,
        admin_token,
        editor_token,
        viewer_token,
        outsider_token,
        platform_admin_token,
    }
}

fn delete_workspace_request(workspace_id: &str, token: Option<&str>) -> Request<Body> {
    let mut request = Request::builder()
        .method("DELETE")
        .uri(format!("/v1/team/my-workspaces/{workspace_id}"));
    if let Some(token) = token {
        request = request.header(header::AUTHORIZATION, format!("Bearer {token}"));
    }
    request.body(Body::empty()).expect("delete request")
}

fn list_workspaces_request(token: &str) -> Request<Body> {
    Request::builder()
        .method("GET")
        .uri("/v1/team/my-workspaces")
        .header(header::AUTHORIZATION, format!("Bearer {token}"))
        .body(Body::empty())
        .expect("list request")
}

async fn response_json(response: axum::response::Response) -> serde_json::Value {
    let bytes = response
        .into_body()
        .collect()
        .await
        .expect("response body")
        .to_bytes();
    serde_json::from_slice(&bytes).expect("json response")
}

#[tokio::test]
async fn owner_deletes_workspace_and_access_disappears_for_every_member() {
    let fixture = team_fixture().await;
    let response = fixture
        .app
        .clone()
        .oneshot(delete_workspace_request(
            &fixture.workspace_id,
            Some(&fixture.owner_token),
        ))
        .await
        .expect("delete response");
    assert_eq!(response.status(), StatusCode::NO_CONTENT);
    let body = response
        .into_body()
        .collect()
        .await
        .expect("delete body")
        .to_bytes();
    assert!(body.is_empty());

    for token in [
        &fixture.owner_token,
        &fixture.admin_token,
        &fixture.editor_token,
        &fixture.viewer_token,
    ] {
        let response = fixture
            .app
            .clone()
            .oneshot(list_workspaces_request(token))
            .await
            .expect("list response");
        assert_eq!(response.status(), StatusCode::OK);
        let body = response_json(response).await;
        assert_eq!(body["workspaces"].as_array().map(Vec::len), Some(0));
    }
    assert!(fixture
        .store
        .list_members(&fixture.workspace_id)
        .await
        .expect("members after delete")
        .is_empty());
    assert!(fixture
        .store
        .list_pending_invites(&fixture.workspace_id)
        .await
        .expect("invites after delete")
        .is_empty());

    let repeated = fixture
        .app
        .oneshot(delete_workspace_request(
            &fixture.workspace_id,
            Some(&fixture.owner_token),
        ))
        .await
        .expect("repeat delete response");
    assert_eq!(repeated.status(), StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn non_owners_and_outsider_cannot_delete_workspace() {
    let fixture = team_fixture().await;
    for token in [
        &fixture.admin_token,
        &fixture.editor_token,
        &fixture.viewer_token,
        &fixture.outsider_token,
        &fixture.platform_admin_token,
    ] {
        let response = fixture
            .app
            .clone()
            .oneshot(delete_workspace_request(&fixture.workspace_id, Some(token)))
            .await
            .expect("forbidden delete response");
        assert_eq!(response.status(), StatusCode::FORBIDDEN);
        let error: ApiError =
            serde_json::from_value(response_json(response).await).expect("api error");
        assert_eq!(error.code, tl_core::ApiErrorCode::Forbidden);
    }

    assert_eq!(
        fixture
            .store
            .list_workspaces_for_user(fixture.owner_id)
            .await
            .expect("owner workspaces")
            .len(),
        1
    );
    assert_eq!(
        fixture
            .store
            .list_pending_invites(&fixture.workspace_id)
            .await
            .expect("pending invite retained")
            .len(),
        1
    );

    let unknown = fixture
        .app
        .oneshot(delete_workspace_request(
            "ws_missing",
            Some(&fixture.owner_token),
        ))
        .await
        .expect("unknown delete response");
    assert_eq!(unknown.status(), StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn only_platform_admins_receive_cross_workspace_access() {
    let fixture = team_fixture().await;

    let outsider = fixture
        .app
        .clone()
        .oneshot(list_workspaces_request(&fixture.outsider_token))
        .await
        .expect("outsider list response");
    assert_eq!(outsider.status(), StatusCode::OK);
    let outsider_body = response_json(outsider).await;
    assert_eq!(outsider_body["is_platform_admin"], false);
    assert_eq!(
        outsider_body["workspaces"].as_array().map(Vec::len),
        Some(0)
    );

    let platform_admin = fixture
        .app
        .oneshot(list_workspaces_request(&fixture.platform_admin_token))
        .await
        .expect("platform admin list response");
    assert_eq!(platform_admin.status(), StatusCode::OK);
    let platform_admin_body = response_json(platform_admin).await;
    assert_eq!(platform_admin_body["is_platform_admin"], true);
    assert_eq!(
        platform_admin_body["workspaces"].as_array().map(Vec::len),
        Some(1)
    );
    assert_eq!(
        platform_admin_body["workspaces"][0]["id"],
        fixture.workspace_id
    );
    assert_eq!(platform_admin_body["workspaces"][0]["role"], "admin");
}

#[tokio::test]
async fn delete_workspace_requires_authentication_and_user_identity() {
    let fixture = team_fixture().await;
    let missing_bearer = fixture
        .app
        .clone()
        .oneshot(delete_workspace_request(&fixture.workspace_id, None))
        .await
        .expect("missing bearer response");
    assert_eq!(missing_bearer.status(), StatusCode::UNAUTHORIZED);

    let missing_identity = fixture
        .app
        .oneshot(delete_workspace_request(
            &fixture.workspace_id,
            Some("sk-internal"),
        ))
        .await
        .expect("missing identity response");
    assert_eq!(missing_identity.status(), StatusCode::BAD_REQUEST);
}
