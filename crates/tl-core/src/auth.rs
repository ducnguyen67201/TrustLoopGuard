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
