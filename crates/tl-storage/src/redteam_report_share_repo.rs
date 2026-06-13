//! Durable storage for red-team report share tokens.
//!
//! A share is a capability granting public, read-only access to one report.
//! Validity is enforced in SQL (`get_by_token` filters revoked/expired rows), so
//! a missing, expired, or revoked token is indistinguishable to callers — the
//! public endpoint is not a token-validity oracle.

use chrono::{DateTime, Utc};
use diesel::dsl::now;
use diesel::prelude::*;
use diesel_async::RunQueryDsl;
use uuid::Uuid;

use crate::models::{NewRedteamReportShare, RedteamReportShareRecord};
use crate::postgres::{DbConnection, DbPool};
use crate::schema::redteam_report_shares;
use crate::StorageError;

#[derive(Clone)]
pub struct RedteamReportShareRepo {
    pool: DbPool,
}

/// Inputs for minting a share. Timestamps are computed by the caller.
pub struct NewShare<'a> {
    pub token: &'a str,
    pub workspace_id: &'a str,
    pub job_id: &'a str,
    pub compare_job_id: Option<&'a str>,
    pub expires_at: Option<DateTime<Utc>>,
}

/// A valid (non-revoked, non-expired) share. Timestamps are RFC 3339 strings to
/// match the rest of the red-team wire layer.
#[derive(Debug, Clone)]
pub struct ReportShareRow {
    pub token: String,
    pub workspace_id: String,
    pub job_id: String,
    pub compare_job_id: Option<String>,
    pub created_at: String,
    pub expires_at: Option<String>,
}

impl RedteamReportShareRepo {
    pub fn new(pool: DbPool) -> Self {
        Self { pool }
    }

    pub async fn create(&self, new: NewShare<'_>) -> Result<ReportShareRow, StorageError> {
        let row = NewRedteamReportShare {
            token: new.token.to_string(),
            workspace_id: new.workspace_id.to_string(),
            job_id: parse_uuid(new.job_id)?,
            compare_job_id: new.compare_job_id.map(parse_uuid).transpose()?,
            expires_at: new.expires_at,
        };
        let mut conn = self.connection().await?;
        diesel::insert_into(redteam_report_shares::table)
            .values(&row)
            .execute(&mut conn)
            .await
            .map_err(|e| StorageError::Internal(format!("report share create: {e}")))?;
        self.get_by_token(new.token).await
    }

    /// Resolve a share that is neither revoked nor expired. Anything else
    /// (missing, revoked, expired) returns `NotFound`.
    pub async fn get_by_token(&self, token: &str) -> Result<ReportShareRow, StorageError> {
        let mut conn = self.connection().await?;
        let record = redteam_report_shares::table
            .filter(redteam_report_shares::token.eq(token))
            .filter(redteam_report_shares::revoked_at.is_null())
            .filter(
                redteam_report_shares::expires_at
                    .is_null()
                    .or(redteam_report_shares::expires_at.gt(now)),
            )
            .select(RedteamReportShareRecord::as_select())
            .first::<RedteamReportShareRecord>(&mut conn)
            .await
            .optional()
            .map_err(|e| StorageError::Internal(format!("report share get: {e}")))?
            .ok_or(StorageError::NotFound)?;
        Ok(share_row(record))
    }

    /// Revoke a share within its workspace. Idempotent on an already-revoked
    /// row would re-match nothing, so a missing/foreign/already-revoked token
    /// returns `NotFound`.
    pub async fn revoke(&self, workspace_id: &str, token: &str) -> Result<(), StorageError> {
        let mut conn = self.connection().await?;
        let affected = diesel::update(
            redteam_report_shares::table
                .filter(redteam_report_shares::token.eq(token))
                .filter(redteam_report_shares::workspace_id.eq(workspace_id))
                .filter(redteam_report_shares::revoked_at.is_null()),
        )
        .set(redteam_report_shares::revoked_at.eq(now))
        .execute(&mut conn)
        .await
        .map_err(|e| StorageError::Internal(format!("report share revoke: {e}")))?;
        if affected == 0 {
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

impl std::fmt::Debug for RedteamReportShareRepo {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("RedteamReportShareRepo")
            .finish_non_exhaustive()
    }
}

fn parse_uuid(value: &str) -> Result<Uuid, StorageError> {
    Uuid::parse_str(value).map_err(|_| StorageError::NotFound)
}

fn share_row(record: RedteamReportShareRecord) -> ReportShareRow {
    ReportShareRow {
        token: record.token,
        workspace_id: record.workspace_id,
        job_id: record.job_id.to_string(),
        compare_job_id: record.compare_job_id.map(|id| id.to_string()),
        created_at: record.created_at.to_rfc3339(),
        expires_at: record.expires_at.map(|ts| ts.to_rfc3339()),
    }
}
