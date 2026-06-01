use std::sync::Arc;

use axum::{extract::State, http::StatusCode, Json};
use tl_core::{AuthRequest, ChangePasswordRequest};

use super::password::{hash_password, verify_password};
use super::validation::{validate_password_hex, validate_username};
use super::*;

const VALID_HEX: &str = "9f86d081884c7d659a2feaa0c55ad015a3bf4f1b2b0b822cd15d6c15b0f00a08"; // sha256("test")
const OTHER_HEX: &str = "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855"; // sha256("")

fn test_state(password_auth_enabled: bool) -> AuthUserState {
    AuthUserState {
        store: Arc::new(MemoryUserStore::new()),
        password_auth_enabled,
        jwt_signer: None,
    }
}

#[test]
fn hash_roundtrip_matches() {
    let phc = hash_password(VALID_HEX).unwrap();
    assert!(phc.starts_with("$argon2id$"));
    assert!(verify_password(VALID_HEX, &phc).unwrap());
}

#[test]
fn verify_rejects_wrong_password() {
    let phc = hash_password(VALID_HEX).unwrap();
    assert!(!verify_password(OTHER_HEX, &phc).unwrap());
}

#[test]
fn validate_username_rules() {
    assert!(validate_username("ab").is_err());
    assert!(validate_username("a".repeat(65).as_str()).is_err());
    assert!(validate_username("bad space").is_err());
    assert!(validate_username("bad!char").is_err());
    assert!(validate_username("good-user.1").is_ok());
    assert!(validate_username("Alice_2").is_ok());
}

#[test]
fn validate_password_rules() {
    assert!(validate_password_hex(VALID_HEX).is_ok());
    assert!(validate_password_hex("short").is_err());
    assert!(validate_password_hex(&"z".repeat(64)).is_err());
    assert!(validate_password_hex(&VALID_HEX.to_ascii_uppercase()).is_ok());
}

#[tokio::test]
async fn memory_store_create_and_find_case_insensitive() {
    let store = MemoryUserStore::new();
    let record = store.create("Alice", "phc").await.unwrap();
    let found = store.find_by_username("alice").await.unwrap();
    assert_eq!(record.id, found.id);
}

#[tokio::test]
async fn memory_store_conflict_on_duplicate() {
    let store = MemoryUserStore::new();
    store.create("alice", "phc").await.unwrap();
    let err = store.create("ALICE", "phc").await.unwrap_err();
    assert!(matches!(err, UserStoreError::Conflict));
}

#[tokio::test]
async fn signup_then_login_round_trip() {
    let state = test_state(true);
    let req = AuthRequest {
        username: "alice".into(),
        password: VALID_HEX.into(),
    };

    let resp = signup(State(state.clone()), Json(req)).await;
    assert_eq!(resp.status(), StatusCode::CREATED);

    let login_resp = login(
        State(state),
        Json(AuthRequest {
            username: "ALICE".into(),
            password: VALID_HEX.into(),
        }),
    )
    .await;
    assert_eq!(login_resp.status(), StatusCode::OK);
}

#[tokio::test]
async fn login_wrong_password_is_401() {
    let state = test_state(true);
    state
        .store
        .create("alice", &hash_password(VALID_HEX).unwrap())
        .await
        .unwrap();

    let resp = login(
        State(state),
        Json(AuthRequest {
            username: "alice".into(),
            password: "0".repeat(64),
        }),
    )
    .await;
    assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn login_unknown_user_is_401() {
    let state = test_state(true);
    let resp = login(
        State(state),
        Json(AuthRequest {
            username: "ghost".into(),
            password: VALID_HEX.into(),
        }),
    )
    .await;
    assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn change_password_then_login_with_new_password() {
    let state = test_state(true);
    signup(
        State(state.clone()),
        Json(AuthRequest {
            username: "alice".into(),
            password: VALID_HEX.into(),
        }),
    )
    .await;

    let resp = change_password(
        State(state.clone()),
        Json(ChangePasswordRequest {
            username: "alice".into(),
            current_password: VALID_HEX.into(),
            new_password: OTHER_HEX.into(),
        }),
    )
    .await;
    assert_eq!(resp.status(), StatusCode::OK);

    let old = login(
        State(state.clone()),
        Json(AuthRequest {
            username: "alice".into(),
            password: VALID_HEX.into(),
        }),
    )
    .await;
    assert_eq!(old.status(), StatusCode::UNAUTHORIZED);

    let new = login(
        State(state),
        Json(AuthRequest {
            username: "alice".into(),
            password: OTHER_HEX.into(),
        }),
    )
    .await;
    assert_eq!(new.status(), StatusCode::OK);
}

#[tokio::test]
async fn change_password_wrong_current_is_401() {
    let state = test_state(true);
    signup(
        State(state.clone()),
        Json(AuthRequest {
            username: "alice".into(),
            password: VALID_HEX.into(),
        }),
    )
    .await;
    let resp = change_password(
        State(state),
        Json(ChangePasswordRequest {
            username: "alice".into(),
            current_password: OTHER_HEX.into(),
            new_password: "1".repeat(64),
        }),
    )
    .await;
    assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn change_password_same_as_current_is_400() {
    let state = test_state(true);
    signup(
        State(state.clone()),
        Json(AuthRequest {
            username: "alice".into(),
            password: VALID_HEX.into(),
        }),
    )
    .await;
    let resp = change_password(
        State(state),
        Json(ChangePasswordRequest {
            username: "alice".into(),
            current_password: VALID_HEX.into(),
            new_password: VALID_HEX.into(),
        }),
    )
    .await;
    assert_eq!(resp.status(), StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn password_auth_endpoints_are_not_found_when_disabled() {
    let state = test_state(false);

    let signup_resp = signup(
        State(state.clone()),
        Json(AuthRequest {
            username: "alice".into(),
            password: VALID_HEX.into(),
        }),
    )
    .await;
    assert_eq!(signup_resp.status(), StatusCode::NOT_FOUND);

    let login_resp = login(
        State(state.clone()),
        Json(AuthRequest {
            username: "alice".into(),
            password: VALID_HEX.into(),
        }),
    )
    .await;
    assert_eq!(login_resp.status(), StatusCode::NOT_FOUND);

    let change_resp = change_password(
        State(state),
        Json(ChangePasswordRequest {
            username: "alice".into(),
            current_password: VALID_HEX.into(),
            new_password: OTHER_HEX.into(),
        }),
    )
    .await;
    assert_eq!(change_resp.status(), StatusCode::NOT_FOUND);
}
