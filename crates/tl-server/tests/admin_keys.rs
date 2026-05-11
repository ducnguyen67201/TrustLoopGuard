//! Integration tests for the admin API key endpoints. Requires Docker
//! (testcontainers Postgres) and the `postgres` feature:
//!
//!   cargo test -p tl-server --test admin_keys --features postgres

#![cfg(feature = "postgres")]

use std::sync::Arc;

use axum::{
    body::Body,
    http::{header, Request, StatusCode},
};
use http_body_util::BodyExt;
use serde_json::json;
use sqlx::postgres::PgPoolOptions;
use testcontainers::runners::AsyncRunner;
use testcontainers_modules::postgres::Postgres as PostgresImage;
use tl_engine::Engine;
use tl_server::{admin, memory_app_state, router, AdminConfig, AppState};
use tl_storage::{hash_plaintext, migrate_postgres, ApiKeyRepo};
use tower::ServiceExt;

const ADMIN_KEY: &str = "test-admin-secret";

struct Harness {
    app: axum::Router,
    repo: ApiKeyRepo,
    _container: testcontainers::ContainerAsync<PostgresImage>,
}

async fn build_harness(admin: Option<Arc<AdminConfig>>) -> Harness {
    let container = PostgresImage::default()
        .start()
        .await
        .expect("postgres container");
    let host = container.get_host().await.expect("host");
    let port = container
        .get_host_port_ipv4(5432)
        .await
        .expect("postgres port");
    let url = format!("postgres://postgres:postgres@{host}:{port}/postgres");
    let pool = PgPoolOptions::new()
        .max_connections(4)
        .connect(&url)
        .await
        .expect("connect");
    migrate_postgres(&pool).await.expect("migrate");

    let repo = ApiKeyRepo::new(pool);

    // memory_app_state gives us an AppState shell; we splice the repo
    // in so the admin router has a Postgres-backed state.
    let mut state: AppState = memory_app_state(Arc::new(Engine::empty()));
    state.api_key_repo = Some(repo.clone());

    let app = router(state, None, admin);
    Harness {
        app,
        repo,
        _container: container,
    }
}

fn create_req(token: Option<&str>, body: serde_json::Value) -> Request<Body> {
    let mut b = Request::builder()
        .method("POST")
        .uri("/v1/admin/keys")
        .header(header::CONTENT_TYPE, "application/json");
    if let Some(t) = token {
        b = b.header(header::AUTHORIZATION, format!("Bearer {t}"));
    }
    b.body(Body::from(body.to_string())).unwrap()
}

fn list_req(token: Option<&str>, user_id: &str) -> Request<Body> {
    let mut b = Request::builder()
        .method("GET")
        .uri(format!("/v1/admin/keys?user_id={user_id}"));
    if let Some(t) = token {
        b = b.header(header::AUTHORIZATION, format!("Bearer {t}"));
    }
    b.body(Body::empty()).unwrap()
}

fn delete_req(token: Option<&str>, id: &str, user_id: &str) -> Request<Body> {
    let mut b = Request::builder()
        .method("DELETE")
        .uri(format!("/v1/admin/keys/{id}?user_id={user_id}"));
    if let Some(t) = token {
        b = b.header(header::AUTHORIZATION, format!("Bearer {t}"));
    }
    b.body(Body::empty()).unwrap()
}

async fn read_json(resp: axum::response::Response) -> serde_json::Value {
    let bytes = resp.into_body().collect().await.unwrap().to_bytes();
    if bytes.is_empty() {
        serde_json::Value::Null
    } else {
        serde_json::from_slice(&bytes).unwrap_or(serde_json::Value::Null)
    }
}

#[tokio::test]
async fn create_with_valid_bearer_returns_plaintext() {
    let h = build_harness(Some(AdminConfig::new(ADMIN_KEY))).await;
    let resp = h
        .app
        .clone()
        .oneshot(create_req(
            Some(ADMIN_KEY),
            json!({ "user_id": "u_1", "name": "production" }),
        ))
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::CREATED);
    let body = read_json(resp).await;
    let plaintext = body["plaintext"].as_str().expect("plaintext field");
    assert!(plaintext.starts_with("tlg_"), "got {plaintext}");
    assert_eq!(body["user_id"], "u_1");
    assert_eq!(body["name"], "production");

    // The plaintext, hashed, must lookup to the same record.
    let hash = hash_plaintext(plaintext);
    let row = h
        .repo
        .lookup_by_hash(&hash)
        .await
        .expect("lookup")
        .expect("row present");
    assert_eq!(row.user_id, "u_1");
}

#[tokio::test]
async fn create_without_bearer_returns_401() {
    let h = build_harness(Some(AdminConfig::new(ADMIN_KEY))).await;
    let resp = h
        .app
        .oneshot(create_req(None, json!({ "user_id": "u_1", "name": "p" })))
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn create_with_wrong_bearer_returns_401() {
    let h = build_harness(Some(AdminConfig::new(ADMIN_KEY))).await;
    let resp = h
        .app
        .oneshot(create_req(
            Some("wrong"),
            json!({ "user_id": "u_1", "name": "p" }),
        ))
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn list_scopes_by_user_id() {
    let h = build_harness(Some(AdminConfig::new(ADMIN_KEY))).await;
    // Two keys for u_1, one for u_2.
    for name in ["one", "two"] {
        let resp = h
            .app
            .clone()
            .oneshot(create_req(
                Some(ADMIN_KEY),
                json!({ "user_id": "u_1", "name": name }),
            ))
            .await
            .unwrap();
        assert_eq!(resp.status(), StatusCode::CREATED);
    }
    let resp = h
        .app
        .clone()
        .oneshot(create_req(
            Some(ADMIN_KEY),
            json!({ "user_id": "u_2", "name": "other" }),
        ))
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::CREATED);

    let resp = h
        .app
        .clone()
        .oneshot(list_req(Some(ADMIN_KEY), "u_1"))
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let body = read_json(resp).await;
    let keys = body["keys"].as_array().expect("keys array");
    assert_eq!(keys.len(), 2);

    let resp = h
        .app
        .oneshot(list_req(Some(ADMIN_KEY), "u_nobody"))
        .await
        .unwrap();
    let body = read_json(resp).await;
    assert_eq!(body["keys"].as_array().unwrap().len(), 0);
}

#[tokio::test]
async fn delete_is_204_then_404() {
    let h = build_harness(Some(AdminConfig::new(ADMIN_KEY))).await;
    let resp = h
        .app
        .clone()
        .oneshot(create_req(
            Some(ADMIN_KEY),
            json!({ "user_id": "u_1", "name": "kill" }),
        ))
        .await
        .unwrap();
    let body = read_json(resp).await;
    let id = body["id"].as_str().unwrap().to_string();

    let resp = h
        .app
        .clone()
        .oneshot(delete_req(Some(ADMIN_KEY), &id, "u_1"))
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::NO_CONTENT);

    // Second delete on the same id is a miss now (already revoked).
    let resp = h
        .app
        .clone()
        .oneshot(delete_req(Some(ADMIN_KEY), &id, "u_1"))
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::NOT_FOUND);

    // The row should still appear in list, with revoked_at populated.
    let resp = h
        .app
        .oneshot(list_req(Some(ADMIN_KEY), "u_1"))
        .await
        .unwrap();
    let body = read_json(resp).await;
    let keys = body["keys"].as_array().expect("keys");
    assert_eq!(keys.len(), 1);
    assert!(!keys[0]["revoked_at"].is_null());
}

#[tokio::test]
async fn create_with_no_admin_config_is_open() {
    // Mirrors the dev escape hatch: when TL_ADMIN_KEY is unset, the
    // server logs a warning at boot and the routes accept any request.
    let h = build_harness(None).await;
    let resp = h
        .app
        .oneshot(create_req(None, json!({ "user_id": "u_1", "name": "p" })))
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::CREATED);
}

#[tokio::test]
async fn create_rejects_empty_inputs() {
    let h = build_harness(Some(AdminConfig::new(ADMIN_KEY))).await;
    let resp = h
        .app
        .oneshot(create_req(
            Some(ADMIN_KEY),
            json!({ "user_id": "", "name": "p" }),
        ))
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::BAD_REQUEST);
}

// Silence unused warning when admin module re-export is not directly
// referenced.
#[allow(dead_code)]
fn _ref_admin_types(_v: admin::ApiKeyView) {}
