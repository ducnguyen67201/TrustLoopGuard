//! User-session JWT issuance + verification.
//!
//! `tl-server` mints a short-lived HS256 JWT on successful
//! `POST /v1/auth/{signup,login}` and verifies it on protected `/v1/*`
//! routes via the bearer middleware. Web stashes the token in the
//! NextAuth session; the customer's browser cookie does the auto-send
//! part. SDKs do **not** use this — they'll use per-workspace API
//! keys (see `docs/concept/authorization.md`).
//!
//! Claims are deliberately minimal:
//! - `sub` — the user's UUID (string)
//! - `username` — for downstream logging / auditing
//! - `exp` / `iat` — standard timestamps
//!
//! No roles, no scopes — authorization decisions still live at the
//! workspace/membership layer.

use std::sync::Arc;

use chrono::{Duration, Utc};
use jsonwebtoken::{
    decode, encode, errors::ErrorKind, Algorithm, DecodingKey, EncodingKey, Header, Validation,
};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// How long a freshly-minted user-session JWT stays valid.
pub const JWT_TTL_DAYS: i64 = 7;

/// How long an OAuth access token (MCP) stays valid. Short — refresh to renew.
pub const ACCESS_TOKEN_TTL_MINUTES: i64 = 60;

#[derive(Debug, Serialize, Deserialize)]
pub struct Claims {
    pub sub: String,
    pub username: String,
    pub iat: i64,
    pub exp: i64,
    /// OAuth: the workspace this token is scoped to. `None` for user-session
    /// JWTs (workspace resolved per-request); `Some` for MCP access tokens,
    /// which are bound to exactly one workspace at consent time.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub workspace_id: Option<String>,
    /// OAuth: token type — `"access"` for MCP access tokens. Absent on
    /// user-session JWTs.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub token_type: Option<String>,
}

#[derive(Debug, thiserror::Error)]
pub enum JwtError {
    #[error("invalid jwt: {0}")]
    Invalid(String),
    #[error("expired jwt")]
    Expired,
    #[error("malformed user id in jwt sub claim")]
    BadSubject,
}

/// Loaded once at boot. Holds both the encode and decode keys so the
/// hot path doesn't re-derive them per request.
pub struct JwtSigner {
    encode_key: EncodingKey,
    decode_key: DecodingKey,
    validation: Validation,
}

impl std::fmt::Debug for JwtSigner {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("JwtSigner").finish_non_exhaustive()
    }
}

impl JwtSigner {
    pub fn new(secret: impl AsRef<[u8]>) -> Arc<Self> {
        let bytes = secret.as_ref();
        let mut validation = Validation::new(Algorithm::HS256);
        validation.leeway = 5; // seconds
        Arc::new(Self {
            encode_key: EncodingKey::from_secret(bytes),
            decode_key: DecodingKey::from_secret(bytes),
            validation,
        })
    }

    /// Reads `TL_JWT_SECRET` from the environment. Returns `Ok(None)`
    /// when unset — callers decide whether that's fatal (production)
    /// or fine (memory-only dev, where unauth'd endpoints already
    /// imply "anything goes").
    pub fn from_env() -> Option<Arc<Self>> {
        let raw = std::env::var("TL_JWT_SECRET").ok()?;
        let trimmed = raw.trim();
        if trimmed.is_empty() {
            return None;
        }
        if trimmed.len() < 32 {
            tracing::warn!(
                "TL_JWT_SECRET is shorter than 32 chars — use at least 32 random bytes for HS256"
            );
        }
        Some(Self::new(trimmed))
    }

    pub fn mint(&self, user_id: Uuid, username: &str) -> Result<String, JwtError> {
        let now = Utc::now();
        let claims = Claims {
            sub: user_id.to_string(),
            username: username.to_string(),
            iat: now.timestamp(),
            exp: (now + Duration::days(JWT_TTL_DAYS)).timestamp(),
            workspace_id: None,
            token_type: None,
        };
        self.encode(&claims)
    }

    /// Mint a short-lived OAuth access token bound to one workspace. Used by
    /// the MCP token endpoint; the resource-server lane reads `workspace_id`.
    pub fn mint_access_token(
        &self,
        user_id: Uuid,
        username: &str,
        workspace_id: &str,
    ) -> Result<String, JwtError> {
        let now = Utc::now();
        let claims = Claims {
            sub: user_id.to_string(),
            username: username.to_string(),
            iat: now.timestamp(),
            exp: (now + Duration::minutes(ACCESS_TOKEN_TTL_MINUTES)).timestamp(),
            workspace_id: Some(workspace_id.to_string()),
            token_type: Some("access".to_string()),
        };
        self.encode(&claims)
    }

    fn encode(&self, claims: &Claims) -> Result<String, JwtError> {
        encode(&Header::new(Algorithm::HS256), claims, &self.encode_key)
            .map_err(|e| JwtError::Invalid(e.to_string()))
    }

    pub fn verify(&self, token: &str) -> Result<Claims, JwtError> {
        let data =
            decode::<Claims>(token, &self.decode_key, &self.validation).map_err(|e| {
                match e.kind() {
                    ErrorKind::ExpiredSignature => JwtError::Expired,
                    _ => JwtError::Invalid(e.to_string()),
                }
            })?;
        // Sanity check the sub is a parseable uuid; the middleware
        // attaches it as Uuid so handlers don't need to re-parse.
        Uuid::parse_str(&data.claims.sub).map_err(|_| JwtError::BadSubject)?;
        Ok(data.claims)
    }
}

/// Identity attached to the request extension by the bearer
/// middleware when the caller presented a valid user JWT (as opposed
/// to the internal `TL_API_KEY`). Handlers that need user context
/// without trusting raw headers should read this.
#[derive(Debug, Clone)]
pub struct UserContext {
    pub user_id: Uuid,
    pub username: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    fn signer() -> Arc<JwtSigner> {
        JwtSigner::new("test-secret-test-secret-test-secret-12")
    }

    #[test]
    fn round_trip_mints_and_verifies() {
        let s = signer();
        let id = Uuid::new_v4();
        let token = s.mint(id, "alice").unwrap();
        let claims = s.verify(&token).unwrap();
        assert_eq!(claims.sub, id.to_string());
        assert_eq!(claims.username, "alice");
    }

    #[test]
    fn rejects_wrong_secret() {
        let a = signer();
        let b = JwtSigner::new("different-secret-different-secret-12");
        let token = a.mint(Uuid::new_v4(), "alice").unwrap();
        assert!(b.verify(&token).is_err());
    }

    #[test]
    fn access_token_carries_workspace_and_type() {
        let s = signer();
        let id = Uuid::new_v4();
        let token = s.mint_access_token(id, "alice", "ws_test").unwrap();
        let claims = s.verify(&token).unwrap();
        assert_eq!(claims.workspace_id.as_deref(), Some("ws_test"));
        assert_eq!(claims.token_type.as_deref(), Some("access"));
    }

    #[test]
    fn user_jwt_has_no_workspace_scope() {
        let s = signer();
        let claims = s.verify(&s.mint(Uuid::new_v4(), "alice").unwrap()).unwrap();
        assert!(claims.workspace_id.is_none());
        assert!(claims.token_type.is_none());
    }

    #[test]
    fn rejects_garbage() {
        let s = signer();
        assert!(matches!(
            s.verify("not.a.jwt").unwrap_err(),
            JwtError::Invalid(_)
        ));
    }
}
