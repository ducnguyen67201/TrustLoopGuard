//! Integration tests for `/v1/check` accepting both the static
//! `TL_API_KEY` and per-user keys minted via the admin surface.
//! Requires Docker (testcontainers Postgres) and the `postgres` feature:
//!
//!   cargo test -p tl-server --test check_with_user_keys --features postgres

#![cfg(feature = "postgres")]

use std::sync::Arc;
use std::time::Duration;

use axum::{
    body::Body,
    http::{header, Request, StatusCode},
    middleware::from_fn_with_state,
    routing::post,
    Extension, Router,
};
use http_body_util::BodyExt;
use serde_json::json;
use sqlx::postgres::PgPoolOptions;
use testcontainers::runners::AsyncRunner;
use testcontainers_modules::postgres::Postgres as PostgresImage;
use tl_engine::Engine;
use tl_server::{
    auth::{require_auth, AuthenticatedUser},
    memory_app_state, router, AppState, AuthConfig, AuthLayer,
};
use tl_storage::{migrate_postgres, ApiKeyRepo};
use tower::ServiceExt;

const STATIC_KEY: &str = "static-sk-test";

struct Harness {
    app: axum::Router,
    repo: ApiKeyRepo,
    _container: testcontainers::ContainerAsync<PostgresImage>,
}

async fn boot_postgres() -> (sqlx::PgPool, testcontainers::ContainerAsync<PostgresImage>) {
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
    (pool, container)
}

/// Standard harness: AppState with `api_key_repo` wired so the auth
/// layer accepts both the static key and DB-backed user keys.
async fn build_harness(static_auth: Option<Arc<AuthConfig>>) -> Harness {
    let (pool, container) = boot_postgres().await;
    let repo = ApiKeyRepo::new(pool);

    let mut state: AppState = memory_app_state(Arc::new(Engine::empty()));
    state.api_key_repo = Some(repo.clone());

    let app = router(state, static_auth, None);
    Harness {
        app,
        repo,
        _container: container,
    }
}

fn check_req(token: Option<&str>) -> Request<Body> {
    let mut b = Request::builder()
        .method("POST")
        .uri("/v1/check")
        .header(header::CONTENT_TYPE, "application/json");
    if let Some(t) = token {
        b = b.header(header::AUTHORIZATION, format!("Bearer {t}"));
    }
    let body = json!({
        "agent_id": "agent-x",
        "channel": "chat",
        "input": "hi",
        "proposed_output": "hello",
    });
    b.body(Body::from(body.to_string())).unwrap()
}

async fn read_status(resp: axum::response::Response) -> StatusCode {
    let s = resp.status();
    let _ = resp.into_body().collect().await;
    s
}

#[tokio::test]
async fn static_key_still_authenticates() {
    let h = build_harness(Some(AuthConfig::new(STATIC_KEY))).await;
    let resp = h.app.oneshot(check_req(Some(STATIC_KEY))).await.unwrap();
    assert_eq!(read_status(resp).await, StatusCode::OK);
}

#[tokio::test]
async fn minted_user_key_authenticates() {
    let h = build_harness(Some(AuthConfig::new(STATIC_KEY))).await;
    let minted = h.repo.create("u_1", "production").await.expect("mint");

    let resp = h
        .app
        .oneshot(check_req(Some(&minted.plaintext)))
        .await
        .unwrap();
    assert_eq!(read_status(resp).await, StatusCode::OK);
}

#[tokio::test]
async fn unknown_token_is_rejected() {
    let h = build_harness(Some(AuthConfig::new(STATIC_KEY))).await;
    let resp = h
        .app
        .oneshot(check_req(Some("tlg_bogus_token_value")))
        .await
        .unwrap();
    assert_eq!(read_status(resp).await, StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn missing_bearer_is_rejected() {
    let h = build_harness(Some(AuthConfig::new(STATIC_KEY))).await;
    let resp = h.app.oneshot(check_req(None)).await.unwrap();
    assert_eq!(read_status(resp).await, StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn user_key_authenticates_without_static_key_configured() {
    // No static key at all — only DB-backed per-user keys.
    let h = build_harness(None).await;
    let minted = h.repo.create("u_2", "personal").await.expect("mint");

    let resp = h
        .app
        .clone()
        .oneshot(check_req(Some(&minted.plaintext)))
        .await
        .unwrap();
    assert_eq!(read_status(resp).await, StatusCode::OK);

    let resp = h.app.oneshot(check_req(Some("wrong"))).await.unwrap();
    assert_eq!(read_status(resp).await, StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn revoked_key_is_rejected_after_negative_ttl_expires() {
    // Build an AuthLayer directly with tiny TTLs so revocation visibility
    // can be asserted without sleeping for the production 30s.
    let (pool, _container) = boot_postgres().await;
    let repo = ApiKeyRepo::new(pool);

    let minted = repo.create("u_3", "ephemeral").await.expect("mint");
    let key_id = minted.record.id;

    let layer = Arc::new(AuthLayer::with_repo_and_ttls(
        Some(AuthConfig::new(STATIC_KEY)),
        repo.clone(),
        Duration::from_millis(50),
        Duration::from_millis(50),
    ));

    let state: AppState = memory_app_state(Arc::new(Engine::empty()));
    let app = Router::new()
        .route("/v1/check", post(tl_server::check))
        .with_state(state)
        .layer(from_fn_with_state(layer, require_auth));

    // Authenticated before revocation.
    let resp = app
        .clone()
        .oneshot(check_req(Some(&minted.plaintext)))
        .await
        .unwrap();
    assert_eq!(read_status(resp).await, StatusCode::OK);

    // Revoke directly.
    assert!(repo.revoke(key_id, "u_3").await.expect("revoke"));

    // Cache still holds the positive entry; wait for it to expire.
    tokio::time::sleep(Duration::from_millis(120)).await;

    let resp = app
        .oneshot(check_req(Some(&minted.plaintext)))
        .await
        .unwrap();
    assert_eq!(read_status(resp).await, StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn authenticated_user_extension_is_populated() {
    // Spin up a minimal router with a custom downstream handler that
    // echoes the AuthenticatedUser extension — this is the only way to
    // observe the extension from outside the middleware.
    let (pool, _container) = boot_postgres().await;
    let repo = ApiKeyRepo::new(pool);
    let minted = repo.create("u_echo", "echo").await.expect("mint");

    let layer = Arc::new(AuthLayer::with_repo(
        Some(AuthConfig::new(STATIC_KEY)),
        repo.clone(),
    ));

    async fn echo(Extension(user): Extension<AuthenticatedUser>) -> String {
        user.user_id
    }

    let app = Router::new()
        .route("/echo", post(echo))
        .layer(from_fn_with_state(layer, require_auth));

    let req = Request::builder()
        .method("POST")
        .uri("/echo")
        .header(
            header::AUTHORIZATION,
            format!("Bearer {}", minted.plaintext),
        )
        .body(Body::empty())
        .unwrap();

    let resp = app.oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let bytes = resp.into_body().collect().await.unwrap().to_bytes();
    assert_eq!(std::str::from_utf8(&bytes).unwrap(), "u_echo");
}
