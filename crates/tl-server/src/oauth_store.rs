use std::collections::HashMap;
use std::sync::Mutex;

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use sha2::{Digest, Sha256};
use uuid::Uuid;

#[derive(Debug, Clone, thiserror::Error)]
pub enum OAuthStoreError {
    #[error("not found")]
    NotFound,
    #[error("conflict")]
    Conflict,
    #[error("internal: {0}")]
    Internal(String),
}

#[derive(Debug, Clone)]
pub struct OAuthClientRecord {
    pub client_id: String,
    pub client_name: Option<String>,
    pub redirect_uris: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct OAuthAuthorizationCodeRecord {
    pub client_id: String,
    pub redirect_uri: String,
    pub user_id: Uuid,
    pub username: String,
    pub workspace_id: String,
    pub resource: String,
    pub scope: String,
    pub code_challenge: String,
    pub expires_at: DateTime<Utc>,
}

#[derive(Debug, Clone)]
pub struct OAuthRefreshTokenRecord {
    pub client_id: String,
    pub user_id: Uuid,
    pub username: String,
    pub workspace_id: String,
    pub resource: String,
    pub scope: String,
    pub expires_at: DateTime<Utc>,
}

#[async_trait]
pub trait OAuthStore: Send + Sync {
    async fn client_count(&self) -> Result<usize, OAuthStoreError>;
    async fn create_client(&self, client: OAuthClientRecord) -> Result<(), OAuthStoreError>;
    async fn get_client(&self, client_id: &str) -> Result<OAuthClientRecord, OAuthStoreError>;
    async fn put_code(
        &self,
        code_hash: String,
        record: OAuthAuthorizationCodeRecord,
    ) -> Result<(), OAuthStoreError>;
    async fn take_code(
        &self,
        code_hash: &str,
    ) -> Result<OAuthAuthorizationCodeRecord, OAuthStoreError>;
    async fn put_refresh(
        &self,
        token_hash: String,
        record: OAuthRefreshTokenRecord,
    ) -> Result<(), OAuthStoreError>;
    async fn take_refresh(
        &self,
        token_hash: &str,
    ) -> Result<OAuthRefreshTokenRecord, OAuthStoreError>;
}

#[derive(Default)]
pub struct MemoryOAuthStore {
    clients: Mutex<HashMap<String, OAuthClientRecord>>,
    codes: Mutex<HashMap<String, OAuthAuthorizationCodeRecord>>,
    refresh: Mutex<HashMap<String, OAuthRefreshTokenRecord>>,
}

#[async_trait]
impl OAuthStore for MemoryOAuthStore {
    async fn client_count(&self) -> Result<usize, OAuthStoreError> {
        Ok(self
            .clients
            .lock()
            .unwrap_or_else(|error| error.into_inner())
            .len())
    }

    async fn create_client(&self, client: OAuthClientRecord) -> Result<(), OAuthStoreError> {
        let mut clients = self
            .clients
            .lock()
            .unwrap_or_else(|error| error.into_inner());
        if clients.contains_key(&client.client_id) {
            return Err(OAuthStoreError::Conflict);
        }
        clients.insert(client.client_id.clone(), client);
        Ok(())
    }

    async fn get_client(&self, client_id: &str) -> Result<OAuthClientRecord, OAuthStoreError> {
        self.clients
            .lock()
            .unwrap_or_else(|error| error.into_inner())
            .get(client_id)
            .cloned()
            .ok_or(OAuthStoreError::NotFound)
    }

    async fn put_code(
        &self,
        code_hash: String,
        record: OAuthAuthorizationCodeRecord,
    ) -> Result<(), OAuthStoreError> {
        let replaced = self
            .codes
            .lock()
            .unwrap_or_else(|error| error.into_inner())
            .insert(code_hash, record);
        if replaced.is_some() {
            Err(OAuthStoreError::Conflict)
        } else {
            Ok(())
        }
    }

    async fn take_code(
        &self,
        code_hash: &str,
    ) -> Result<OAuthAuthorizationCodeRecord, OAuthStoreError> {
        self.codes
            .lock()
            .unwrap_or_else(|error| error.into_inner())
            .remove(code_hash)
            .ok_or(OAuthStoreError::NotFound)
    }

    async fn put_refresh(
        &self,
        token_hash: String,
        record: OAuthRefreshTokenRecord,
    ) -> Result<(), OAuthStoreError> {
        let replaced = self
            .refresh
            .lock()
            .unwrap_or_else(|error| error.into_inner())
            .insert(token_hash, record);
        if replaced.is_some() {
            Err(OAuthStoreError::Conflict)
        } else {
            Ok(())
        }
    }

    async fn take_refresh(
        &self,
        token_hash: &str,
    ) -> Result<OAuthRefreshTokenRecord, OAuthStoreError> {
        self.refresh
            .lock()
            .unwrap_or_else(|error| error.into_inner())
            .remove(token_hash)
            .ok_or(OAuthStoreError::NotFound)
    }
}

pub fn hash_opaque_token(token: &str) -> String {
    Sha256::digest(token.as_bytes())
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect()
}

pub fn expires_after_seconds(seconds: i64) -> DateTime<Utc> {
    Utc::now() + chrono::Duration::seconds(seconds)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn opaque_token_hash_is_stable_and_does_not_contain_plaintext() {
        let hash = hash_opaque_token("sensitive-token");
        assert_eq!(hash.len(), 64);
        assert!(!hash.contains("sensitive-token"));
        assert_eq!(hash, hash_opaque_token("sensitive-token"));
    }

    #[tokio::test]
    async fn bounded_registration_is_atomic_for_the_memory_store() {
        let store = MemoryOAuthStore::default();
        let first = OAuthClientRecord {
            client_id: "first".into(),
            client_name: None,
            redirect_uris: vec!["http://127.0.0.1/callback".into()],
        };
        let second = OAuthClientRecord {
            client_id: "second".into(),
            client_name: None,
            redirect_uris: vec!["http://127.0.0.1/callback".into()],
        };

        store
            .create_client_bounded(first, 1)
            .await
            .expect("first registration");
        assert!(matches!(
            store.create_client_bounded(second, 1).await,
            Err(OAuthStoreError::Capacity)
        ));
    }
}
