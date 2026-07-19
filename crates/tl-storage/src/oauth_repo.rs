//! Durable OAuth state for MCP dynamic clients, authorization codes, and refresh tokens.

use chrono::{DateTime, Utc};
use diesel::prelude::*;
use diesel_async::RunQueryDsl;
use uuid::Uuid;

use crate::models::{
    McpOAuthAuthorizationCodeRecord, McpOAuthClientRecord, McpOAuthRefreshTokenRecord,
    NewMcpOAuthAuthorizationCode, NewMcpOAuthClient, NewMcpOAuthRefreshToken,
};
use crate::postgres::{DbConnection, DbPool};
use crate::schema::{mcp_oauth_authorization_codes, mcp_oauth_clients, mcp_oauth_refresh_tokens};
use crate::StorageError;

#[derive(Debug, Clone)]
pub struct StoredOAuthClient {
    pub client_id: String,
    pub client_name: Option<String>,
    pub redirect_uris: Vec<String>,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone)]
pub struct NewOAuthAuthorizationCode {
    pub code_hash: String,
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
pub struct StoredOAuthAuthorizationCode {
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
pub struct NewOAuthRefreshToken {
    pub token_hash: String,
    pub client_id: String,
    pub user_id: Uuid,
    pub username: String,
    pub workspace_id: String,
    pub resource: String,
    pub scope: String,
    pub expires_at: DateTime<Utc>,
}

#[derive(Debug, Clone)]
pub struct StoredOAuthRefreshToken {
    pub client_id: String,
    pub user_id: Uuid,
    pub username: String,
    pub workspace_id: String,
    pub resource: String,
    pub scope: String,
    pub expires_at: DateTime<Utc>,
}

#[derive(Clone)]
pub struct OAuthRepo {
    pool: DbPool,
}

impl OAuthRepo {
    pub fn new(pool: DbPool) -> Self {
        Self { pool }
    }

    pub async fn client_count(&self) -> Result<i64, StorageError> {
        let mut conn = self.connection().await?;
        mcp_oauth_clients::table
            .count()
            .get_result(&mut conn)
            .await
            .map_err(Into::into)
    }

    pub async fn create_client(
        &self,
        client_id: &str,
        client_name: Option<&str>,
        redirect_uris: &[String],
    ) -> Result<StoredOAuthClient, StorageError> {
        let mut conn = self.connection().await?;
        let row = diesel::insert_into(mcp_oauth_clients::table)
            .values(NewMcpOAuthClient {
                client_id: client_id.to_string(),
                client_name: client_name.map(str::to_string),
                redirect_uris: serde_json::to_value(redirect_uris)
                    .map_err(|error| StorageError::Internal(error.to_string()))?,
            })
            .returning(McpOAuthClientRecord::as_returning())
            .get_result::<McpOAuthClientRecord>(&mut conn)
            .await?;
        map_client(row)
    }

    pub async fn get_client(&self, client_id: &str) -> Result<StoredOAuthClient, StorageError> {
        let mut conn = self.connection().await?;
        let row = mcp_oauth_clients::table
            .filter(mcp_oauth_clients::client_id.eq(client_id))
            .select(McpOAuthClientRecord::as_select())
            .first::<McpOAuthClientRecord>(&mut conn)
            .await?;
        map_client(row)
    }

    pub async fn put_code(&self, input: NewOAuthAuthorizationCode) -> Result<(), StorageError> {
        let mut conn = self.connection().await?;
        diesel::insert_into(mcp_oauth_authorization_codes::table)
            .values(NewMcpOAuthAuthorizationCode {
                code_hash: input.code_hash,
                client_id: input.client_id,
                redirect_uri: input.redirect_uri,
                user_id: input.user_id,
                username: input.username,
                workspace_id: input.workspace_id,
                resource: input.resource,
                scope: input.scope,
                code_challenge: input.code_challenge,
                expires_at: input.expires_at,
            })
            .execute(&mut conn)
            .await?;
        Ok(())
    }

    /// Atomically consumes a code. Concurrent callers have exactly one winner.
    pub async fn take_code_by_hash(
        &self,
        code_hash: &str,
    ) -> Result<StoredOAuthAuthorizationCode, StorageError> {
        let mut conn = self.connection().await?;
        let row = diesel::delete(
            mcp_oauth_authorization_codes::table
                .filter(mcp_oauth_authorization_codes::code_hash.eq(code_hash)),
        )
        .returning(McpOAuthAuthorizationCodeRecord::as_returning())
        .get_result::<McpOAuthAuthorizationCodeRecord>(&mut conn)
        .await?;
        Ok(StoredOAuthAuthorizationCode {
            client_id: row.client_id,
            redirect_uri: row.redirect_uri,
            user_id: row.user_id,
            username: row.username,
            workspace_id: row.workspace_id,
            resource: row.resource,
            scope: row.scope,
            code_challenge: row.code_challenge,
            expires_at: row.expires_at,
        })
    }

    pub async fn put_refresh(&self, input: NewOAuthRefreshToken) -> Result<(), StorageError> {
        let mut conn = self.connection().await?;
        diesel::insert_into(mcp_oauth_refresh_tokens::table)
            .values(NewMcpOAuthRefreshToken {
                token_hash: input.token_hash,
                client_id: input.client_id,
                user_id: input.user_id,
                username: input.username,
                workspace_id: input.workspace_id,
                resource: input.resource,
                scope: input.scope,
                expires_at: input.expires_at,
            })
            .execute(&mut conn)
            .await?;
        Ok(())
    }

    /// Atomically rotates a refresh token. Concurrent callers have one winner.
    pub async fn take_refresh_by_hash(
        &self,
        token_hash: &str,
    ) -> Result<StoredOAuthRefreshToken, StorageError> {
        let mut conn = self.connection().await?;
        let row = diesel::delete(
            mcp_oauth_refresh_tokens::table
                .filter(mcp_oauth_refresh_tokens::token_hash.eq(token_hash)),
        )
        .returning(McpOAuthRefreshTokenRecord::as_returning())
        .get_result::<McpOAuthRefreshTokenRecord>(&mut conn)
        .await?;
        Ok(StoredOAuthRefreshToken {
            client_id: row.client_id,
            user_id: row.user_id,
            username: row.username,
            workspace_id: row.workspace_id,
            resource: row.resource,
            scope: row.scope,
            expires_at: row.expires_at,
        })
    }

    async fn connection(&self) -> Result<DbConnection<'_>, StorageError> {
        self.pool
            .get()
            .await
            .map_err(|error| StorageError::Internal(format!("db pool: {error}")))
    }
}

fn map_client(row: McpOAuthClientRecord) -> Result<StoredOAuthClient, StorageError> {
    let redirect_uris = serde_json::from_value(row.redirect_uris)
        .map_err(|error| StorageError::Internal(format!("redirect URIs: {error}")))?;
    Ok(StoredOAuthClient {
        client_id: row.client_id,
        client_name: row.client_name,
        redirect_uris,
        created_at: row.created_at,
    })
}
