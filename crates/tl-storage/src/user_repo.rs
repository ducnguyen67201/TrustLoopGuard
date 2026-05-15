//! User account repository for username/password auth.
//!
//! No cache: login is rare and the `users_username_idx` covers the
//! hot lookup. Usernames are stored as-given but matched case-
//! insensitively (`LOWER(username) = LOWER($1)`) — the unique index
//! is on `LOWER(username)`.

use diesel::prelude::*;
use diesel_async::RunQueryDsl;
use uuid::Uuid;

use crate::models::{NewUser, UserRecord};
use crate::postgres::{DbConnection, DbPool};
use crate::schema::users;
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

    async fn connection(&self) -> Result<DbConnection<'_>, StorageError> {
        self.pool
            .get()
            .await
            .map_err(|e| StorageError::Internal(format!("db pool: {e}")))
    }
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
