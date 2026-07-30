//! Component tests for knowledge-source mutation authorization.

use std::sync::Arc;

use axum::{
    body::Body,
    http::{header, Request, StatusCode},
};
use tl_core::WorkspaceRole;
use tl_engine::Engine;
use tl_server::knowledge_sources::KnowledgeStore;
use tl_server::{
    memory_app_state, router, AuthConfig, MemoryTeamStore, MemoryUserStore, TeamStore,
};
use tower::ServiceExt;
use uuid::Uuid;

struct Fixture {
    app: axum::Router,
    knowledge_store: Arc<dyn KnowledgeStore>,
    workspace_id: String,
    owner_id: Uuid,
    admin_id: Uuid,
    viewer_id: Uuid,
}

async fn fixture() -> Fixture {
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
    let viewer = user_store
        .create_approved_for_tests("viewer@example.com")
        .await
        .expect("viewer");
    let team_store = Arc::new(MemoryTeamStore::new());
    let workspace = team_store
        .create_workspace(owner.id, "Knowledge Workspace")
        .await
        .expect("workspace");
    for (user, role) in [
        (&admin, WorkspaceRole::Admin),
        (&viewer, WorkspaceRole::Viewer),
    ] {
        team_store
            .add_member_or_invite(&workspace.id, &user.username, role, Some(owner.id))
            .await
            .expect("invite member");
        team_store
            .accept_pending_invites_for_email(&user.username, user.id)
            .await
            .expect("accept invite");
    }

    let knowledge_store = state.knowledge_store.clone();
    state.user_store = user_store;
    state.team_store = team_store;
    let app = router(state, Some(AuthConfig::new("sk-internal")), [0u8; 32]);

    Fixture {
        app,
        knowledge_store,
        workspace_id: workspace.id,
        owner_id: owner.id,
        admin_id: admin.id,
        viewer_id: viewer.id,
    }
}

fn create_request(workspace_id: &str, user_id: Uuid, title: &str) -> Request<Body> {
    Request::builder()
        .method("POST")
        .uri("/v1/knowledge-sources")
        .header(header::AUTHORIZATION, "Bearer sk-internal")
        .header(header::CONTENT_TYPE, "application/json")
        .header("x-tlg-workspace-id", workspace_id)
        .header("x-tlg-user-id", user_id.to_string())
        .body(Body::from(
            serde_json::json!({
                "title": title,
                "kind": "note",
                "notes": "Trusted operating instructions",
            })
            .to_string(),
        ))
        .expect("knowledge request")
}

#[tokio::test]
async fn viewer_cannot_create_knowledge_source() {
    let fixture = fixture().await;
    let response = fixture
        .app
        .clone()
        .oneshot(create_request(
            &fixture.workspace_id,
            fixture.viewer_id,
            "Viewer source",
        ))
        .await
        .expect("viewer response");
    assert_eq!(response.status(), StatusCode::FORBIDDEN);
    assert!(fixture
        .knowledge_store
        .list(&fixture.workspace_id)
        .await
        .expect("knowledge list")
        .is_empty());
}

#[tokio::test]
async fn owner_and_admin_can_create_knowledge_sources() {
    let fixture = fixture().await;
    for (user_id, title) in [
        (fixture.owner_id, "Owner source"),
        (fixture.admin_id, "Admin source"),
    ] {
        let response = fixture
            .app
            .clone()
            .oneshot(create_request(&fixture.workspace_id, user_id, title))
            .await
            .expect("authorized response");
        assert_eq!(response.status(), StatusCode::CREATED);
    }
    assert_eq!(
        fixture
            .knowledge_store
            .list(&fixture.workspace_id)
            .await
            .expect("knowledge list")
            .len(),
        2
    );
}
