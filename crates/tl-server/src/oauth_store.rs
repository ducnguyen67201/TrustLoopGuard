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
    #[error("capacity reached")]
    Capacity,
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
    async fn create_client_bounded(
        &self,
        client: OAuthClientRecord,
        max_clients: usize,
    ) -> Result<(), OAuthStoreError>;
    async fn prune_inactive_clients(
        &self,
        inactive_before: DateTime<Utc>,
    ) -> Result<usize, OAuthStoreError>;
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
    clients: Mutex<HashMap<String, (OAuthClientRecord, DateTime<Utc>)>>,
    codes: Mutex<HashMap<String, OAuthAuthorizationCodeRecord>>,
    refresh: Mutex<HashMap<String, OAuthRefreshTokenRecord>>,
}

#[async_trait]
impl OAuthStore for MemoryOAuthStore {
    async fn create_client_bounded(
        &self,
        client: OAuthClientRecord,
        max_clients: usize,
    ) -> Result<(), OAuthStoreError> {
        let mut clients = self
            .clients
            .lock()
            .unwrap_or_else(|error| error.into_inner());
        if clients.contains_key(&client.client_id) {
            return Err(OAuthStoreError::Conflict);
        }
        if clients.len() >= max_clients {
            return Err(OAuthStoreError::Capacity);
        }
        clients.insert(client.client_id.clone(), (client, Utc::now()));
        Ok(())
    }

    async fn prune_inactive_clients(
        &self,
        inactive_before: DateTime<Utc>,
    ) -> Result<usize, OAuthStoreError> {
        let active_client_ids = {
            let now = Utc::now();
            let codes = self.codes.lock().unwrap_or_else(|error| error.into_inner());
            let refresh = self
                .refresh
                .lock()
                .unwrap_or_else(|error| error.into_inner());
            codes
                .values()
                .filter(|record| record.expires_at > now)
                .map(|record| record.client_id.clone())
                .chain(
                    refresh
                        .values()
                        .filter(|record| record.expires_at > now)
                        .map(|record| record.client_id.clone()),
                )
                .collect::<std::collections::HashSet<_>>()
        };
        let mut clients = self
            .clients
            .lock()
            .unwrap_or_else(|error| error.into_inner());
        let before = clients.len();
        clients.retain(|client_id, (_, created_at)| {
            *created_at >= inactive_before || active_client_ids.contains(client_id)
        });
        Ok(before - clients.len())
    }

    async fn get_client(&self, client_id: &str) -> Result<OAuthClientRecord, OAuthStoreError> {
        self.clients
            .lock()
            .unwrap_or_else(|error| error.into_inner())
            .get(client_id)
            .map(|(client, _)| client.clone())
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

    #[tokio::test]
    async fn inactive_registration_without_live_tokens_is_pruned() {
        let store = MemoryOAuthStore::default();
        store
            .create_client_bounded(
                OAuthClientRecord {
                    client_id: "inactive".into(),
                    client_name: None,
                    redirect_uris: vec!["http://127.0.0.1/callback".into()],
                },
                10,
            )
            .await
            .expect("registration");
        store
            .clients
            .lock()
            .unwrap()
            .get_mut("inactive")
            .expect("stored client")
            .1 = Utc::now() - chrono::Duration::days(31);

        assert_eq!(
            store
                .prune_inactive_clients(Utc::now() - chrono::Duration::days(30))
                .await
                .expect("prune"),
            1
        );
        assert!(matches!(
            store.get_client("inactive").await,
            Err(OAuthStoreError::NotFound)
        ));
    }
}
