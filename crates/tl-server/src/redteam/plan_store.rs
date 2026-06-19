//! Saved attack-plan storage: the trait plus an in-memory impl for tests and
//! non-Postgres dev. The Postgres adapter lives in
//! `state::postgres_adapters::redteam_plan`. Mirrors `RedteamJobStore`.

use async_trait::async_trait;
use tl_core::{AttackVector, RedteamPlanResponse, WorkflowPath};
use tokio::sync::RwLock;

#[derive(Debug, thiserror::Error)]
pub enum RedteamPlanStoreError {
    #[error("not found")]
    NotFound,
    #[error("internal: {0}")]
    Internal(String),
}

/// Durable storage for saved, named attack plans (per agent).
#[async_trait]
pub trait RedteamPlanStore: Send + Sync {
    #[allow(clippy::too_many_arguments)]
    async fn create(
        &self,
        workspace_id: &str,
        environment_id: &str,
        agent_id: &str,
        name: &str,
        vectors: Vec<AttackVector>,
        paths: Vec<WorkflowPath>,
        unmapped_node_types: Vec<String>,
    ) -> Result<RedteamPlanResponse, RedteamPlanStoreError>;
    /// Saved plans for one agent, newest first (capped at `limit`).
    async fn list(
        &self,
        workspace_id: &str,
        agent_id: &str,
        limit: usize,
    ) -> Result<Vec<RedteamPlanResponse>, RedteamPlanStoreError>;
    async fn delete(&self, workspace_id: &str, plan_id: &str) -> Result<(), RedteamPlanStoreError>;
}

/// In-memory plan store. Rows are (workspace_id, plan); insertion order is
/// chronological, so reverse iteration yields newest-first.
#[derive(Default)]
pub struct MemoryRedteamPlanStore {
    plans: RwLock<Vec<(String, RedteamPlanResponse)>>,
}

impl MemoryRedteamPlanStore {
    pub fn new() -> Self {
        Self::default()
    }
}

#[async_trait]
impl RedteamPlanStore for MemoryRedteamPlanStore {
    #[allow(clippy::too_many_arguments)]
    async fn create(
        &self,
        workspace_id: &str,
        _environment_id: &str,
        agent_id: &str,
        name: &str,
        vectors: Vec<AttackVector>,
        paths: Vec<WorkflowPath>,
        unmapped_node_types: Vec<String>,
    ) -> Result<RedteamPlanResponse, RedteamPlanStoreError> {
        let plan = RedteamPlanResponse {
            id: uuid::Uuid::now_v7().to_string(),
            agent_id: agent_id.to_string(),
            name: name.to_string(),
            vectors,
            paths,
            unmapped_node_types,
            generated_at: chrono::Utc::now().to_rfc3339(),
        };
        self.plans
            .write()
            .await
            .push((workspace_id.to_string(), plan.clone()));
        Ok(plan)
    }

    async fn list(
        &self,
        workspace_id: &str,
        agent_id: &str,
        limit: usize,
    ) -> Result<Vec<RedteamPlanResponse>, RedteamPlanStoreError> {
        let plans = self.plans.read().await;
        let mut matched: Vec<RedteamPlanResponse> = plans
            .iter()
            .filter(|(ws, plan)| ws == workspace_id && plan.agent_id == agent_id)
            .map(|(_, plan)| plan.clone())
            .collect();
        matched.reverse();
        matched.truncate(limit);
        Ok(matched)
    }

    async fn delete(&self, workspace_id: &str, plan_id: &str) -> Result<(), RedteamPlanStoreError> {
        let mut plans = self.plans.write().await;
        let before = plans.len();
        plans.retain(|(ws, plan)| !(ws == workspace_id && plan.id == plan_id));
        if plans.len() == before {
            return Err(RedteamPlanStoreError::NotFound);
        }
        Ok(())
    }
}
