//! Username/password authentication for local development.
//!
//! Companion to [`crate::auth`] (the static bearer-token middleware
//! that protects SDK calls). This module adds *user* accounts so
//! local developers have a way to sign in without configuring
//! GitHub/Google OAuth providers in `apps/web`.
//!
//! Endpoints (public only when `AuthUserState.password_auth_enabled`
//! is true):
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
//! OAuth login is the staging/production path. After Auth.js finishes
//! with Google/GitHub, `POST /v1/identity/oauth-session` links that
//! provider identity to the Rust-owned local user row.

use std::sync::Arc;

use async_trait::async_trait;
use uuid::Uuid;

pub(crate) mod handlers;
mod memory_store;
pub(crate) mod oauth;
mod password;
mod response;
#[cfg(test)]
mod tests;
mod validation;
pub use handlers::{
    __path_change_password, __path_login, __path_signup, change_password, login, signup,
};
pub use memory_store::MemoryUserStore;
pub use oauth::oauth_session;

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
    pub is_approved: bool,
}

#[async_trait]
pub trait UserStore: Send + Sync {
    async fn create(
        &self,
        username: &str,
        password_hash: &str,
    ) -> Result<UserRecord, UserStoreError>;
    async fn find_by_username(&self, username: &str) -> Result<UserRecord, UserStoreError>;
    async fn is_approved(&self, id: Uuid) -> Result<bool, UserStoreError>;
    async fn ensure_oauth_identity(
        &self,
        provider: &str,
        provider_subject: &str,
        email: &str,
    ) -> Result<UserRecord, UserStoreError>;
    async fn update_password(&self, id: Uuid, password_hash: &str) -> Result<(), UserStoreError>;
}

pub(super) fn normalize_oauth_provider(provider: &str) -> Result<String, UserStoreError> {
    let normalized = provider.trim().to_ascii_lowercase();
    match normalized.as_str() {
        "google" | "github" => Ok(normalized),
        _ => Err(UserStoreError::Internal(format!(
            "unsupported oauth provider: {provider}"
        ))),
    }
}

// -- Endpoint handlers ---------------------------------------------------

#[derive(Clone)]
pub struct AuthUserState {
    pub store: Arc<dyn UserStore>,
    pub password_auth_enabled: bool,
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
