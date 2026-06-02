use std::sync::Arc;

use anyhow::Result;
use async_trait::async_trait;
use tl_storage::UserRepo;

use crate::auth_user::{UserRecord, UserStore, UserStoreError};

pub struct PostgresUserAdapter(pub Arc<UserRepo>);

impl PostgresUserAdapter {
    pub fn new(repo: Arc<UserRepo>) -> Arc<Self> {
        Arc::new(Self(repo))
    }
}

#[async_trait]
impl UserStore for PostgresUserAdapter {
    async fn create(
        &self,
        username: &str,
        password_hash: &str,
    ) -> Result<UserRecord, UserStoreError> {
        let row = self
            .0
            .create(username, password_hash)
            .await
            .map_err(user_store_create_error)?;
        Ok(user_record_from_row(row))
    }

    async fn find_by_username(&self, username: &str) -> Result<UserRecord, UserStoreError> {
        let row = self
            .0
            .find_by_username(username)
            .await
            .map_err(user_store_not_found_error)?;
        Ok(user_record_from_row(row))
    }

    async fn is_approved(&self, id: uuid::Uuid) -> Result<bool, UserStoreError> {
        self.0
            .is_approved(id)
            .await
            .map_err(user_store_not_found_error)
    }

    async fn ensure_oauth_identity(
        &self,
        provider: &str,
        provider_subject: &str,
        email: &str,
    ) -> Result<UserRecord, UserStoreError> {
        let row = self
            .0
            .ensure_oauth_identity(provider, provider_subject, email)
            .await
            .map_err(user_store_oauth_error)?;
        Ok(user_record_from_row(row))
    }

    async fn update_password(
        &self,
        id: uuid::Uuid,
        password_hash: &str,
    ) -> Result<(), UserStoreError> {
        self.0
            .update_password(id, password_hash)
            .await
            .map_err(user_store_not_found_error)
    }
}

fn user_record_from_row(row: tl_storage::UserRecord) -> UserRecord {
    UserRecord {
        id: row.id,
        username: row.username,
        password_hash: row.password_hash,
        is_approved: row.is_approved,
    }
}

fn user_store_create_error(error: tl_storage::StorageError) -> UserStoreError {
    match error {
        tl_storage::StorageError::Conflict => UserStoreError::Conflict,
        other => UserStoreError::Internal(other.to_string()),
    }
}

fn user_store_not_found_error(error: tl_storage::StorageError) -> UserStoreError {
    match error {
        tl_storage::StorageError::NotFound => UserStoreError::NotFound,
        other => UserStoreError::Internal(other.to_string()),
    }
}

fn user_store_oauth_error(error: tl_storage::StorageError) -> UserStoreError {
    match error {
        tl_storage::StorageError::NotFound => UserStoreError::NotFound,
        tl_storage::StorageError::Conflict => UserStoreError::Conflict,
        other => UserStoreError::Internal(other.to_string()),
    }
}
