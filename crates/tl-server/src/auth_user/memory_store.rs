use async_trait::async_trait;
use tokio::sync::RwLock;
use uuid::Uuid;

use super::{normalize_oauth_provider, UserRecord, UserStore, UserStoreError};

/// Process-local store. Useful for local dev, tests, and the no-DB
/// boot path. Not durable across restarts.
#[derive(Debug, Default)]
pub struct MemoryUserStore {
    inner: RwLock<std::collections::HashMap<String, UserRecord>>,
    oauth_identities: RwLock<std::collections::HashMap<(String, String), Uuid>>,
}

impl MemoryUserStore {
    pub fn new() -> Self {
        Self::default()
    }

    pub async fn create_approved_for_tests(
        &self,
        username: &str,
    ) -> Result<UserRecord, UserStoreError> {
        self.insert_approved_for_tests(Uuid::new_v4(), username)
            .await
    }

    pub async fn insert_approved_for_tests(
        &self,
        id: Uuid,
        username: &str,
    ) -> Result<UserRecord, UserStoreError> {
        let key = username.to_ascii_lowercase();
        let mut guard = self.inner.write().await;
        if guard.contains_key(&key) {
            return Err(UserStoreError::Conflict);
        }
        let record = UserRecord {
            id,
            username: username.to_string(),
            password_hash: "test:approved-user".to_string(),
            is_approved: true,
        };
        guard.insert(key, record.clone());
        Ok(record)
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
            is_approved: false,
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

    async fn is_approved(&self, id: Uuid) -> Result<bool, UserStoreError> {
        self.inner
            .read()
            .await
            .values()
            .find(|record| record.id == id)
            .map(|record| record.is_approved)
            .ok_or(UserStoreError::NotFound)
    }

    async fn ensure_oauth_identity(
        &self,
        provider: &str,
        provider_subject: &str,
        email: &str,
    ) -> Result<UserRecord, UserStoreError> {
        let provider = normalize_oauth_provider(provider)?;
        let subject = provider_subject.trim();
        let email = email.trim();
        if subject.is_empty() {
            return Err(UserStoreError::Internal(
                "provider subject is required".into(),
            ));
        }
        if email.is_empty() {
            return Err(UserStoreError::Internal("email is required".into()));
        }

        let identity_key = (provider, subject.to_string());
        if let Some(user_id) = self
            .oauth_identities
            .read()
            .await
            .get(&identity_key)
            .copied()
        {
            let users = self.inner.read().await;
            if let Some(record) = users.values().find(|record| record.id == user_id) {
                return Ok(record.clone());
            }
        }

        let username_key = email.to_ascii_lowercase();
        let mut users = self.inner.write().await;
        let record = match users.get(&username_key) {
            Some(record) => record.clone(),
            None => {
                let record = UserRecord {
                    id: Uuid::new_v4(),
                    username: email.to_string(),
                    password_hash: "oauth:external-provider".to_string(),
                    is_approved: false,
                };
                users.insert(username_key, record.clone());
                record
            }
        };
        drop(users);

        self.oauth_identities
            .write()
            .await
            .insert(identity_key, record.id);
        Ok(record)
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
