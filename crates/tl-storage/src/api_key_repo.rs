//! Per-user API key repository.
//!
//! Mints, lists, revokes, and looks up API keys stored in the
//! `"ApiKey"` table. Plaintext is shown to the caller once at
//! creation and discarded — we hold only SHA-256(plaintext).

use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use base64::Engine;
use chrono::{DateTime, Utc};
use rand::RngCore;
use sha2::{Digest, Sha256};
use sqlx::postgres::PgPool;
use sqlx::FromRow;
use uuid::Uuid;

use crate::StorageError;

#[derive(Debug, Clone, FromRow)]
pub struct ApiKeyRecord {
    pub id: Uuid,
    pub user_id: String,
    pub name: String,
    pub prefix: String,
    pub last_used_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
    pub revoked_at: Option<DateTime<Utc>>,
}

/// Result of a successful `create` — includes the plaintext, which the
/// caller MUST return to the user immediately and then drop. Never
/// stored, never logged.
#[derive(Debug)]
pub struct MintedApiKey {
    pub record: ApiKeyRecord,
    pub plaintext: String,
}

#[derive(Clone)]
pub struct ApiKeyRepo {
    pool: PgPool,
}

impl ApiKeyRepo {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    pub fn pool(&self) -> &PgPool {
        &self.pool
    }

    /// Mint a new key. Returns the plaintext (shown once) plus the
    /// stored record metadata.
    pub async fn create(&self, user_id: &str, name: &str) -> Result<MintedApiKey, StorageError> {
        let (plaintext, prefix, hash) = generate_key();
        let id = Uuid::new_v4();

        let record: ApiKeyRecord = sqlx::query_as(
            r#"
            INSERT INTO "ApiKey" (id, user_id, name, prefix, hash)
            VALUES ($1, $2, $3, $4, $5)
            RETURNING id, user_id, name, prefix, last_used_at, created_at, revoked_at
            "#,
        )
        .bind(id)
        .bind(user_id)
        .bind(name)
        .bind(&prefix)
        .bind(&hash[..])
        .fetch_one(&self.pool)
        .await
        .map_err(|e| StorageError::Internal(format!("api_key create: {e}")))?;

        Ok(MintedApiKey { record, plaintext })
    }

    pub async fn list_by_user(&self, user_id: &str) -> Result<Vec<ApiKeyRecord>, StorageError> {
        sqlx::query_as::<_, ApiKeyRecord>(
            r#"
            SELECT id, user_id, name, prefix, last_used_at, created_at, revoked_at
            FROM "ApiKey"
            WHERE user_id = $1
            ORDER BY created_at DESC
            "#,
        )
        .bind(user_id)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| StorageError::Internal(format!("api_key list: {e}")))
    }

    /// Soft-revoke. Returns false when the row doesn't exist for this
    /// user or is already revoked, so revoking twice surfaces a 404.
    pub async fn revoke(&self, id: Uuid, user_id: &str) -> Result<bool, StorageError> {
        let result = sqlx::query(
            r#"
            UPDATE "ApiKey"
            SET revoked_at = NOW()
            WHERE id = $1 AND user_id = $2 AND revoked_at IS NULL
            "#,
        )
        .bind(id)
        .bind(user_id)
        .execute(&self.pool)
        .await
        .map_err(|e| StorageError::Internal(format!("api_key revoke: {e}")))?;

        Ok(result.rows_affected() > 0)
    }

    /// Hot-path lookup used by `/v1/check` in a later PR. Returns None
    /// for missing or revoked keys.
    pub async fn lookup_by_hash(&self, hash: &[u8]) -> Result<Option<ApiKeyRecord>, StorageError> {
        sqlx::query_as::<_, ApiKeyRecord>(
            r#"
            SELECT id, user_id, name, prefix, last_used_at, created_at, revoked_at
            FROM "ApiKey"
            WHERE hash = $1 AND revoked_at IS NULL
            "#,
        )
        .bind(hash)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| StorageError::Internal(format!("api_key lookup: {e}")))
    }

    /// Fire-and-forget last_used update. Never blocks the request path.
    pub async fn touch(&self, id: Uuid) -> Result<(), StorageError> {
        sqlx::query(r#"UPDATE "ApiKey" SET last_used_at = NOW() WHERE id = $1"#)
            .bind(id)
            .execute(&self.pool)
            .await
            .map_err(|e| StorageError::Internal(format!("api_key touch: {e}")))?;
        Ok(())
    }
}

impl std::fmt::Debug for ApiKeyRepo {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ApiKeyRepo").finish_non_exhaustive()
    }
}

/// Generate plaintext, prefix, and the SHA-256 hash of plaintext.
/// Plaintext format: `tlg_<5chars>_<base64url-no-pad of 32 random bytes>`
fn generate_key() -> (String, String, [u8; 32]) {
    let mut random = [0u8; 32];
    rand::thread_rng().fill_bytes(&mut random);
    let random_b64 = URL_SAFE_NO_PAD.encode(random);

    let prefix: String = random_b64.chars().take(5).collect();
    let plaintext = format!("tlg_{}_{}", prefix, random_b64);

    let hash = hash_plaintext(&plaintext);
    (plaintext, prefix, hash)
}

/// Hash a plaintext token. Exposed so `/v1/check` (later PR) can hash
/// the caller-supplied bearer the same way before calling
/// `lookup_by_hash`.
pub fn hash_plaintext(plaintext: &str) -> [u8; 32] {
    let mut hasher = Sha256::new();
    hasher.update(plaintext.as_bytes());
    hasher.finalize().into()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn generated_key_has_expected_shape() {
        let (plaintext, prefix, hash) = generate_key();
        assert!(plaintext.starts_with("tlg_"));
        assert!(plaintext.contains(&format!("_{prefix}_")));
        assert_eq!(prefix.len(), 5);
        assert_eq!(hash.len(), 32);

        let recomputed = hash_plaintext(&plaintext);
        assert_eq!(hash, recomputed);
    }

    #[test]
    fn generated_keys_are_unique() {
        let (a, _, _) = generate_key();
        let (b, _, _) = generate_key();
        assert_ne!(a, b);
    }
}
