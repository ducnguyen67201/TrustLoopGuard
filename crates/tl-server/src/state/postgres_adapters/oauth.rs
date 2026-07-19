use std::sync::Arc;

use async_trait::async_trait;
use tl_storage::{NewOAuthAuthorizationCode, NewOAuthRefreshToken, OAuthRepo, StorageError};

use crate::oauth_store::{
    OAuthAuthorizationCodeRecord, OAuthClientRecord, OAuthRefreshTokenRecord, OAuthStore,
    OAuthStoreError,
};

pub struct PostgresOAuthAdapter {
    repo: Arc<OAuthRepo>,
}

impl PostgresOAuthAdapter {
    pub fn new(repo: Arc<OAuthRepo>) -> Self {
        Self { repo }
    }
}

#[async_trait]
impl OAuthStore for PostgresOAuthAdapter {
    async fn client_count(&self) -> Result<usize, OAuthStoreError> {
        let count = self.repo.client_count().await.map_err(map_error)?;
        usize::try_from(count)
            .map_err(|_| OAuthStoreError::Internal("OAuth client count overflow".to_string()))
    }

    async fn create_client(&self, client: OAuthClientRecord) -> Result<(), OAuthStoreError> {
        self.repo
            .create_client(
                &client.client_id,
                client.client_name.as_deref(),
                &client.redirect_uris,
            )
            .await
            .map(|_| ())
            .map_err(map_error)
    }

    async fn get_client(&self, client_id: &str) -> Result<OAuthClientRecord, OAuthStoreError> {
        let client = self.repo.get_client(client_id).await.map_err(map_error)?;
        Ok(OAuthClientRecord {
            client_id: client.client_id,
            client_name: client.client_name,
            redirect_uris: client.redirect_uris,
        })
    }

    async fn put_code(
        &self,
        code_hash: String,
        record: OAuthAuthorizationCodeRecord,
    ) -> Result<(), OAuthStoreError> {
        self.repo
            .put_code(NewOAuthAuthorizationCode {
                code_hash,
                client_id: record.client_id,
                redirect_uri: record.redirect_uri,
                user_id: record.user_id,
                username: record.username,
                workspace_id: record.workspace_id,
                resource: record.resource,
                scope: record.scope,
                code_challenge: record.code_challenge,
                expires_at: record.expires_at,
            })
            .await
            .map_err(map_error)
    }

    async fn take_code(
        &self,
        code_hash: &str,
    ) -> Result<OAuthAuthorizationCodeRecord, OAuthStoreError> {
        let code = self
            .repo
            .take_code_by_hash(code_hash)
            .await
            .map_err(map_error)?;
        Ok(OAuthAuthorizationCodeRecord {
            client_id: code.client_id,
            redirect_uri: code.redirect_uri,
            user_id: code.user_id,
            username: code.username,
            workspace_id: code.workspace_id,
            resource: code.resource,
            scope: code.scope,
            code_challenge: code.code_challenge,
            expires_at: code.expires_at,
        })
    }

    async fn put_refresh(
        &self,
        token_hash: String,
        record: OAuthRefreshTokenRecord,
    ) -> Result<(), OAuthStoreError> {
        self.repo
            .put_refresh(NewOAuthRefreshToken {
                token_hash,
                client_id: record.client_id,
                user_id: record.user_id,
                username: record.username,
                workspace_id: record.workspace_id,
                resource: record.resource,
                scope: record.scope,
                expires_at: record.expires_at,
            })
            .await
            .map_err(map_error)
    }

    async fn take_refresh(
        &self,
        token_hash: &str,
    ) -> Result<OAuthRefreshTokenRecord, OAuthStoreError> {
        let refresh = self
            .repo
            .take_refresh_by_hash(token_hash)
            .await
            .map_err(map_error)?;
        Ok(OAuthRefreshTokenRecord {
            client_id: refresh.client_id,
            user_id: refresh.user_id,
            username: refresh.username,
            workspace_id: refresh.workspace_id,
            resource: refresh.resource,
            scope: refresh.scope,
            expires_at: refresh.expires_at,
        })
    }
}

fn map_error(error: StorageError) -> OAuthStoreError {
    match error {
        StorageError::NotFound => OAuthStoreError::NotFound,
        StorageError::Conflict => OAuthStoreError::Conflict,
        StorageError::Internal(message) => OAuthStoreError::Internal(message),
    }
}
