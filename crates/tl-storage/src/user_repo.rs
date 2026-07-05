//! User account repository for username/password auth.
//!
//! No cache: login is rare and the `users_username_idx` covers the
//! hot lookup. Usernames are stored as-given but matched case-
//! insensitively (`LOWER(username) = LOWER($1)`) — the unique index
//! is on `LOWER(username)`.

use diesel::prelude::*;
use diesel_async::{AsyncConnection, RunQueryDsl};
use uuid::Uuid;

use crate::models::{NewOAuthIdentity, NewUser, UserRecord};
use crate::postgres::{DbConnection, DbPool};
use crate::schema::{oauth_identities, users};
use crate::StorageError;

#[derive(Clone)]
pub struct UserRepo {
    pool: DbPool,
}

impl UserRepo {
    pub fn new(pool: DbPool) -> Self {
        Self { pool }
    }

    /// Insert a new user. Returns `StorageError::Conflict` if the
    /// username (case-insensitive) is already taken.
    pub async fn create(
        &self,
        username: &str,
        password_hash: &str,
    ) -> Result<UserRecord, StorageError> {
        let new_user = NewUser {
            id: Uuid::new_v4(),
            username: username.to_string(),
            password_hash: password_hash.to_string(),
        };
        let mut conn = self.connection().await?;
        diesel::insert_into(users::table)
            .values(&new_user)
            .returning(UserRecord::as_returning())
            .get_result(&mut conn)
            .await
            .map_err(map_insert_err)
    }

    /// Resolve a username (case-insensitive) to its row. `NotFound`
    /// when no match.
    pub async fn find_by_username(&self, username: &str) -> Result<UserRecord, StorageError> {
        let mut conn = self.connection().await?;
        let lowered = username.to_ascii_lowercase();
        users::table
            .filter(
                diesel::dsl::sql::<diesel::sql_types::Bool>("LOWER(username) = ")
                    .bind::<diesel::sql_types::Text, _>(lowered),
            )
            .select(UserRecord::as_select())
            .first(&mut conn)
            .await
            .optional()
            .map_err(|e| StorageError::Internal(format!("user lookup: {e}")))?
            .ok_or(StorageError::NotFound)
    }

    /// Return whether a local app user has been approved for dashboard access.
    pub async fn is_approved(&self, id: Uuid) -> Result<bool, StorageError> {
        let mut conn = self.connection().await?;
        users::table
            .filter(users::id.eq(id))
            .select(users::is_approved)
            .first(&mut conn)
            .await
            .optional()
            .map_err(|e| StorageError::Internal(format!("user approval lookup: {e}")))?
            .ok_or(StorageError::NotFound)
    }

    /// Update the password hash for an existing user. Returns
    /// `NotFound` if the id doesn't match a row.
    pub async fn update_password(&self, id: Uuid, password_hash: &str) -> Result<(), StorageError> {
        let mut conn = self.connection().await?;
        let rows = diesel::update(users::table.filter(users::id.eq(id)))
            .set((
                users::password_hash.eq(password_hash),
                users::updated_at.eq(diesel::dsl::now),
            ))
            .execute(&mut conn)
            .await
            .map_err(|e| StorageError::Internal(format!("user update: {e}")))?;
        if rows == 0 {
            return Err(StorageError::NotFound);
        }
        Ok(())
    }

    /// Resolve an OAuth identity to a local app user. The provider has
    /// already authenticated the browser user; this only establishes
    /// which TrustLoopGuard `users.id` owns app memberships.
    pub async fn ensure_oauth_identity(
        &self,
        provider: &str,
        provider_subject: &str,
        email: &str,
    ) -> Result<UserRecord, StorageError> {
        let provider = normalize_provider(provider)?;
        let subject = provider_subject.trim();
        let email = email.trim();
        if subject.is_empty() {
            return Err(StorageError::Internal(
                "provider subject is required".to_string(),
            ));
        }
        if email.is_empty() {
            return Err(StorageError::Internal("email is required".to_string()));
        }

        let mut conn = self.connection().await?;
        conn.transaction::<UserRecord, StorageError, _>(async |conn| {
            if let Some(user) = find_user_by_oauth(conn, &provider, subject).await? {
                return Ok(user);
            }

            let user = match find_user_by_username_conn(conn, email).await? {
                Some(user) => user,
                None => {
                    let new_user = NewUser {
                        id: Uuid::new_v4(),
                        username: email.to_string(),
                        password_hash: "oauth:external-provider".to_string(),
                    };
                    diesel::insert_into(users::table)
                        .values(&new_user)
                        .returning(UserRecord::as_returning())
                        .get_result(conn)
                        .await
                        .map_err(map_insert_err)?
                }
            };

            let identity = NewOAuthIdentity {
                provider: provider.clone(),
                provider_subject: subject.to_string(),
                user_id: user.id,
                email: email.to_string(),
            };
            diesel::insert_into(oauth_identities::table)
                .values(&identity)
                .on_conflict((
                    oauth_identities::provider,
                    oauth_identities::provider_subject,
                ))
                .do_update()
                .set((
                    oauth_identities::user_id.eq(user.id),
                    oauth_identities::email.eq(email),
                    oauth_identities::updated_at.eq(diesel::dsl::now),
                ))
                .execute(conn)
                .await
                .map_err(|e| StorageError::Internal(format!("oauth identity upsert: {e}")))?;

            Ok(user)
        })
        .await
    }

    async fn connection(&self) -> Result<DbConnection<'_>, StorageError> {
        self.pool
            .get()
            .await
            .map_err(|e| StorageError::Internal(format!("db pool: {e}")))
    }
}

fn normalize_provider(provider: &str) -> Result<String, StorageError> {
    let normalized = provider.trim().to_ascii_lowercase();
    match normalized.as_str() {
        "google" | "github" => Ok(normalized),
        _ => Err(StorageError::Internal(format!(
            "unsupported oauth provider: {provider}"
        ))),
    }
}

async fn find_user_by_oauth(
    conn: &mut DbConnection<'_>,
    provider: &str,
    provider_subject: &str,
) -> Result<Option<UserRecord>, StorageError> {
    let row = oauth_identities::table
        .inner_join(users::table.on(users::id.eq(oauth_identities::user_id)))
        .filter(oauth_identities::provider.eq(provider))
        .filter(oauth_identities::provider_subject.eq(provider_subject))
        .select(UserRecord::as_select())
        .first(conn)
        .await
        .optional()
        .map_err(|e| StorageError::Internal(format!("oauth user lookup: {e}")))?;
    Ok(row)
}

async fn find_user_by_username_conn(
    conn: &mut DbConnection<'_>,
    username: &str,
) -> Result<Option<UserRecord>, StorageError> {
    let lowered = username.to_ascii_lowercase();
    users::table
        .filter(
            diesel::dsl::sql::<diesel::sql_types::Bool>("LOWER(username) = ")
                .bind::<diesel::sql_types::Text, _>(lowered),
        )
        .select(UserRecord::as_select())
        .first(conn)
        .await
        .optional()
        .map_err(|e| StorageError::Internal(format!("user lookup: {e}")))
}

fn map_insert_err(e: diesel::result::Error) -> StorageError {
    match e {
        diesel::result::Error::DatabaseError(
            diesel::result::DatabaseErrorKind::UniqueViolation,
            _,
        ) => StorageError::Conflict,
        other => StorageError::Internal(format!("user insert: {other}")),
    }
}
