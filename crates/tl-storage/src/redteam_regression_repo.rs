//! Durable regression-case storage for the evolving eval loop.
//!
//! Harden promotion writes one stable row per verified survivor. The full
//! red-team session evidence remains in `redteam_attack_sessions` and
//! `redteam_session_events`; this repo stores the suite/index row a future
//! regression runner can select and refresh idempotently.

use diesel::dsl::now;
use diesel::prelude::*;
use diesel_async::RunQueryDsl;
use tl_core::{
    RegressionCaseSource, RegressionCaseSummary, RegressionExpectedOutcome,
    RegressionResultSnapshotSummary,
};
use uuid::Uuid;

use crate::models::{
    NewRedteamRegressionCase, NewRedteamRegressionResultSnapshot, RedteamRegressionCaseRecord,
    RedteamRegressionResultSnapshotRecord,
};
use crate::postgres::{DbConnection, DbPool};
use crate::schema::{redteam_regression_cases, redteam_regression_result_snapshots};
use crate::StorageError;

#[derive(Debug, Clone)]
pub struct NewRegressionCaseParams {
    pub case_key: String,
    pub environment_id: String,
    pub agent_id: Option<String>,
    pub source: RegressionCaseSource,
    pub source_job_id: Option<String>,
    pub source_session_seqs: Vec<i32>,
    pub substrate: String,
    pub artifact_id: String,
    pub expected_outcome: RegressionExpectedOutcome,
    pub attack: String,
    pub goal: String,
}

#[derive(Debug, Clone, Default)]
pub struct RedteamRegressionCaseFilter {
    pub agent_id: Option<String>,
    pub source_job_id: Option<String>,
    pub case_keys: Vec<String>,
    pub limit: i64,
}

#[derive(Debug, Clone)]
pub struct NewRegressionResultSnapshotParams {
    pub job_id: String,
    pub source_job_id: String,
    pub environment_id: String,
    pub agent_id: Option<String>,
    pub case_keys: Vec<String>,
    pub total: u32,
    pub passed: u32,
    pub failed: u32,
    pub missing: u32,
    pub inconclusive: u32,
}

#[derive(Debug, Clone, Default)]
pub struct RedteamRegressionResultFilter {
    pub source_job_id: Option<String>,
    pub job_id: Option<String>,
    pub agent_id: Option<String>,
    pub limit: i64,
}

#[derive(Clone)]
pub struct RedteamRegressionRepo {
    pool: DbPool,
}

impl RedteamRegressionRepo {
    pub fn new(pool: DbPool) -> Self {
        Self { pool }
    }

    pub async fn upsert(
        &self,
        workspace_id: &str,
        input: NewRegressionCaseParams,
    ) -> Result<RegressionCaseSummary, StorageError> {
        let source_job_id = input.source_job_id.as_deref().map(parse_uuid).transpose()?;
        let source_session_seqs = serde_json::to_value(&input.source_session_seqs)
            .map_err(|e| StorageError::Internal(format!("regression seqs serialize: {e}")))?;
        let new_case = NewRedteamRegressionCase {
            workspace_id: workspace_id.to_string(),
            id: Uuid::now_v7(),
            case_key: input.case_key,
            environment_id: input.environment_id,
            agent_id: input.agent_id,
            source: source_to_str(input.source).to_string(),
            source_job_id,
            source_session_seqs,
            substrate: input.substrate,
            artifact_id: input.artifact_id,
            expected_outcome: expected_to_str(input.expected_outcome).to_string(),
            attack: input.attack,
            goal: input.goal,
        };
        let mut conn = self.connection().await?;
        let record = diesel::insert_into(redteam_regression_cases::table)
            .values(&new_case)
            .on_conflict((
                redteam_regression_cases::workspace_id,
                redteam_regression_cases::case_key,
            ))
            .do_update()
            .set((
                redteam_regression_cases::environment_id.eq(&new_case.environment_id),
                redteam_regression_cases::agent_id.eq(&new_case.agent_id),
                redteam_regression_cases::source.eq(&new_case.source),
                redteam_regression_cases::source_job_id.eq(new_case.source_job_id),
                redteam_regression_cases::source_session_seqs.eq(&new_case.source_session_seqs),
                redteam_regression_cases::substrate.eq(&new_case.substrate),
                redteam_regression_cases::artifact_id.eq(&new_case.artifact_id),
                redteam_regression_cases::expected_outcome.eq(&new_case.expected_outcome),
                redteam_regression_cases::attack.eq(&new_case.attack),
                redteam_regression_cases::goal.eq(&new_case.goal),
                redteam_regression_cases::updated_at.eq(now),
            ))
            .returning(RedteamRegressionCaseRecord::as_returning())
            .get_result::<RedteamRegressionCaseRecord>(&mut conn)
            .await
            .map_err(|e| StorageError::Internal(format!("regression case upsert: {e}")))?;
        case_summary(record)
    }

    pub async fn list(
        &self,
        workspace_id: &str,
        filter: RedteamRegressionCaseFilter,
    ) -> Result<Vec<RegressionCaseSummary>, StorageError> {
        let mut conn = self.connection().await?;
        let mut query = redteam_regression_cases::table
            .filter(redteam_regression_cases::workspace_id.eq(workspace_id))
            .into_boxed();
        if let Some(agent_id) = filter.agent_id {
            query = query.filter(redteam_regression_cases::agent_id.eq(agent_id));
        }
        if let Some(source_job_id) = filter.source_job_id {
            query = query
                .filter(redteam_regression_cases::source_job_id.eq(parse_uuid(&source_job_id)?));
        }
        if !filter.case_keys.is_empty() {
            query = query.filter(redteam_regression_cases::case_key.eq_any(filter.case_keys));
        }
        let records = query
            .order(redteam_regression_cases::updated_at.desc())
            .limit(filter.limit.clamp(1, 100))
            .select(RedteamRegressionCaseRecord::as_select())
            .load::<RedteamRegressionCaseRecord>(&mut conn)
            .await
            .map_err(|e| StorageError::Internal(format!("regression case list: {e}")))?;
        records.into_iter().map(case_summary).collect()
    }

    pub async fn record_result_snapshot(
        &self,
        workspace_id: &str,
        input: NewRegressionResultSnapshotParams,
    ) -> Result<RegressionResultSnapshotSummary, StorageError> {
        let job_id = parse_uuid(&input.job_id)?;
        let source_job_id = parse_uuid(&input.source_job_id)?;
        let case_keys = serde_json::to_value(&input.case_keys)
            .map_err(|e| StorageError::Internal(format!("regression case keys serialize: {e}")))?;
        let new_snapshot = NewRedteamRegressionResultSnapshot {
            workspace_id: workspace_id.to_string(),
            snapshot_key: result_snapshot_key(
                &input.job_id,
                &input.source_job_id,
                &input.case_keys,
            ),
            id: Uuid::now_v7(),
            job_id,
            source_job_id,
            environment_id: input.environment_id,
            agent_id: input.agent_id,
            case_keys,
            total: input.total as i32,
            passed: input.passed as i32,
            failed: input.failed as i32,
            missing: input.missing as i32,
            inconclusive: input.inconclusive as i32,
        };
        let mut conn = self.connection().await?;
        let record = diesel::insert_into(redteam_regression_result_snapshots::table)
            .values(&new_snapshot)
            .on_conflict((
                redteam_regression_result_snapshots::workspace_id,
                redteam_regression_result_snapshots::snapshot_key,
            ))
            .do_update()
            .set((
                redteam_regression_result_snapshots::environment_id
                    .eq(&new_snapshot.environment_id),
                redteam_regression_result_snapshots::agent_id.eq(&new_snapshot.agent_id),
                redteam_regression_result_snapshots::case_keys.eq(&new_snapshot.case_keys),
                redteam_regression_result_snapshots::total.eq(new_snapshot.total),
                redteam_regression_result_snapshots::passed.eq(new_snapshot.passed),
                redteam_regression_result_snapshots::failed.eq(new_snapshot.failed),
                redteam_regression_result_snapshots::missing.eq(new_snapshot.missing),
                redteam_regression_result_snapshots::inconclusive.eq(new_snapshot.inconclusive),
                redteam_regression_result_snapshots::updated_at.eq(now),
            ))
            .returning(RedteamRegressionResultSnapshotRecord::as_returning())
            .get_result::<RedteamRegressionResultSnapshotRecord>(&mut conn)
            .await
            .map_err(|e| {
                StorageError::Internal(format!("regression result snapshot upsert: {e}"))
            })?;
        snapshot_summary(record)
    }

    pub async fn list_result_snapshots(
        &self,
        workspace_id: &str,
        filter: RedteamRegressionResultFilter,
    ) -> Result<Vec<RegressionResultSnapshotSummary>, StorageError> {
        let mut conn = self.connection().await?;
        let mut query = redteam_regression_result_snapshots::table
            .filter(redteam_regression_result_snapshots::workspace_id.eq(workspace_id))
            .into_boxed();
        if let Some(source_job_id) = filter.source_job_id {
            query = query.filter(
                redteam_regression_result_snapshots::source_job_id.eq(parse_uuid(&source_job_id)?),
            );
        }
        if let Some(job_id) = filter.job_id {
            query =
                query.filter(redteam_regression_result_snapshots::job_id.eq(parse_uuid(&job_id)?));
        }
        if let Some(agent_id) = filter.agent_id {
            query = query.filter(redteam_regression_result_snapshots::agent_id.eq(agent_id));
        }
        let records = query
            .order(redteam_regression_result_snapshots::updated_at.desc())
            .limit(filter.limit.clamp(1, 100))
            .select(RedteamRegressionResultSnapshotRecord::as_select())
            .load::<RedteamRegressionResultSnapshotRecord>(&mut conn)
            .await
            .map_err(|e| StorageError::Internal(format!("regression result snapshot list: {e}")))?;
        records.into_iter().map(snapshot_summary).collect()
    }

    async fn connection(&self) -> Result<DbConnection<'_>, StorageError> {
        self.pool
            .get()
            .await
            .map_err(|e| StorageError::Internal(format!("db pool: {e}")))
    }
}

impl std::fmt::Debug for RedteamRegressionRepo {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("RedteamRegressionRepo")
            .finish_non_exhaustive()
    }
}

fn case_summary(
    record: RedteamRegressionCaseRecord,
) -> Result<RegressionCaseSummary, StorageError> {
    let source_session_seqs: Vec<i32> = serde_json::from_value(record.source_session_seqs)
        .map_err(|e| StorageError::Internal(format!("regression seqs deserialize: {e}")))?;
    Ok(RegressionCaseSummary {
        id: record.id.to_string(),
        case_key: record.case_key,
        environment_id: record.environment_id,
        agent_id: record.agent_id,
        source: parse_source(&record.source)?,
        source_job_id: record.source_job_id.map(|id| id.to_string()),
        source_session_seqs,
        substrate: record.substrate,
        artifact_id: record.artifact_id,
        expected_outcome: parse_expected(&record.expected_outcome)?,
        attack: record.attack,
        goal: record.goal,
        created_at: record.created_at.to_rfc3339(),
        updated_at: record.updated_at.to_rfc3339(),
    })
}

fn snapshot_summary(
    record: RedteamRegressionResultSnapshotRecord,
) -> Result<RegressionResultSnapshotSummary, StorageError> {
    let case_keys: Vec<String> = serde_json::from_value(record.case_keys)
        .map_err(|e| StorageError::Internal(format!("regression case keys deserialize: {e}")))?;
    Ok(RegressionResultSnapshotSummary {
        id: record.id.to_string(),
        job_id: record.job_id.to_string(),
        source_job_id: record.source_job_id.to_string(),
        environment_id: record.environment_id,
        agent_id: record.agent_id,
        case_keys,
        total: record.total as u32,
        passed: record.passed as u32,
        failed: record.failed as u32,
        missing: record.missing as u32,
        inconclusive: record.inconclusive as u32,
        created_at: record.created_at.to_rfc3339(),
        updated_at: record.updated_at.to_rfc3339(),
    })
}

fn parse_uuid(value: &str) -> Result<Uuid, StorageError> {
    Uuid::parse_str(value).map_err(|_| StorageError::Internal("invalid source job id".into()))
}

fn result_snapshot_key(job_id: &str, source_job_id: &str, case_keys: &[String]) -> String {
    if case_keys.is_empty() {
        return format!("{job_id}:{source_job_id}:all");
    }
    format!("{job_id}:{source_job_id}:{}", case_keys.join("\u{1f}"))
}

fn source_to_str(source: RegressionCaseSource) -> &'static str {
    match source {
        RegressionCaseSource::Harden => "harden",
        RegressionCaseSource::Manual => "manual",
    }
}

fn parse_source(value: &str) -> Result<RegressionCaseSource, StorageError> {
    match value {
        "harden" => Ok(RegressionCaseSource::Harden),
        "manual" => Ok(RegressionCaseSource::Manual),
        other => Err(StorageError::Internal(format!(
            "unknown regression case source: {other}"
        ))),
    }
}

fn expected_to_str(expected: RegressionExpectedOutcome) -> &'static str {
    match expected {
        RegressionExpectedOutcome::Block => "block",
        RegressionExpectedOutcome::Escalate => "escalate",
        RegressionExpectedOutcome::Stop => "stop",
    }
}

fn parse_expected(value: &str) -> Result<RegressionExpectedOutcome, StorageError> {
    match value {
        "block" => Ok(RegressionExpectedOutcome::Block),
        "escalate" => Ok(RegressionExpectedOutcome::Escalate),
        "stop" => Ok(RegressionExpectedOutcome::Stop),
        other => Err(StorageError::Internal(format!(
            "unknown regression expected outcome: {other}"
        ))),
    }
}
