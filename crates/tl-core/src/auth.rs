//! Username/password authentication DTOs.
//!
//! Wire format only — the request/response shapes for
//! `POST /v1/auth/{signup,login,password}`. Hashing, validation,
//! storage, and the actual handlers live in `tl-server`; this module
//! exists so OpenAPI + Python + TypeScript codegen sees a single
//! source of truth.

use serde::{Deserialize, Serialize};

#[cfg(feature = "schema")]
use schemars::JsonSchema;
#[cfg(feature = "ts-export")]
use ts_rs::TS;
#[cfg(feature = "openapi")]
use utoipa::ToSchema;

/// Login + signup request body. `password` is the SHA-256 hex digest
/// of the user's plaintext password — see `tl-server::auth_user` for
/// the storage-side argon2 wrap.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(JsonSchema))]
#[cfg_attr(feature = "openapi", derive(ToSchema))]
#[cfg_attr(feature = "ts-export", derive(TS))]
#[cfg_attr(feature = "ts-export", ts(export))]
pub struct AuthRequest {
    /// Account identifier. Stored as-given, matched case-insensitively.
    pub username: String,
    /// SHA-256-hex of the user's plaintext password.
    pub password: String,
}

/// Login + signup success response. `user_id` is a v4 UUID rendered as
/// a string so SDK type generation stays uniform across languages.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(JsonSchema))]
#[cfg_attr(feature = "openapi", derive(ToSchema))]
#[cfg_attr(feature = "ts-export", derive(TS))]
#[cfg_attr(feature = "ts-export", ts(export))]
pub struct AuthResponse {
    pub user_id: String,
    pub username: String,
    /// Signed bearer token (HS256) issued by `tl-server` on successful
    /// signup or login. The web dashboard stashes this in the
    /// NextAuth session and forwards it on every Rust API call as
    /// `Authorization: Bearer <jwt>`. Absent when the server runs
    /// without `TL_JWT_SECRET` configured (memory-only dev mode).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub jwt: Option<String>,
}

/// OAuth identity resolved after Google/GitHub has already
/// authenticated the browser user. This endpoint does not verify
/// provider credentials; the trusted web app sends the provider's
/// stable subject and profile after Auth.js completes the OAuth flow.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(JsonSchema))]
#[cfg_attr(feature = "openapi", derive(ToSchema))]
#[cfg_attr(feature = "ts-export", derive(TS))]
#[cfg_attr(feature = "ts-export", ts(export))]
pub struct OAuthIdentityRequest {
    /// Provider id from Auth.js, e.g. `google` or `github`.
    pub provider: String,
    /// Stable provider account id (`account.providerAccountId`).
    pub provider_subject: String,
    /// Provider-verified or provider-supplied email. Used to link a
    /// first OAuth sign-in to an existing local app user.
    pub email: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts-export", ts(optional))]
    pub name: Option<String>,
}

/// PKCE authorization-code request submitted by the trusted dashboard service.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(JsonSchema))]
#[cfg_attr(feature = "openapi", derive(ToSchema))]
#[cfg_attr(feature = "ts-export", derive(TS))]
#[cfg_attr(feature = "ts-export", ts(export))]
pub struct AuthorizeRequest {
    pub client_id: String,
    pub redirect_uri: String,
    pub code_challenge: String,
    #[serde(default)]
    pub code_challenge_method: Option<String>,
    #[serde(default)]
    pub resource: Option<String>,
    #[serde(default)]
    pub scope: Option<String>,
    pub agent_id: Option<String>,
}

/// PKCE-bound OAuth authorization code issued to the trusted dashboard.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(JsonSchema))]
#[cfg_attr(feature = "openapi", derive(ToSchema))]
#[cfg_attr(feature = "ts-export", derive(TS))]
#[cfg_attr(feature = "ts-export", ts(export))]
pub struct AuthorizeResponse {
    pub code: String,
}

/// Change-password request body. The caller demonstrates knowledge of
/// the current password by hashing it the same way as a login.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(JsonSchema))]
#[cfg_attr(feature = "openapi", derive(ToSchema))]
#[cfg_attr(feature = "ts-export", derive(TS))]
#[cfg_attr(feature = "ts-export", ts(export))]
pub struct ChangePasswordRequest {
    pub username: String,
    /// SHA-256-hex of the user's current password.
    pub current_password: String,
    /// SHA-256-hex of the new password to store.
    pub new_password: String,
}
