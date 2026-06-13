//! Report share tokens: the durable capability that backs a public report link.
//!
//! A *share* points at one completed job (optionally a same-agent comparison)
//! and is addressed by an unguessable token. The token is the sole bearer
//! credential for the public report endpoint; `workspace_id` is held internally
//! to scope the job lookup and is never returned to the public client.

use std::collections::HashMap;

use async_trait::async_trait;
use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine as _};
use rand::{rngs::OsRng, RngCore};
use tokio::sync::RwLock;

use super::RedteamJobStoreError;

/// Server-internal record of a minted share. Timestamps are RFC 3339 strings to
/// match the rest of the red-team layer.
#[derive(Debug, Clone)]
pub struct ReportShare {
    pub token: String,
    pub workspace_id: String,
    pub job_id: String,
    pub compare_job_id: Option<String>,
    pub created_at: String,
    pub expires_at: Option<String>,
}

/// Inputs for minting a share. The caller owns token generation and expiry math.
pub struct NewReportShare<'a> {
    pub token: &'a str,
    pub workspace_id: &'a str,
    pub job_id: &'a str,
    pub compare_job_id: Option<&'a str>,
    pub expires_at: Option<&'a str>,
}

/// Durable storage for report share tokens. Mirrors the `RedteamJobStore`
/// split: a memory implementation for tests/memory-only builds and a Postgres
/// adapter for production.
#[async_trait]
pub trait RedteamReportShareStore: Send + Sync {
    async fn create(&self, share: NewReportShare<'_>) -> Result<ReportShare, RedteamJobStoreError>;
    /// Resolve a non-revoked, non-expired share. Missing/revoked/expired all map
    /// to `NotFound` so the public endpoint is not a token-validity oracle.
    async fn get_by_token(&self, token: &str) -> Result<ReportShare, RedteamJobStoreError>;
    /// Revoke a share within its workspace. `NotFound` when the token is absent,
    /// foreign, or already revoked.
    async fn revoke(&self, workspace_id: &str, token: &str) -> Result<(), RedteamJobStoreError>;
}

/// 32 bytes of OS randomness (~256 bits), URL-safe, `rpt_`-prefixed. Mirrors the
/// API-key generator in `dashboard_admin`.
pub fn generate_share_token() -> String {
    let mut bytes = [0_u8; 32];
    OsRng.fill_bytes(&mut bytes);
    format!("rpt_{}", URL_SAFE_NO_PAD.encode(bytes))
}

/// True when an RFC 3339 expiry is in the past. An unparseable value is treated
/// as expired (defensive — a malformed row must never read as live).
fn is_expired(expires_at: &Option<String>) -> bool {
    match expires_at {
        None => false,
        Some(ts) => match chrono::DateTime::parse_from_rfc3339(ts) {
            Ok(dt) => dt <= chrono::Utc::now(),
            Err(_) => true,
        },
    }
}

#[derive(Debug, Clone)]
struct MemShare {
    share: ReportShare,
    revoked: bool,
}

/// In-memory `RedteamReportShareStore` for tests and memory-only deployments.
#[derive(Debug, Default)]
pub struct MemoryRedteamReportShareStore {
    shares: RwLock<HashMap<String, MemShare>>,
}

impl MemoryRedteamReportShareStore {
    pub fn new() -> Self {
        Self::default()
    }
}

#[async_trait]
impl RedteamReportShareStore for MemoryRedteamReportShareStore {
    async fn create(&self, new: NewReportShare<'_>) -> Result<ReportShare, RedteamJobStoreError> {
        let share = ReportShare {
            token: new.token.to_string(),
            workspace_id: new.workspace_id.to_string(),
            job_id: new.job_id.to_string(),
            compare_job_id: new.compare_job_id.map(str::to_string),
            created_at: chrono::Utc::now().to_rfc3339(),
            expires_at: new.expires_at.map(str::to_string),
        };
        self.shares.write().await.insert(
            share.token.clone(),
            MemShare {
                share: share.clone(),
                revoked: false,
            },
        );
        Ok(share)
    }

    async fn get_by_token(&self, token: &str) -> Result<ReportShare, RedteamJobStoreError> {
        let shares = self.shares.read().await;
        let entry = shares.get(token).ok_or(RedteamJobStoreError::NotFound)?;
        if entry.revoked || is_expired(&entry.share.expires_at) {
            return Err(RedteamJobStoreError::NotFound);
        }
        Ok(entry.share.clone())
    }

    async fn revoke(&self, workspace_id: &str, token: &str) -> Result<(), RedteamJobStoreError> {
        let mut shares = self.shares.write().await;
        let entry = shares
            .get_mut(token)
            .ok_or(RedteamJobStoreError::NotFound)?;
        if entry.share.workspace_id != workspace_id || entry.revoked {
            return Err(RedteamJobStoreError::NotFound);
        }
        entry.revoked = true;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn new_share<'a>(token: &'a str, expires_at: Option<&'a str>) -> NewReportShare<'a> {
        NewReportShare {
            token,
            workspace_id: "ws",
            job_id: "job-1",
            compare_job_id: None,
            expires_at,
        }
    }

    #[test]
    fn token_is_prefixed_and_unguessable() {
        let a = generate_share_token();
        let b = generate_share_token();
        assert!(a.starts_with("rpt_"));
        assert_ne!(a, b);
        assert!(a.len() > 40);
    }

    #[tokio::test]
    async fn create_then_get_round_trips() {
        let store = MemoryRedteamReportShareStore::new();
        let created = store.create(new_share("rpt_a", None)).await.unwrap();
        let fetched = store.get_by_token("rpt_a").await.unwrap();
        assert_eq!(created.job_id, fetched.job_id);
        assert_eq!(fetched.workspace_id, "ws");
    }

    #[tokio::test]
    async fn expired_share_reads_as_not_found() {
        let store = MemoryRedteamReportShareStore::new();
        store
            .create(new_share("rpt_old", Some("2000-01-01T00:00:00Z")))
            .await
            .unwrap();
        assert!(store.get_by_token("rpt_old").await.is_err());
    }

    #[tokio::test]
    async fn revoked_share_reads_as_not_found() {
        let store = MemoryRedteamReportShareStore::new();
        store.create(new_share("rpt_r", None)).await.unwrap();
        store.revoke("ws", "rpt_r").await.unwrap();
        assert!(store.get_by_token("rpt_r").await.is_err());
        // Revoking again is NotFound (already revoked).
        assert!(store.revoke("ws", "rpt_r").await.is_err());
    }

    #[tokio::test]
    async fn revoke_is_workspace_scoped() {
        let store = MemoryRedteamReportShareStore::new();
        store.create(new_share("rpt_w", None)).await.unwrap();
        assert!(store.revoke("other-ws", "rpt_w").await.is_err());
        // Still live for its own workspace.
        assert!(store.get_by_token("rpt_w").await.is_ok());
    }
}
