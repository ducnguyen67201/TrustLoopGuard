//! Durable regression-case storage: a trait plus an in-memory implementation
//! for tests and non-Postgres dev. The Postgres adapter lives in
//! `state::postgres_adapters::redteam_regression`.

use async_trait::async_trait;
use tl_core::{
    RegressionCaseSource, RegressionCaseSummary, RegressionExpectedOutcome,
    RegressionResultSnapshotSummary,
};
use tokio::sync::RwLock;

#[derive(Debug, thiserror::Error)]
pub enum RedteamRegressionStoreError {
    #[error("not found")]
    NotFound,
    #[error("internal: {0}")]
    Internal(String),
}

#[derive(Debug, Clone)]
pub struct NewRegressionCase {
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

#[derive(Debug, Clone)]
pub struct NewRegressionResultSnapshot {
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
pub struct RedteamRegressionCaseFilter {
    pub agent_id: Option<String>,
    pub source_job_id: Option<String>,
    pub case_keys: Vec<String>,
    pub limit: usize,
}

#[derive(Debug, Clone, Default)]
pub struct RedteamRegressionResultFilter {
    pub source_job_id: Option<String>,
    pub job_id: Option<String>,
    pub agent_id: Option<String>,
    pub limit: usize,
}

#[async_trait]
pub trait RedteamRegressionStore: Send + Sync {
    async fn upsert(
        &self,
        workspace_id: &str,
        input: NewRegressionCase,
    ) -> Result<RegressionCaseSummary, RedteamRegressionStoreError>;

    async fn list(
        &self,
        workspace_id: &str,
        filter: RedteamRegressionCaseFilter,
    ) -> Result<Vec<RegressionCaseSummary>, RedteamRegressionStoreError>;

    async fn record_result_snapshot(
        &self,
        workspace_id: &str,
        input: NewRegressionResultSnapshot,
    ) -> Result<RegressionResultSnapshotSummary, RedteamRegressionStoreError>;

    async fn list_result_snapshots(
        &self,
        workspace_id: &str,
        filter: RedteamRegressionResultFilter,
    ) -> Result<Vec<RegressionResultSnapshotSummary>, RedteamRegressionStoreError>;
}

#[derive(Default)]
pub struct MemoryRedteamRegressionStore {
    cases: RwLock<Vec<(String, RegressionCaseSummary)>>,
    snapshots: RwLock<Vec<(String, String, RegressionResultSnapshotSummary)>>,
}

impl MemoryRedteamRegressionStore {
    pub fn new() -> Self {
        Self::default()
    }
}

#[async_trait]
impl RedteamRegressionStore for MemoryRedteamRegressionStore {
    async fn upsert(
        &self,
        workspace_id: &str,
        input: NewRegressionCase,
    ) -> Result<RegressionCaseSummary, RedteamRegressionStoreError> {
        let now = chrono::Utc::now().to_rfc3339();
        let mut cases = self.cases.write().await;
        if let Some((_, existing)) = cases
            .iter_mut()
            .find(|(ws, case)| ws == workspace_id && case.case_key == input.case_key)
        {
            existing.environment_id = input.environment_id;
            existing.agent_id = input.agent_id;
            existing.source = input.source;
            existing.source_job_id = input.source_job_id;
            existing.source_session_seqs = input.source_session_seqs;
            existing.substrate = input.substrate;
            existing.artifact_id = input.artifact_id;
            existing.expected_outcome = input.expected_outcome;
            existing.attack = input.attack;
            existing.goal = input.goal;
            existing.updated_at = now;
            return Ok(existing.clone());
        }

        let case = RegressionCaseSummary {
            id: uuid::Uuid::now_v7().to_string(),
            case_key: input.case_key,
            environment_id: input.environment_id,
            agent_id: input.agent_id,
            source: input.source,
            source_job_id: input.source_job_id,
            source_session_seqs: input.source_session_seqs,
            substrate: input.substrate,
            artifact_id: input.artifact_id,
            expected_outcome: input.expected_outcome,
            attack: input.attack,
            goal: input.goal,
            created_at: now.clone(),
            updated_at: now,
        };
        cases.push((workspace_id.to_string(), case.clone()));
        Ok(case)
    }

    async fn list(
        &self,
        workspace_id: &str,
        filter: RedteamRegressionCaseFilter,
    ) -> Result<Vec<RegressionCaseSummary>, RedteamRegressionStoreError> {
        let cases = self.cases.read().await;
        let mut matched: Vec<RegressionCaseSummary> = cases
            .iter()
            .filter(|(ws, case)| {
                ws == workspace_id
                    && filter
                        .agent_id
                        .as_deref()
                        .map_or(true, |agent_id| case.agent_id.as_deref() == Some(agent_id))
                    && filter
                        .source_job_id
                        .as_deref()
                        .map_or(true, |job_id| case.source_job_id.as_deref() == Some(job_id))
                    && (filter.case_keys.is_empty()
                        || filter
                            .case_keys
                            .iter()
                            .any(|case_key| case_key == &case.case_key))
            })
            .map(|(_, case)| case.clone())
            .collect();
        matched.sort_by(|a, b| b.updated_at.cmp(&a.updated_at));
        matched.truncate(filter.limit.clamp(1, 100));
        Ok(matched)
    }

    async fn record_result_snapshot(
        &self,
        workspace_id: &str,
        input: NewRegressionResultSnapshot,
    ) -> Result<RegressionResultSnapshotSummary, RedteamRegressionStoreError> {
        let key = result_snapshot_key(&input.job_id, &input.source_job_id, &input.case_keys);
        let now = chrono::Utc::now().to_rfc3339();
        let mut snapshots = self.snapshots.write().await;
        if let Some((_, _, existing)) = snapshots
            .iter_mut()
            .find(|(ws, snapshot_key, _)| ws == workspace_id && snapshot_key == &key)
        {
            existing.environment_id = input.environment_id;
            existing.agent_id = input.agent_id;
            existing.case_keys = input.case_keys;
            existing.total = input.total;
            existing.passed = input.passed;
            existing.failed = input.failed;
            existing.missing = input.missing;
            existing.inconclusive = input.inconclusive;
            existing.updated_at = now;
            return Ok(existing.clone());
        }

        let snapshot = RegressionResultSnapshotSummary {
            id: uuid::Uuid::now_v7().to_string(),
            job_id: input.job_id,
            source_job_id: input.source_job_id,
            environment_id: input.environment_id,
            agent_id: input.agent_id,
            case_keys: input.case_keys,
            total: input.total,
            passed: input.passed,
            failed: input.failed,
            missing: input.missing,
            inconclusive: input.inconclusive,
            created_at: now.clone(),
            updated_at: now,
        };
        snapshots.push((workspace_id.to_string(), key, snapshot.clone()));
        Ok(snapshot)
    }

    async fn list_result_snapshots(
        &self,
        workspace_id: &str,
        filter: RedteamRegressionResultFilter,
    ) -> Result<Vec<RegressionResultSnapshotSummary>, RedteamRegressionStoreError> {
        let snapshots = self.snapshots.read().await;
        let mut matched: Vec<RegressionResultSnapshotSummary> = snapshots
            .iter()
            .filter(|(ws, _, snapshot)| {
                ws == workspace_id
                    && filter
                        .source_job_id
                        .as_deref()
                        .map_or(true, |job_id| snapshot.source_job_id == job_id)
                    && filter
                        .job_id
                        .as_deref()
                        .map_or(true, |job_id| snapshot.job_id == job_id)
                    && filter.agent_id.as_deref().map_or(true, |agent_id| {
                        snapshot.agent_id.as_deref() == Some(agent_id)
                    })
            })
            .map(|(_, _, snapshot)| snapshot.clone())
            .collect();
        matched.sort_by(|a, b| b.updated_at.cmp(&a.updated_at));
        matched.truncate(filter.limit.clamp(1, 100));
        Ok(matched)
    }
}

pub fn result_snapshot_key(job_id: &str, source_job_id: &str, case_keys: &[String]) -> String {
    if case_keys.is_empty() {
        return format!("{job_id}:{source_job_id}:all");
    }
    format!("{job_id}:{source_job_id}:{}", case_keys.join("\u{1f}"))
}
