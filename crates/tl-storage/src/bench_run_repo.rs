//! Durable storage for TrustLoopGuardBench parent runs and arm mappings.

use diesel::dsl::now;
use diesel::prelude::*;
use diesel_async::RunQueryDsl;
use tl_core::{
    BenchArm, BenchRunArmSummary, BenchRunCreateRequest, BenchRunDetail, BenchRunStatus,
    BenchRunSummary, RedteamGenerator,
};
use uuid::Uuid;

use crate::models::{BenchRunArmRecord, BenchRunRecord, NewBenchRun, NewBenchRunArm};
use crate::postgres::{DbConnection, DbPool};
use crate::schema::{bench_run_arms, bench_runs};
use crate::StorageError;

#[derive(Clone)]
pub struct BenchRunRepo {
    pool: DbPool,
}

#[derive(Debug, Default)]
pub struct BenchRunFilter {
    pub limit: i64,
}

#[derive(Debug, Clone)]
pub struct BenchRunArmRowInput {
    pub arm: String,
    pub label: String,
    pub target: String,
    pub redteam_job_id: Option<String>,
    pub checker_config: Option<String>,
}

impl BenchRunRepo {
    pub fn new(pool: DbPool) -> Self {
        Self { pool }
    }

    pub async fn create(
        &self,
        workspace_id: &str,
        environment_id: &str,
        request: &BenchRunCreateRequest,
    ) -> Result<BenchRunSummary, StorageError> {
        let id = Uuid::now_v7();
        let generator = request.generator.unwrap_or(RedteamGenerator::Deterministic);
        let new_run = NewBenchRun {
            workspace_id: workspace_id.to_string(),
            id,
            environment_id: environment_id.to_string(),
            status: status_text(BenchRunStatus::Queued).to_string(),
            profile: request.profile.clone(),
            generator: generator_text(generator).to_string(),
            agent_id: clean_optional(request.agent_id.as_deref()),
            seed: clean_optional(request.seed.as_deref()),
        };
        let mut conn = self.connection().await?;
        diesel::insert_into(bench_runs::table)
            .values(&new_run)
            .execute(&mut conn)
            .await
            .map_err(|e| StorageError::Internal(format!("bench run create: {e}")))?;
        self.get(workspace_id, &id.to_string()).await
    }

    pub async fn list(
        &self,
        workspace_id: &str,
        filter: BenchRunFilter,
    ) -> Result<Vec<BenchRunSummary>, StorageError> {
        let mut conn = self.connection().await?;
        let records = bench_runs::table
            .filter(bench_runs::workspace_id.eq(workspace_id))
            .select(BenchRunRecord::as_select())
            .order(bench_runs::created_at.desc())
            .limit(filter.limit.clamp(1, 100))
            .load::<BenchRunRecord>(&mut conn)
            .await
            .map_err(|e| StorageError::Internal(format!("bench run list: {e}")))?;
        Ok(records.into_iter().map(run_summary).collect())
    }

    pub async fn get(
        &self,
        workspace_id: &str,
        run_id: &str,
    ) -> Result<BenchRunSummary, StorageError> {
        let id = parse_uuid(run_id)?;
        let mut conn = self.connection().await?;
        let record = bench_runs::table
            .filter(bench_runs::workspace_id.eq(workspace_id))
            .filter(bench_runs::id.eq(id))
            .select(BenchRunRecord::as_select())
            .first::<BenchRunRecord>(&mut conn)
            .await
            .optional()
            .map_err(|e| StorageError::Internal(format!("bench run get: {e}")))?
            .ok_or(StorageError::NotFound)?;
        Ok(run_summary(record))
    }

    pub async fn get_detail(
        &self,
        workspace_id: &str,
        run_id: &str,
    ) -> Result<BenchRunDetail, StorageError> {
        let run = self.get(workspace_id, run_id).await?;
        let arms = self.list_arms(workspace_id, run_id).await?;
        Ok(BenchRunDetail {
            run,
            arms,
            raw_job: None,
            guarded_job: None,
        })
    }

    pub async fn attach_arm(
        &self,
        workspace_id: &str,
        run_id: &str,
        input: BenchRunArmRowInput,
    ) -> Result<BenchRunArmSummary, StorageError> {
        let run_uuid = parse_uuid(run_id)?;
        // Verify workspace/run existence before inserting an arm because the
        // migration intentionally avoids a cross-table FK with composite workspace key.
        let _ = self.get(workspace_id, run_id).await?;
        let new_arm = NewBenchRunArm {
            workspace_id: workspace_id.to_string(),
            run_id: run_uuid,
            arm: input.arm,
            label: input.label,
            target: input.target,
            redteam_job_id: input
                .redteam_job_id
                .as_deref()
                .map(parse_uuid)
                .transpose()?,
            checker_config: clean_optional(input.checker_config.as_deref()),
        };
        let mut conn = self.connection().await?;
        diesel::insert_into(bench_run_arms::table)
            .values(&new_arm)
            .on_conflict((
                bench_run_arms::workspace_id,
                bench_run_arms::run_id,
                bench_run_arms::arm,
            ))
            .do_update()
            .set((
                bench_run_arms::label.eq(new_arm.label.clone()),
                bench_run_arms::target.eq(new_arm.target.clone()),
                bench_run_arms::redteam_job_id.eq(new_arm.redteam_job_id),
                bench_run_arms::checker_config.eq(new_arm.checker_config.clone()),
                bench_run_arms::updated_at.eq(now),
            ))
            .execute(&mut conn)
            .await
            .map_err(|e| StorageError::Internal(format!("bench arm attach: {e}")))?;
        self.get_arm(workspace_id, run_id, &new_arm.arm).await
    }

    pub async fn set_status(
        &self,
        workspace_id: &str,
        run_id: &str,
        status: BenchRunStatus,
        error: Option<&str>,
    ) -> Result<(), StorageError> {
        let id = parse_uuid(run_id)?;
        let mut conn = self.connection().await?;
        diesel::update(
            bench_runs::table
                .filter(bench_runs::workspace_id.eq(workspace_id))
                .filter(bench_runs::id.eq(id))
                .filter(bench_runs::status.ne_all(TERMINAL_STATUSES)),
        )
        .set((
            bench_runs::status.eq(status_text(status)),
            bench_runs::error.eq(error.map(str::to_string)),
            bench_runs::updated_at.eq(now),
        ))
        .execute(&mut conn)
        .await
        .map_err(|e| StorageError::Internal(format!("bench run status: {e}")))?;
        Ok(())
    }

    async fn list_arms(
        &self,
        workspace_id: &str,
        run_id: &str,
    ) -> Result<Vec<BenchRunArmSummary>, StorageError> {
        let id = parse_uuid(run_id)?;
        let mut conn = self.connection().await?;
        let records = bench_run_arms::table
            .filter(bench_run_arms::workspace_id.eq(workspace_id))
            .filter(bench_run_arms::run_id.eq(id))
            .select(BenchRunArmRecord::as_select())
            .order(bench_run_arms::created_at.asc())
            .load::<BenchRunArmRecord>(&mut conn)
            .await
            .map_err(|e| StorageError::Internal(format!("bench arm list: {e}")))?;
        Ok(records.into_iter().map(arm_summary).collect())
    }

    async fn get_arm(
        &self,
        workspace_id: &str,
        run_id: &str,
        arm: &str,
    ) -> Result<BenchRunArmSummary, StorageError> {
        let id = parse_uuid(run_id)?;
        let mut conn = self.connection().await?;
        let record = bench_run_arms::table
            .filter(bench_run_arms::workspace_id.eq(workspace_id))
            .filter(bench_run_arms::run_id.eq(id))
            .filter(bench_run_arms::arm.eq(arm))
            .select(BenchRunArmRecord::as_select())
            .first::<BenchRunArmRecord>(&mut conn)
            .await
            .optional()
            .map_err(|e| StorageError::Internal(format!("bench arm get: {e}")))?
            .ok_or(StorageError::NotFound)?;
        Ok(arm_summary(record))
    }

    async fn connection(&self) -> Result<DbConnection<'_>, StorageError> {
        self.pool
            .get()
            .await
            .map_err(|e| StorageError::Internal(format!("db pool: {e}")))
    }
}

impl std::fmt::Debug for BenchRunRepo {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("BenchRunRepo").finish_non_exhaustive()
    }
}

const TERMINAL_STATUSES: [&str; 3] = ["complete", "error", "cancelled"];

fn parse_uuid(value: &str) -> Result<Uuid, StorageError> {
    Uuid::parse_str(value).map_err(|_| StorageError::NotFound)
}

fn status_text(status: BenchRunStatus) -> &'static str {
    match status {
        BenchRunStatus::Queued => "queued",
        BenchRunStatus::Running => "running",
        BenchRunStatus::Complete => "complete",
        BenchRunStatus::Error => "error",
        BenchRunStatus::Cancelled => "cancelled",
    }
}

fn parse_status(text: &str) -> BenchRunStatus {
    match text {
        "running" => BenchRunStatus::Running,
        "complete" => BenchRunStatus::Complete,
        "error" => BenchRunStatus::Error,
        "cancelled" => BenchRunStatus::Cancelled,
        _ => BenchRunStatus::Queued,
    }
}

fn parse_arm(text: &str) -> BenchArm {
    match text {
        "guarded" => BenchArm::Guarded,
        _ => BenchArm::Raw,
    }
}

fn generator_text(generator: RedteamGenerator) -> &'static str {
    match generator {
        RedteamGenerator::Deterministic => "deterministic",
        RedteamGenerator::Hackagent => "hackagent",
    }
}

fn parse_generator(text: &str) -> RedteamGenerator {
    match text {
        "hackagent" => RedteamGenerator::Hackagent,
        _ => RedteamGenerator::Deterministic,
    }
}

fn clean_optional(value: Option<&str>) -> Option<String> {
    value
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(str::to_string)
}

fn run_summary(record: BenchRunRecord) -> BenchRunSummary {
    BenchRunSummary {
        id: record.id.to_string(),
        workspace_id: record.workspace_id,
        environment_id: record.environment_id,
        status: parse_status(&record.status),
        profile: record.profile,
        generator: parse_generator(&record.generator),
        agent_id: record.agent_id,
        seed: record.seed,
        error: record.error,
        created_at: record.created_at.to_rfc3339(),
        updated_at: record.updated_at.to_rfc3339(),
    }
}

fn arm_summary(record: BenchRunArmRecord) -> BenchRunArmSummary {
    BenchRunArmSummary {
        run_id: record.run_id.to_string(),
        arm: parse_arm(&record.arm),
        label: record.label,
        target: record.target,
        redteam_job_id: record.redteam_job_id.map(|id| id.to_string()),
        checker_config: record.checker_config,
        created_at: record.created_at.to_rfc3339(),
        updated_at: record.updated_at.to_rfc3339(),
    }
}
