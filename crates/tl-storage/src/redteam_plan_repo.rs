//! Durable storage for saved, named attack plans (per agent).
//!
//! A plan's generated content (vectors + analyzer paths) is stored as a JSONB
//! blob; the id, name, agent, and timestamp live in their own columns so the
//! library can be listed newest-first per agent.

use diesel::prelude::*;
use diesel_async::RunQueryDsl;
use serde::{Deserialize, Serialize};
use tl_core::{AttackVector, RedteamPlanResponse, WorkflowPath};
use uuid::Uuid;

use crate::models::{NewRedteamPlan, RedteamPlanRecord};
use crate::postgres::{DbConnection, DbPool};
use crate::schema::redteam_plans;
use crate::StorageError;

/// The JSONB body of a saved plan — the generated attack content, minus the row
/// metadata (id/name/agent/created_at live in their own columns).
#[derive(Debug, Serialize, Deserialize)]
struct PlanBody {
    vectors: Vec<AttackVector>,
    paths: Vec<WorkflowPath>,
    unmapped_node_types: Vec<String>,
}

#[derive(Clone)]
pub struct RedteamPlanRepo {
    pool: DbPool,
}

impl RedteamPlanRepo {
    pub fn new(pool: DbPool) -> Self {
        Self { pool }
    }

    pub async fn create(
        &self,
        workspace_id: &str,
        environment_id: &str,
        agent_id: &str,
        name: &str,
        vectors: Vec<AttackVector>,
        paths: Vec<WorkflowPath>,
        unmapped_node_types: Vec<String>,
    ) -> Result<RedteamPlanResponse, StorageError> {
        let id = Uuid::now_v7();
        let body = PlanBody {
            vectors,
            paths,
            unmapped_node_types,
        };
        let plan = serde_json::to_value(&body)
            .map_err(|e| StorageError::Internal(format!("plan serialize: {e}")))?;
        let new_plan = NewRedteamPlan {
            workspace_id: workspace_id.to_string(),
            id,
            environment_id: environment_id.to_string(),
            agent_id: agent_id.to_string(),
            name: name.to_string(),
            plan,
        };
        let mut conn = self.connection().await?;
        let record = diesel::insert_into(redteam_plans::table)
            .values(&new_plan)
            .returning(RedteamPlanRecord::as_returning())
            .get_result::<RedteamPlanRecord>(&mut conn)
            .await
            .map_err(|e| StorageError::Internal(format!("redteam plan create: {e}")))?;
        plan_response(record)
    }

    pub async fn list(
        &self,
        workspace_id: &str,
        agent_id: &str,
        limit: i64,
    ) -> Result<Vec<RedteamPlanResponse>, StorageError> {
        let mut conn = self.connection().await?;
        let records = redteam_plans::table
            .filter(redteam_plans::workspace_id.eq(workspace_id))
            .filter(redteam_plans::agent_id.eq(agent_id))
            .order(redteam_plans::created_at.desc())
            .limit(limit)
            .select(RedteamPlanRecord::as_select())
            .load::<RedteamPlanRecord>(&mut conn)
            .await
            .map_err(|e| StorageError::Internal(format!("redteam plan list: {e}")))?;
        records.into_iter().map(plan_response).collect()
    }

    pub async fn delete(&self, workspace_id: &str, plan_id: &str) -> Result<(), StorageError> {
        let id = parse_uuid(plan_id)?;
        let mut conn = self.connection().await?;
        let rows = diesel::delete(
            redteam_plans::table
                .filter(redteam_plans::workspace_id.eq(workspace_id))
                .filter(redteam_plans::id.eq(id)),
        )
        .execute(&mut conn)
        .await
        .map_err(|e| StorageError::Internal(format!("redteam plan delete: {e}")))?;
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

impl std::fmt::Debug for RedteamPlanRepo {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("RedteamPlanRepo").finish_non_exhaustive()
    }
}

fn plan_response(record: RedteamPlanRecord) -> Result<RedteamPlanResponse, StorageError> {
    let body: PlanBody = serde_json::from_value(record.plan)
        .map_err(|e| StorageError::Internal(format!("plan deserialize: {e}")))?;
    Ok(RedteamPlanResponse {
        id: record.id.to_string(),
        agent_id: record.agent_id,
        name: record.name,
        vectors: body.vectors,
        paths: body.paths,
        unmapped_node_types: body.unmapped_node_types,
        generated_at: record.created_at.to_rfc3339(),
    })
}

fn parse_uuid(value: &str) -> Result<Uuid, StorageError> {
    Uuid::parse_str(value).map_err(|_| StorageError::NotFound)
}
