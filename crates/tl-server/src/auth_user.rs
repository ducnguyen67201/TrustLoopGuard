//! Username/password authentication for self-hosted deployments.
//!
//! Companion to [`crate::auth`] (the static bearer-token middleware
//! that protects SDK calls). This module adds *user* accounts so
//! self-hosters who can't configure the GitHub/Google OAuth providers
//! in `apps/web` still have a way to sign in.
//!
//! Endpoints (all public — no `Authorization` header required):
//! - `POST /v1/auth/signup`   — create an account
//! - `POST /v1/auth/login`    — verify credentials
//! - `POST /v1/auth/password` — change password (requires current password)
//!
//! Password handling:
//! - The client SHA-256-hexes the password before sending. That hex
//!   is what we hash with argon2id. SHA-256 alone is **not** safe at
//!   rest — argon2id provides the KDF.
//! - Hashes are stored as argon2's PHC string (`$argon2id$...`) so
//!   parameters and salt travel with the hash.
//!
//! Session/JWT issuance is intentionally **not** in scope, full stop.
//! `tl-server`'s bearer middleware accepts a single shared
//! `TL_API_KEY` (internal/web) and, eventually, per-workspace API
//! keys (SDKs). User identity travels as `X-TLG-User-Id` +
//! `X-TLG-User-Email` headers forwarded by the trusted web proxy.
//! See `docs/concept/authorization.md`.

use std::sync::Arc;

use argon2::{
    password_hash::{rand_core::OsRng, PasswordHash, PasswordHasher, PasswordVerifier, SaltString},
    Argon2,
};
use async_trait::async_trait;
use axum::{
    extract::State,
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};
use serde_json::json;
use tl_core::{ApiError, ApiErrorCode, AuthRequest, AuthResponse, ChangePasswordRequest};
use tokio::sync::RwLock;
use uuid::Uuid;

// -- Store trait + memory impl -------------------------------------------

#[derive(Debug, thiserror::Error)]
pub enum UserStoreError {
    #[error("not found")]
    NotFound,
    #[error("username already exists")]
    Conflict,
    #[error("internal: {0}")]
    Internal(String),
}

#[derive(Debug, Clone)]
pub struct UserRecord {
    pub id: Uuid,
    pub username: String,
    pub password_hash: String,
}

#[async_trait]
pub trait UserStore: Send + Sync {
    async fn create(
        &self,
        username: &str,
        password_hash: &str,
    ) -> Result<UserRecord, UserStoreError>;
    async fn find_by_username(&self, username: &str) -> Result<UserRecord, UserStoreError>;
    async fn update_password(&self, id: Uuid, password_hash: &str) -> Result<(), UserStoreError>;
}

/// Process-local store. Useful for local dev, tests, and the no-DB
/// boot path. Not durable across restarts.
#[derive(Debug, Default)]
pub struct MemoryUserStore {
    inner: RwLock<std::collections::HashMap<String, UserRecord>>,
}

impl MemoryUserStore {
    pub fn new() -> Self {
        Self::default()
    }
}

#[async_trait]
impl UserStore for MemoryUserStore {
    async fn create(
        &self,
        username: &str,
        password_hash: &str,
    ) -> Result<UserRecord, UserStoreError> {
        let key = username.to_ascii_lowercase();
        let mut guard = self.inner.write().await;
        if guard.contains_key(&key) {
            return Err(UserStoreError::Conflict);
        }
        let record = UserRecord {
            id: Uuid::new_v4(),
            username: username.to_string(),
            password_hash: password_hash.to_string(),
        };
        guard.insert(key, record.clone());
        Ok(record)
    }

    async fn find_by_username(&self, username: &str) -> Result<UserRecord, UserStoreError> {
        self.inner
            .read()
            .await
            .get(&username.to_ascii_lowercase())
            .cloned()
            .ok_or(UserStoreError::NotFound)
    }

    async fn update_password(&self, id: Uuid, password_hash: &str) -> Result<(), UserStoreError> {
        let mut guard = self.inner.write().await;
        for record in guard.values_mut() {
            if record.id == id {
                record.password_hash = password_hash.to_string();
                return Ok(());
            }
        }
        Err(UserStoreError::NotFound)
    }
}

// -- Password hashing ----------------------------------------------------

#[derive(Debug, thiserror::Error)]
pub enum PasswordError {
    #[error("hash: {0}")]
    Hash(String),
    #[error("verify: {0}")]
    Verify(String),
}

/// argon2id hash of the SHA-256-hex the client sent. Returns the
/// PHC string (`$argon2id$v=19$...`) suitable for storage.
pub fn hash_password(password_hex: &str) -> Result<String, PasswordError> {
    let salt = SaltString::generate(&mut OsRng);
    let argon = Argon2::default();
    argon
        .hash_password(password_hex.as_bytes(), &salt)
        .map(|hash| hash.to_string())
        .map_err(|e| PasswordError::Hash(e.to_string()))
}

/// Constant-time verification of a candidate SHA-256-hex against a
/// stored PHC string. Returns `Ok(true)` only on match.
pub fn verify_password(password_hex: &str, phc: &str) -> Result<bool, PasswordError> {
    let parsed = PasswordHash::new(phc).map_err(|e| PasswordError::Verify(e.to_string()))?;
    match Argon2::default().verify_password(password_hex.as_bytes(), &parsed) {
        Ok(()) => Ok(true),
        Err(argon2::password_hash::Error::Password) => Ok(false),
        Err(e) => Err(PasswordError::Verify(e.to_string())),
    }
}

// -- Validation ----------------------------------------------------------

const MIN_USERNAME_LEN: usize = 3;
const MAX_USERNAME_LEN: usize = 64;
/// SHA-256 hex digest is always exactly 64 lowercase hex chars.
const SHA256_HEX_LEN: usize = 64;

fn validate_username(s: &str) -> Result<(), String> {
    let trimmed = s.trim();
    if trimmed.len() < MIN_USERNAME_LEN {
        return Err(format!(
            "username must be at least {MIN_USERNAME_LEN} characters"
        ));
    }
    if trimmed.len() > MAX_USERNAME_LEN {
        return Err(format!(
            "username must be at most {MAX_USERNAME_LEN} characters"
        ));
    }
    if !trimmed
        .chars()
        .all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '-' || c == '.')
    {
        return Err("username may only contain a-z, A-Z, 0-9, _, -, .".into());
    }
    Ok(())
}

fn validate_password_hex(s: &str) -> Result<(), String> {
    if s.len() != SHA256_HEX_LEN || !s.chars().all(|c| c.is_ascii_hexdigit()) {
        return Err("password must be a SHA-256 hex digest (64 lowercase hex chars)".into());
    }
    Ok(())
}

// -- Endpoint handlers ---------------------------------------------------

#[derive(Clone)]
pub struct AuthUserState {
    pub store: Arc<dyn UserStore>,
    /// Optional JWT signer. When present, signup/login responses
    /// carry a freshly-minted token in the `jwt` field. When absent
    /// (no `TL_JWT_SECRET` configured, e.g. memory-only dev), the
    /// field is null — the web falls back to header-forwarded
    /// identity via `TL_API_KEY`.
    pub jwt_signer: Option<Arc<crate::jwt::JwtSigner>>,
}

impl AuthUserState {
    fn mint_jwt(&self, user_id: Uuid, username: &str) -> Option<String> {
        let signer = self.jwt_signer.as_ref()?;
        match signer.mint(user_id, username) {
            Ok(token) => Some(token),
            Err(e) => {
                tracing::warn!(error = %e, user_id = %user_id, "jwt mint failed");
                None
            }
        }
    }
}

/// `POST /v1/auth/signup` — create a new account.
#[utoipa::path(
    post,
    path = "/v1/auth/signup",
    tag = "auth",
    request_body = AuthRequest,
    responses(
        (status = 201, description = "Account created", body = AuthResponse),
        (status = 400, description = "Validation failed", body = ApiError),
        (status = 409, description = "Username already exists", body = ApiError),
        (status = 500, description = "Internal error", body = ApiError),
    ),
)]
pub async fn signup(State(state): State<AuthUserState>, Json(req): Json<AuthRequest>) -> Response {
    if let Err(msg) = validate_username(&req.username) {
        return api_error(StatusCode::BAD_REQUEST, ApiErrorCode::Invalid, msg);
    }
    if let Err(msg) = validate_password_hex(&req.password) {
        return api_error(StatusCode::BAD_REQUEST, ApiErrorCode::Invalid, msg);
    }

    let hash = match hash_password(&req.password) {
        Ok(h) => h,
        Err(e) => {
            return api_error(
                StatusCode::INTERNAL_SERVER_ERROR,
                ApiErrorCode::Internal,
                e.to_string(),
            )
        }
    };

    let record = match state.store.create(req.username.trim(), &hash).await {
        Ok(record) => record,
        Err(UserStoreError::Conflict) => {
            return api_error(
                StatusCode::CONFLICT,
                ApiErrorCode::Unprocessable,
                "username already exists".into(),
            )
        }
        Err(e) => {
            return api_error(
                StatusCode::INTERNAL_SERVER_ERROR,
                ApiErrorCode::Internal,
                e.to_string(),
            )
        }
    };

    // Any pending invite for this email is auto-bound on the user's
    // first call to /v1/team/my-workspaces (see
    // TeamStore::accept_pending_invites_for_email).
    let jwt = state.mint_jwt(record.id, &record.username);
    tracing::info!(
        user_id = %record.id,
        username = %record.username,
        "auth signup succeeded"
    );

    (
        StatusCode::CREATED,
        Json(AuthResponse {
            user_id: record.id.to_string(),
            username: record.username,
            jwt,
        }),
    )
        .into_response()
}

/// `POST /v1/auth/login` — verify credentials.
#[utoipa::path(
    post,
    path = "/v1/auth/login",
    tag = "auth",
    request_body = AuthRequest,
    responses(
        (status = 200, description = "Credentials accepted", body = AuthResponse),
        (status = 400, description = "Validation failed", body = ApiError),
        (status = 401, description = "Invalid credentials", body = ApiError),
        (status = 500, description = "Internal error", body = ApiError),
    ),
)]
pub async fn login(State(state): State<AuthUserState>, Json(req): Json<AuthRequest>) -> Response {
    if let Err(msg) = validate_username(&req.username) {
        return api_error(StatusCode::BAD_REQUEST, ApiErrorCode::Invalid, msg);
    }
    if let Err(msg) = validate_password_hex(&req.password) {
        return api_error(StatusCode::BAD_REQUEST, ApiErrorCode::Invalid, msg);
    }

    let record = match state.store.find_by_username(req.username.trim()).await {
        Ok(r) => r,
        // Same response shape for NotFound and bad password so the
        // endpoint doesn't reveal which usernames exist.
        Err(UserStoreError::NotFound) => return invalid_credentials(),
        Err(e) => {
            return api_error(
                StatusCode::INTERNAL_SERVER_ERROR,
                ApiErrorCode::Internal,
                e.to_string(),
            )
        }
    };

    match verify_password(&req.password, &record.password_hash) {
        Ok(true) => {
            let jwt = state.mint_jwt(record.id, &record.username);
            tracing::info!(
                user_id = %record.id,
                username = %record.username,
                "auth login succeeded"
            );
            Json(AuthResponse {
                user_id: record.id.to_string(),
                username: record.username,
                jwt,
            })
            .into_response()
        }
        Ok(false) => invalid_credentials(),
        Err(e) => api_error(
            StatusCode::INTERNAL_SERVER_ERROR,
            ApiErrorCode::Internal,
            e.to_string(),
        ),
    }
}

fn invalid_credentials() -> Response {
    api_error(
        StatusCode::UNAUTHORIZED,
        ApiErrorCode::Unauthorized,
        "invalid username or password".into(),
    )
}

/// `POST /v1/auth/password` — change an existing user's password.
///
/// The caller must demonstrate knowledge of the current password by
/// including it in the request. The current-password check is what proves
/// account ownership here.
#[utoipa::path(
    post,
    path = "/v1/auth/password",
    tag = "auth",
    request_body = ChangePasswordRequest,
    responses(
        (status = 200, description = "Password updated", body = AuthResponse),
        (status = 400, description = "Validation failed", body = ApiError),
        (status = 401, description = "Current password did not match", body = ApiError),
        (status = 500, description = "Internal error", body = ApiError),
    ),
)]
pub async fn change_password(
    State(state): State<AuthUserState>,
    Json(req): Json<ChangePasswordRequest>,
) -> Response {
    if let Err(msg) = validate_username(&req.username) {
        return api_error(StatusCode::BAD_REQUEST, ApiErrorCode::Invalid, msg);
    }
    if let Err(msg) = validate_password_hex(&req.current_password) {
        return api_error(StatusCode::BAD_REQUEST, ApiErrorCode::Invalid, msg);
    }
    if let Err(msg) = validate_password_hex(&req.new_password) {
        return api_error(StatusCode::BAD_REQUEST, ApiErrorCode::Invalid, msg);
    }
    if req.current_password == req.new_password {
        return api_error(
            StatusCode::BAD_REQUEST,
            ApiErrorCode::Invalid,
            "new password must differ from current password".into(),
        );
    }

    let record = match state.store.find_by_username(req.username.trim()).await {
        Ok(r) => r,
        Err(UserStoreError::NotFound) => return invalid_credentials(),
        Err(e) => {
            return api_error(
                StatusCode::INTERNAL_SERVER_ERROR,
                ApiErrorCode::Internal,
                e.to_string(),
            )
        }
    };

    match verify_password(&req.current_password, &record.password_hash) {
        Ok(true) => {}
        Ok(false) => return invalid_credentials(),
        Err(e) => {
            return api_error(
                StatusCode::INTERNAL_SERVER_ERROR,
                ApiErrorCode::Internal,
                e.to_string(),
            )
        }
    }

    let new_hash = match hash_password(&req.new_password) {
        Ok(h) => h,
        Err(e) => {
            return api_error(
                StatusCode::INTERNAL_SERVER_ERROR,
                ApiErrorCode::Internal,
                e.to_string(),
            )
        }
    };

    match state.store.update_password(record.id, &new_hash).await {
        Ok(()) => {
            let jwt = state.mint_jwt(record.id, &record.username);
            Json(AuthResponse {
                user_id: record.id.to_string(),
                username: record.username,
                jwt,
            })
            .into_response()
        }
        Err(UserStoreError::NotFound) => invalid_credentials(),
        Err(e) => api_error(
            StatusCode::INTERNAL_SERVER_ERROR,
            ApiErrorCode::Internal,
            e.to_string(),
        ),
    }
}

fn api_error(status: StatusCode, code: ApiErrorCode, message: String) -> Response {
    crate::log_api_error(status, code, &message);
    let retriable = matches!(
        code,
        ApiErrorCode::RateLimited | ApiErrorCode::Internal | ApiErrorCode::Unavailable
    );
    let body = ApiError {
        code,
        message,
        retriable,
        details: json!(null),
    };
    (status, Json(body)).into_response()
}

// -- Tests ---------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    const VALID_HEX: &str = "9f86d081884c7d659a2feaa0c55ad015a3bf4f1b2b0b822cd15d6c15b0f00a08"; // sha256("test")

    #[test]
    fn hash_roundtrip_matches() {
        let phc = hash_password(VALID_HEX).unwrap();
        assert!(phc.starts_with("$argon2id$"));
        assert!(verify_password(VALID_HEX, &phc).unwrap());
    }

    #[test]
    fn verify_rejects_wrong_password() {
        let phc = hash_password(VALID_HEX).unwrap();
        let other = "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855";
        assert!(!verify_password(other, &phc).unwrap());
    }

    #[test]
    fn validate_username_rules() {
        assert!(validate_username("ab").is_err()); // too short
        assert!(validate_username("a".repeat(65).as_str()).is_err()); // too long
        assert!(validate_username("bad space").is_err());
        assert!(validate_username("bad!char").is_err());
        assert!(validate_username("good-user.1").is_ok());
        assert!(validate_username("Alice_2").is_ok());
    }

    #[test]
    fn validate_password_rules() {
        assert!(validate_password_hex(VALID_HEX).is_ok());
        assert!(validate_password_hex("short").is_err());
        assert!(validate_password_hex(&"z".repeat(64)).is_err()); // non-hex
        assert!(validate_password_hex(&VALID_HEX.to_ascii_uppercase()).is_ok());
        // hex is hex
    }

    #[tokio::test]
    async fn memory_store_create_and_find_case_insensitive() {
        let s = MemoryUserStore::new();
        let r = s.create("Alice", "phc").await.unwrap();
        let found = s.find_by_username("alice").await.unwrap();
        assert_eq!(r.id, found.id);
    }

    #[tokio::test]
    async fn memory_store_conflict_on_duplicate() {
        let s = MemoryUserStore::new();
        s.create("alice", "phc").await.unwrap();
        let err = s.create("ALICE", "phc").await.unwrap_err();
        assert!(matches!(err, UserStoreError::Conflict));
    }

    #[tokio::test]
    async fn signup_then_login_round_trip() {
        let state = AuthUserState {
            store: Arc::new(MemoryUserStore::new()),
            jwt_signer: None,
        };
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
        let state = AuthUserState {
            store: Arc::new(MemoryUserStore::new()),
            jwt_signer: None,
        };
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
        let state = AuthUserState {
            store: Arc::new(MemoryUserStore::new()),
            jwt_signer: None,
        };
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

    const OTHER_HEX: &str = "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855"; // sha256("")

    #[tokio::test]
    async fn change_password_then_login_with_new_password() {
        let state = AuthUserState {
            store: Arc::new(MemoryUserStore::new()),
            jwt_signer: None,
        };
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
        let state = AuthUserState {
            store: Arc::new(MemoryUserStore::new()),
            jwt_signer: None,
        };
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
        let state = AuthUserState {
            store: Arc::new(MemoryUserStore::new()),
            jwt_signer: None,
        };
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
}
