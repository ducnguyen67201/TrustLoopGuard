#![cfg(feature = "postgres")]

use std::sync::Arc;

use anyhow::Result;
use async_trait::async_trait;
use base64::{engine::general_purpose::STANDARD, Engine as _};
use tl_core::AgentProfile;
use tl_engine::ProfileResolver;
use tl_policy::Policy;
use tl_storage::{
    AgentRepo, AnalyticsRepo, DashboardAdminRepo, EnvironmentRepo, GatewayRepo, KnowledgeRepo,
    NewKnowledgeFile, NewKnowledgeSource, PolicyRepo, RunFilter, RunRepo, TraceRepo, UserRepo,
};

use crate::agents::{AgentStore, AgentStoreError};
use crate::analytics::AnalyticsStore;
use crate::auth::{WorkspaceApiKeyVerifier, WorkspaceApiKeyVerifyError, WorkspaceKeyContext};
use crate::auth_user::{UserStore, UserStoreError};
use crate::dashboard_admin::{ApiKeyStore, DashboardAdminStoreError, NewApiKey, SettingsStore};
use crate::environments::EnvironmentStore;
use crate::gateway::GatewayStore;
use crate::knowledge_sources::KnowledgeStore;
use crate::policies::{PolicyStore, PolicyStoreError};
use crate::runs::{RunListFilter, RunStore, RunStoreError};
use crate::traces::TraceStore;

/// Adapter newtype: wraps `tl_storage::AgentRepo` so we can implement
/// `tl_engine::ProfileResolver` and our own `AgentStore` for it
/// without violating Rust's orphan rule (both the trait and the
/// inner type live in foreign crates).
#[cfg(feature = "postgres")]
pub struct PostgresAgentAdapter(pub Arc<AgentRepo>);

#[cfg(feature = "postgres")]
impl PostgresAgentAdapter {
    pub fn new(repo: Arc<AgentRepo>) -> Arc<Self> {
        Arc::new(Self(repo))
    }
}

#[cfg(feature = "postgres")]
#[async_trait]
impl ProfileResolver for PostgresAgentAdapter {
    async fn resolve(&self, workspace_id: &str, agent_id: &str) -> Option<Arc<AgentProfile>> {
        self.0.get(workspace_id, agent_id).await.ok()
    }
}

#[cfg(feature = "postgres")]
#[async_trait]
impl AgentStore for PostgresAgentAdapter {
    async fn upsert(
        &self,
        workspace_id: &str,
        profile: &AgentProfile,
        source_yaml: &str,
    ) -> Result<(), AgentStoreError> {
        self.0
            .upsert(workspace_id, profile, source_yaml)
            .await
            .map_err(|e| AgentStoreError::Internal(e.to_string()))
    }

    async fn get(
        &self,
        workspace_id: &str,
        agent_id: &str,
    ) -> Result<Arc<AgentProfile>, AgentStoreError> {
        self.0
            .get(workspace_id, agent_id)
            .await
            .map_err(|e| match e {
                tl_storage::StorageError::NotFound => AgentStoreError::NotFound,
                other => AgentStoreError::Internal(other.to_string()),
            })
    }

    async fn delete(&self, workspace_id: &str, agent_id: &str) -> Result<(), AgentStoreError> {
        self.0
            .delete(workspace_id, agent_id)
            .await
            .map_err(|e| match e {
                tl_storage::StorageError::NotFound => AgentStoreError::NotFound,
                other => AgentStoreError::Internal(other.to_string()),
            })
    }

    async fn list(&self, workspace_id: &str) -> Result<Vec<Arc<AgentProfile>>, AgentStoreError> {
        self.0
            .list(workspace_id)
            .await
            .map_err(|e| AgentStoreError::Internal(e.to_string()))
    }
}

#[cfg(feature = "postgres")]
pub struct PostgresEnvironmentAdapter(pub Arc<EnvironmentRepo>);

#[cfg(feature = "postgres")]
impl PostgresEnvironmentAdapter {
    pub fn new(repo: Arc<EnvironmentRepo>) -> Arc<Self> {
        Arc::new(Self(repo))
    }
}

#[cfg(feature = "postgres")]
#[async_trait]
impl EnvironmentStore for PostgresEnvironmentAdapter {
    async fn list(
        &self,
        workspace_id: &str,
    ) -> Result<Vec<tl_core::WorkspaceEnvironment>, crate::environments::EnvironmentStoreError>
    {
        self.0
            .list(workspace_id)
            .await
            .map_err(environment_store_error)
    }

    async fn get(
        &self,
        workspace_id: &str,
        environment_id: &str,
    ) -> Result<tl_core::WorkspaceEnvironment, crate::environments::EnvironmentStoreError> {
        self.0
            .get(workspace_id, environment_id)
            .await
            .map_err(environment_store_error)
    }

    async fn default_environment_id(
        &self,
        workspace_id: &str,
    ) -> Result<String, crate::environments::EnvironmentStoreError> {
        self.0
            .default_environment_id(workspace_id)
            .await
            .map_err(environment_store_error)
    }

    async fn create(
        &self,
        workspace_id: &str,
        input: tl_core::CreateWorkspaceEnvironmentRequest,
    ) -> Result<tl_core::WorkspaceEnvironment, crate::environments::EnvironmentStoreError> {
        self.0
            .create(workspace_id, input)
            .await
            .map_err(environment_store_error)
    }

    async fn update(
        &self,
        workspace_id: &str,
        environment_id: &str,
        input: tl_core::UpdateWorkspaceEnvironmentRequest,
    ) -> Result<tl_core::WorkspaceEnvironment, crate::environments::EnvironmentStoreError> {
        self.0
            .update(workspace_id, environment_id, input)
            .await
            .map_err(environment_store_error)
    }

    async fn delete(
        &self,
        workspace_id: &str,
        environment_id: &str,
    ) -> Result<(), crate::environments::EnvironmentStoreError> {
        self.0
            .delete(workspace_id, environment_id)
            .await
            .map_err(environment_store_error)
    }
}

#[cfg(feature = "postgres")]
fn environment_store_error(
    error: tl_storage::StorageError,
) -> crate::environments::EnvironmentStoreError {
    match error {
        tl_storage::StorageError::NotFound => crate::environments::EnvironmentStoreError::NotFound,
        tl_storage::StorageError::Conflict => {
            crate::environments::EnvironmentStoreError::Validation(
                "environment conflicts with an existing row".into(),
            )
        }
        tl_storage::StorageError::Internal(message)
            if message.contains("environment is still referenced")
                || message.contains("default environment cannot be deleted")
                || message.contains("workspace must have one default environment") =>
        {
            crate::environments::EnvironmentStoreError::Validation(message)
        }
        other => crate::environments::EnvironmentStoreError::Internal(other.to_string()),
    }
}

#[cfg(feature = "postgres")]
pub struct PostgresPolicyAdapter(pub Arc<PolicyRepo>);

#[cfg(feature = "postgres")]
impl PostgresPolicyAdapter {
    pub fn new(repo: Arc<PolicyRepo>) -> Arc<Self> {
        Arc::new(Self(repo))
    }
}

#[cfg(feature = "postgres")]
#[async_trait]
impl PolicyStore for PostgresPolicyAdapter {
    async fn upsert(
        &self,
        workspace_id: &str,
        environment_id: &str,
        policy: &Policy,
        source_yaml: &str,
    ) -> Result<tl_core::PolicyDocument, PolicyStoreError> {
        self.0
            .upsert_in(workspace_id, policy, source_yaml)
            .await
            .map_err(|e| PolicyStoreError::Internal(e.to_string()))?;
        self.0
            .set_enabled_in_environment(workspace_id, environment_id, &policy.id, true)
            .await
            .map_err(|e| PolicyStoreError::Internal(e.to_string()))?;
        self.get(workspace_id, environment_id, &policy.id).await
    }

    async fn get(
        &self,
        workspace_id: &str,
        environment_id: &str,
        policy_id: &str,
    ) -> Result<tl_core::PolicyDocument, PolicyStoreError> {
        self.0
            .list_records_in_environment(workspace_id, environment_id)
            .await
            .map_or_else(
                |e| {
                    Err(match e {
                        tl_storage::StorageError::NotFound => PolicyStoreError::NotFound,
                        other => PolicyStoreError::Internal(other.to_string()),
                    })
                },
                |rows| {
                    let row = rows
                        .into_iter()
                        .find(|row| row.policy.id == policy_id)
                        .ok_or(PolicyStoreError::NotFound)?;
                    Ok(tl_core::PolicyDocument {
                        id: row.policy.id,
                        description: row.policy.description,
                        severity: row.policy.severity,
                        enabled: row.enabled,
                        source_yaml: row.source_yaml,
                    })
                },
            )
    }

    async fn list(
        &self,
        workspace_id: &str,
        environment_id: &str,
    ) -> Result<Vec<tl_core::PolicySummary>, PolicyStoreError> {
        self.0
            .list_records_in_environment(workspace_id, environment_id)
            .await
            .map_err(|e| PolicyStoreError::Internal(e.to_string()))
            .map(|rows| {
                rows.into_iter()
                    .map(|row| tl_core::PolicySummary {
                        id: row.policy.id,
                        description: row.policy.description,
                        severity: row.policy.severity,
                        action: Some(policy_action(&row.policy.action)),
                        enabled: row.enabled,
                        owner_agent_id: row.owner_agent_id,
                    })
                    .collect()
            })
    }

    async fn list_enabled(
        &self,
        workspace_id: &str,
        environment_id: &str,
    ) -> Result<Vec<Arc<Policy>>, PolicyStoreError> {
        self.0
            .list_enabled_in_environment(workspace_id, environment_id)
            .await
            .map_err(|e| PolicyStoreError::Internal(e.to_string()))
    }

    async fn set_enabled(
        &self,
        workspace_id: &str,
        environment_id: &str,
        policy_id: &str,
        enabled: bool,
    ) -> Result<tl_core::PolicyDocument, PolicyStoreError> {
        self.0
            .set_enabled_in_environment(workspace_id, environment_id, policy_id, enabled)
            .await
            .map_err(|e| match e {
                tl_storage::StorageError::NotFound => PolicyStoreError::NotFound,
                other => PolicyStoreError::Internal(other.to_string()),
            })?;
        self.get(workspace_id, environment_id, policy_id).await
    }

    async fn batch_set_enabled(
        &self,
        workspace_id: &str,
        environment_id: &str,
        policy_ids: &[String],
        enabled: bool,
    ) -> Result<Vec<tl_core::PolicySummary>, PolicyStoreError> {
        self.0
            .batch_set_enabled_in_environment(workspace_id, environment_id, policy_ids, enabled)
            .await
            .map_err(|e| match e {
                tl_storage::StorageError::NotFound => PolicyStoreError::NotFound,
                other => PolicyStoreError::Internal(other.to_string()),
            })
            .map(|rows| {
                rows.into_iter()
                    .map(|row| tl_core::PolicySummary {
                        id: row.policy.id,
                        description: row.policy.description,
                        severity: row.policy.severity,
                        action: Some(policy_action(&row.policy.action)),
                        enabled: row.enabled,
                        owner_agent_id: row.owner_agent_id,
                    })
                    .collect()
            })
    }

    async fn delete(&self, workspace_id: &str, policy_id: &str) -> Result<(), PolicyStoreError> {
        self.0
            .delete_in(workspace_id, policy_id)
            .await
            .map_err(|e| match e {
                tl_storage::StorageError::NotFound => PolicyStoreError::NotFound,
                other => PolicyStoreError::Internal(other.to_string()),
            })
    }

    async fn list_for_agent(
        &self,
        workspace_id: &str,
        environment_id: &str,
        agent_id: &str,
    ) -> Result<Vec<tl_core::PolicySummary>, PolicyStoreError> {
        self.0
            .list_records_in_environment(workspace_id, environment_id)
            .await
            .map_err(|e| PolicyStoreError::Internal(e.to_string()))
            .map(|rows| {
                rows.into_iter()
                    .filter(|row| row.owner_agent_id.as_deref() == Some(agent_id))
                    .map(|row| tl_core::PolicySummary {
                        id: row.policy.id,
                        description: row.policy.description,
                        severity: row.policy.severity,
                        action: Some(policy_action(&row.policy.action)),
                        enabled: row.enabled,
                        owner_agent_id: row.owner_agent_id,
                    })
                    .collect()
            })
    }

    async fn delete_for_agent(
        &self,
        workspace_id: &str,
        agent_id: &str,
    ) -> Result<Vec<String>, PolicyStoreError> {
        self.0
            .soft_delete_for_agent(workspace_id, agent_id)
            .await
            .map_err(|e| PolicyStoreError::Internal(e.to_string()))
    }

    async fn list_versions(
        &self,
        workspace_id: &str,
        policy_id: &str,
    ) -> Result<tl_core::EntityVersionListResponse, PolicyStoreError> {
        self.0
            .list_versions_in(workspace_id, policy_id)
            .await
            .map_err(|e| PolicyStoreError::Internal(e.to_string()))
            .map(|rows| tl_core::EntityVersionListResponse {
                versions: rows
                    .into_iter()
                    .map(|r| tl_core::EntityVersionSummary {
                        version: r.version,
                        created_at: r.created_at.to_rfc3339(),
                    })
                    .collect(),
            })
    }

    async fn get_version(
        &self,
        workspace_id: &str,
        policy_id: &str,
        version: i32,
    ) -> Result<tl_core::EntityVersionDetail, PolicyStoreError> {
        self.0
            .get_version_in(workspace_id, policy_id, version)
            .await
            .map_err(|e| match e {
                tl_storage::StorageError::NotFound => PolicyStoreError::NotFound,
                other => PolicyStoreError::Internal(other.to_string()),
            })
            .map(|r| tl_core::EntityVersionDetail {
                version: r.version,
                content: r.content,
                created_at: r.created_at.to_rfc3339(),
            })
    }
}

#[cfg(feature = "postgres")]
pub struct PostgresTraceAdapter(pub Arc<TraceRepo>);

#[cfg(feature = "postgres")]
impl PostgresTraceAdapter {
    pub fn new(repo: Arc<TraceRepo>) -> Arc<Self> {
        Arc::new(Self(repo))
    }
}

#[cfg(feature = "postgres")]
#[async_trait]
impl TraceStore for PostgresTraceAdapter {
    async fn list_recent(
        &self,
        workspace_id: &str,
        environment_id: &str,
        limit: usize,
    ) -> Result<Vec<tl_core::TraceSummary>, crate::traces::TraceStoreError> {
        self.0
            .list_recent(workspace_id, environment_id, limit as i64)
            .await
            .map_err(|e| crate::traces::TraceStoreError::Internal(e.to_string()))
            .map(|rows| {
                rows.into_iter()
                    .map(|row| tl_core::TraceSummary {
                        trace_id: row.trace_id.to_string(),
                        run_id: row.run_id.map(|id| id.to_string()),
                        run_event_id: row.run_event_id.map(|id| id.to_string()),
                        environment_id: row.environment_id.clone(),
                        environment: row.environment_id,
                        domain: row.domain,
                        decision: row.decision,
                        elapsed_ms: row.elapsed_ms,
                        latest_review_outcome: row.latest_review_outcome,
                        latest_reviewed_at: row.latest_reviewed_at.map(|value| value.to_rfc3339()),
                        payload: row.payload,
                        created_at: row.created_at.to_rfc3339(),
                    })
                    .collect()
            })
    }
}

#[cfg(feature = "postgres")]
pub struct PostgresRunAdapter(pub Arc<RunRepo>);

#[cfg(feature = "postgres")]
impl PostgresRunAdapter {
    pub fn new(repo: Arc<RunRepo>) -> Arc<Self> {
        Arc::new(Self(repo))
    }
}

#[cfg(feature = "postgres")]
#[async_trait]
impl RunStore for PostgresRunAdapter {
    async fn create(
        &self,
        workspace_id: &str,
        environment_id: &str,
        input: tl_core::CreateRunRequest,
    ) -> Result<tl_core::RunSummary, RunStoreError> {
        self.0
            .create(workspace_id, environment_id, input)
            .await
            .map_err(run_store_error)
    }

    async fn list(
        &self,
        workspace_id: &str,
        environment_id: &str,
        filter: RunListFilter,
    ) -> Result<Vec<tl_core::RunSummary>, RunStoreError> {
        self.0
            .list(
                workspace_id,
                RunFilter {
                    environment_id: Some(environment_id.to_string()),
                    agent_id: filter.agent_id,
                    status: filter.status,
                    kind: filter.kind,
                    external_id: filter.external_id,
                    limit: filter.limit as i64,
                },
            )
            .await
            .map_err(run_store_error)
    }

    async fn get(
        &self,
        workspace_id: &str,
        environment_id: &str,
        run_id: &str,
    ) -> Result<tl_core::RunSummary, RunStoreError> {
        self.0
            .get(workspace_id, run_id)
            .await
            .and_then(|run| {
                if run.environment_id == environment_id {
                    Ok(run)
                } else {
                    Err(tl_storage::StorageError::NotFound)
                }
            })
            .map_err(run_store_error)
    }

    async fn update(
        &self,
        workspace_id: &str,
        environment_id: &str,
        run_id: &str,
        input: tl_core::UpdateRunRequest,
    ) -> Result<tl_core::RunSummary, RunStoreError> {
        self.get(workspace_id, environment_id, run_id).await?;
        self.0
            .update(workspace_id, run_id, input)
            .await
            .map_err(run_store_error)
    }

    async fn create_event(
        &self,
        workspace_id: &str,
        environment_id: &str,
        run_id: &str,
        input: tl_core::CreateRunEventRequest,
    ) -> Result<tl_core::RunEventSummary, RunStoreError> {
        self.get(workspace_id, environment_id, run_id).await?;
        self.0
            .create_event(workspace_id, run_id, input)
            .await
            .map_err(run_store_error)
    }

    async fn events(
        &self,
        workspace_id: &str,
        environment_id: &str,
        run_id: &str,
        limit: usize,
    ) -> Result<Vec<tl_core::RunEventSummary>, RunStoreError> {
        self.get(workspace_id, environment_id, run_id).await?;
        self.0
            .events(workspace_id, run_id, limit as i64)
            .await
            .map_err(run_store_error)
    }

    async fn traces(
        &self,
        workspace_id: &str,
        environment_id: &str,
        run_id: &str,
        limit: usize,
    ) -> Result<Vec<tl_core::TraceSummary>, RunStoreError> {
        self.get(workspace_id, environment_id, run_id).await?;
        self.0
            .traces(workspace_id, run_id, limit as i64)
            .await
            .map_err(run_store_error)
    }
}

#[cfg(feature = "postgres")]
pub struct PostgresHumanReviewAdapter(pub Arc<tl_storage::HumanReviewRepo>);

#[cfg(feature = "postgres")]
impl PostgresHumanReviewAdapter {
    pub fn new(repo: Arc<tl_storage::HumanReviewRepo>) -> Arc<Self> {
        Arc::new(Self(repo))
    }
}

#[cfg(feature = "postgres")]
#[async_trait]
impl crate::human_review::HumanReviewStore for PostgresHumanReviewAdapter {
    async fn create_event(
        &self,
        workspace_id: &str,
        trace_id: &str,
        input: tl_core::CreateHumanReviewEventRequest,
        reviewer_id: Option<String>,
    ) -> Result<tl_core::HumanReviewEvent, crate::human_review::HumanReviewStoreError> {
        self.0
            .create_event(workspace_id, trace_id, input, reviewer_id)
            .await
            .map_err(human_review_store_error)
    }

    async fn list_events(
        &self,
        workspace_id: &str,
        trace_id: &str,
        limit: usize,
    ) -> Result<Vec<tl_core::HumanReviewEvent>, crate::human_review::HumanReviewStoreError> {
        self.0
            .list_events(workspace_id, trace_id, limit as i64)
            .await
            .map_err(human_review_store_error)
    }

    async fn analytics(
        &self,
        workspace_id: &str,
        filter: crate::human_review::HumanReviewAnalyticsFilter,
    ) -> Result<tl_core::HumanReviewAnalyticsResponse, crate::human_review::HumanReviewStoreError>
    {
        self.0
            .analytics(
                workspace_id,
                tl_storage::HumanReviewAnalyticsFilter {
                    agent_id: filter.agent_id,
                    policy_id: filter.policy_id,
                    run_kind: filter.run_kind,
                    workflow_step: filter.workflow_step,
                },
            )
            .await
            .map_err(human_review_store_error)
    }
}

#[cfg(feature = "postgres")]
fn human_review_store_error(
    error: tl_storage::StorageError,
) -> crate::human_review::HumanReviewStoreError {
    match error {
        tl_storage::StorageError::NotFound => crate::human_review::HumanReviewStoreError::NotFound,
        tl_storage::StorageError::Conflict => {
            crate::human_review::HumanReviewStoreError::Internal("conflict".into())
        }
        tl_storage::StorageError::Internal(message) if message.contains("parse") => {
            crate::human_review::HumanReviewStoreError::Validation(message)
        }
        tl_storage::StorageError::Internal(message) => {
            crate::human_review::HumanReviewStoreError::Internal(message)
        }
    }
}

#[cfg(feature = "postgres")]
fn run_store_error(error: tl_storage::StorageError) -> RunStoreError {
    match error {
        tl_storage::StorageError::NotFound => RunStoreError::NotFound,
        tl_storage::StorageError::Conflict => RunStoreError::Internal("conflict".into()),
        tl_storage::StorageError::Internal(message) if message.contains("parse") => {
            RunStoreError::Validation(message)
        }
        tl_storage::StorageError::Internal(message) => RunStoreError::Internal(message),
    }
}

#[cfg(feature = "postgres")]
pub struct PostgresAnalyticsAdapter(pub Arc<AnalyticsRepo>);

#[cfg(feature = "postgres")]
impl PostgresAnalyticsAdapter {
    pub fn new(repo: Arc<AnalyticsRepo>) -> Arc<Self> {
        Arc::new(Self(repo))
    }
}

#[cfg(feature = "postgres")]
#[async_trait]
impl AnalyticsStore for PostgresAnalyticsAdapter {
    async fn catalog(
        &self,
        workspace_id: &str,
    ) -> Result<tl_core::AnalyticsFacetCatalogResponse, crate::analytics::AnalyticsStoreError> {
        self.0
            .catalog(workspace_id)
            .await
            .map_err(analytics_store_error)
    }

    async fn query(
        &self,
        workspace_id: &str,
        request: tl_core::AnalyticsQueryRequest,
    ) -> Result<tl_core::AnalyticsQueryResponse, crate::analytics::AnalyticsStoreError> {
        self.0
            .query(workspace_id, request)
            .await
            .map_err(analytics_store_error)
    }

    async fn list_views(
        &self,
        workspace_id: &str,
    ) -> Result<Vec<tl_core::AnalyticsDashboardView>, crate::analytics::AnalyticsStoreError> {
        self.0
            .list_views(workspace_id)
            .await
            .map_err(analytics_store_error)
    }

    async fn create_view(
        &self,
        workspace_id: &str,
        request: tl_core::CreateAnalyticsDashboardViewRequest,
    ) -> Result<tl_core::AnalyticsDashboardView, crate::analytics::AnalyticsStoreError> {
        self.0
            .create_view(workspace_id, request)
            .await
            .map_err(analytics_store_error)
    }

    async fn update_view(
        &self,
        workspace_id: &str,
        view_id: &str,
        request: tl_core::UpdateAnalyticsDashboardViewRequest,
    ) -> Result<tl_core::AnalyticsDashboardView, crate::analytics::AnalyticsStoreError> {
        self.0
            .update_view(workspace_id, view_id, request)
            .await
            .map_err(analytics_store_error)
    }

    async fn delete_view(
        &self,
        workspace_id: &str,
        view_id: &str,
    ) -> Result<(), crate::analytics::AnalyticsStoreError> {
        self.0
            .delete_view(workspace_id, view_id)
            .await
            .map_err(analytics_store_error)
    }
}

#[cfg(feature = "postgres")]
fn analytics_store_error(error: tl_storage::StorageError) -> crate::analytics::AnalyticsStoreError {
    match error {
        tl_storage::StorageError::NotFound => crate::analytics::AnalyticsStoreError::NotFound,
        tl_storage::StorageError::Conflict => crate::analytics::AnalyticsStoreError::Validation(
            "analytics view already exists".into(),
        ),
        tl_storage::StorageError::Internal(message)
            if message.contains("required")
                || message.contains("must")
                || message.contains("filters") =>
        {
            crate::analytics::AnalyticsStoreError::Validation(message)
        }
        tl_storage::StorageError::Internal(message) => {
            crate::analytics::AnalyticsStoreError::Internal(message)
        }
    }
}

#[cfg(feature = "postgres")]
pub struct PostgresDashboardAdminAdapter(pub Arc<DashboardAdminRepo>);

#[cfg(feature = "postgres")]
impl PostgresDashboardAdminAdapter {
    pub fn new(repo: Arc<DashboardAdminRepo>) -> Arc<Self> {
        Arc::new(Self(repo))
    }
}

#[cfg(feature = "postgres")]
#[async_trait]
impl ApiKeyStore for PostgresDashboardAdminAdapter {
    async fn list(
        &self,
        workspace_id: &str,
    ) -> Result<Vec<tl_core::DashboardApiKey>, crate::dashboard_admin::DashboardAdminStoreError>
    {
        self.0
            .list_api_keys(workspace_id)
            .await
            .map_err(|e| crate::dashboard_admin::DashboardAdminStoreError::Internal(e.to_string()))
    }

    async fn create(
        &self,
        input: NewApiKey,
    ) -> Result<tl_core::DashboardApiKey, DashboardAdminStoreError> {
        self.0
            .create_api_key(
                &input.id,
                &input.workspace_id,
                &input.environment_id,
                &input.name,
                &input.key_prefix,
                &input.key_hash,
                input.created_by_user_id,
            )
            .await
            .map_err(|e| DashboardAdminStoreError::Internal(e.to_string()))
    }

    async fn batch_revoke(
        &self,
        workspace_id: &str,
        ids: &[String],
    ) -> Result<Vec<tl_core::DashboardApiKey>, DashboardAdminStoreError> {
        self.0
            .batch_revoke_api_keys(workspace_id, ids)
            .await
            .map_err(|e| match e {
                tl_storage::StorageError::NotFound => DashboardAdminStoreError::NotFound,
                other => DashboardAdminStoreError::Internal(other.to_string()),
            })
    }
}

#[cfg(feature = "postgres")]
#[async_trait]
impl WorkspaceApiKeyVerifier for PostgresDashboardAdminAdapter {
    async fn verify_workspace_api_key(
        &self,
        key_hash: &str,
    ) -> Result<Option<WorkspaceKeyContext>, WorkspaceApiKeyVerifyError> {
        self.0
            .verify_api_key_hash(key_hash)
            .await
            .map(|row| {
                row.map(|row| WorkspaceKeyContext {
                    api_key_id: row.id,
                    workspace_id: row.workspace_id,
                    environment_id: row.environment_id,
                })
            })
            .map_err(|e| WorkspaceApiKeyVerifyError::Internal(e.to_string()))
    }
}

#[cfg(feature = "postgres")]
#[async_trait]
impl SettingsStore for PostgresDashboardAdminAdapter {
    async fn get(
        &self,
        workspace_id: &str,
    ) -> Result<tl_core::WorkspaceSettings, crate::dashboard_admin::DashboardAdminStoreError> {
        self.0
            .get_settings(workspace_id)
            .await
            .map_err(|e| crate::dashboard_admin::DashboardAdminStoreError::Internal(e.to_string()))
            .map(|settings| settings.unwrap_or_else(crate::dashboard_admin::default_settings))
    }
}

#[cfg(feature = "postgres")]
pub struct PostgresGatewayAdapter(pub Arc<GatewayRepo>);

#[cfg(feature = "postgres")]
impl PostgresGatewayAdapter {
    pub fn new(repo: Arc<GatewayRepo>) -> Arc<Self> {
        Arc::new(Self(repo))
    }
}

#[cfg(feature = "postgres")]
#[async_trait]
impl GatewayStore for PostgresGatewayAdapter {
    async fn list_provider_connections(
        &self,
        workspace_id: &str,
    ) -> Result<Vec<tl_core::GatewayProviderConnection>, crate::gateway::GatewayStoreError> {
        self.0
            .list_provider_connections(workspace_id)
            .await
            .map_err(gateway_store_error)
    }

    async fn create_provider_connection(
        &self,
        input: crate::gateway::NewGatewayProviderConnection,
    ) -> Result<tl_core::GatewayProviderConnection, crate::gateway::GatewayStoreError> {
        self.0
            .create_provider_connection(tl_storage::models::NewGatewayProviderConnection {
                workspace_id: input.workspace_id,
                id: input.id,
                display_name: input.display_name,
                kind: crate::gateway::provider_kind_storage_text(input.kind).to_string(),
                base_url: input.base_url,
                default_model: input.default_model,
                encrypted_api_key: input.encrypted_api_key,
            })
            .await
            .map_err(gateway_store_error)
    }

    async fn update_provider_connection(
        &self,
        workspace_id: &str,
        id: &str,
        patch: crate::gateway::ProviderConnectionPatch,
    ) -> Result<tl_core::GatewayProviderConnection, crate::gateway::GatewayStoreError> {
        self.0
            .update_provider_connection(
                workspace_id,
                id,
                patch.display_name.as_deref(),
                patch.base_url.as_ref().map(|value| value.as_deref()),
                patch.default_model.as_deref(),
                patch.encrypted_api_key.as_deref(),
            )
            .await
            .map_err(gateway_store_error)
    }

    async fn get_provider_connection_secret(
        &self,
        workspace_id: &str,
        id: &str,
    ) -> Result<crate::gateway::ProviderConnectionSecret, crate::gateway::GatewayStoreError> {
        self.0
            .get_provider_connection_secret(workspace_id, id)
            .await
            .map(|secret| crate::gateway::ProviderConnectionSecret {
                connection: secret.connection,
                encrypted_api_key: secret.encrypted_api_key,
            })
            .map_err(gateway_store_error)
    }

    async fn list_enforcement_profiles(
        &self,
        workspace_id: &str,
    ) -> Result<Vec<tl_core::EnforcementProfile>, crate::gateway::GatewayStoreError> {
        self.0
            .list_enforcement_profiles(workspace_id)
            .await
            .map_err(gateway_store_error)
    }

    async fn create_enforcement_profile(
        &self,
        input: crate::gateway::NewEnforcementProfile,
    ) -> Result<tl_core::EnforcementProfile, crate::gateway::GatewayStoreError> {
        self.0
            .create_enforcement_profile(tl_storage::models::NewEnforcementProfile {
                workspace_id: input.workspace_id,
                id: input.id,
                display_name: input.display_name,
                input_action: crate::gateway::input_action_storage_text(input.input_action)
                    .to_string(),
                output_action: crate::gateway::output_action_storage_text(input.output_action)
                    .to_string(),
                fail_mode: crate::gateway::fail_mode_storage_text(input.fail_mode).to_string(),
                retention_mode: crate::gateway::retention_mode_storage_text(input.retention_mode)
                    .to_string(),
                response_mode: crate::gateway::response_mode_storage_text(input.response_mode)
                    .to_string(),
                fallback_message: input.fallback_message,
                max_regenerations: input.max_regenerations as i32,
            })
            .await
            .map_err(gateway_store_error)
    }

    async fn update_enforcement_profile(
        &self,
        workspace_id: &str,
        id: &str,
        patch: crate::gateway::EnforcementProfilePatch,
    ) -> Result<tl_core::EnforcementProfile, crate::gateway::GatewayStoreError> {
        self.0
            .update_enforcement_profile(
                workspace_id,
                id,
                tl_storage::EnforcementProfilePatch {
                    display_name: patch.display_name,
                    input_action: patch
                        .input_action
                        .map(crate::gateway::input_action_storage_text)
                        .map(str::to_string),
                    output_action: patch
                        .output_action
                        .map(crate::gateway::output_action_storage_text)
                        .map(str::to_string),
                    fail_mode: patch
                        .fail_mode
                        .map(crate::gateway::fail_mode_storage_text)
                        .map(str::to_string),
                    retention_mode: patch
                        .retention_mode
                        .map(crate::gateway::retention_mode_storage_text)
                        .map(str::to_string),
                    response_mode: patch
                        .response_mode
                        .map(crate::gateway::response_mode_storage_text)
                        .map(str::to_string),
                    fallback_message: patch.fallback_message,
                    max_regenerations: patch.max_regenerations.map(|value| value as i32),
                },
            )
            .await
            .map_err(gateway_store_error)
    }

    async fn list_gateway_routes(
        &self,
        workspace_id: &str,
    ) -> Result<Vec<tl_core::GatewayRoute>, crate::gateway::GatewayStoreError> {
        self.0
            .list_gateway_routes(workspace_id)
            .await
            .map_err(gateway_store_error)
    }

    async fn create_gateway_route(
        &self,
        input: crate::gateway::NewGatewayRoute,
    ) -> Result<tl_core::GatewayRoute, crate::gateway::GatewayStoreError> {
        self.0
            .create_gateway_route(tl_storage::models::NewGatewayRoute {
                workspace_id: input.workspace_id,
                id: input.id,
                display_name: input.display_name,
                provider_connection_id: input.provider_connection_id,
                agent_id: input.agent_id,
                enforcement_profile_id: input.enforcement_profile_id,
            })
            .await
            .map_err(gateway_store_error)
    }

    async fn update_gateway_route(
        &self,
        workspace_id: &str,
        id: &str,
        patch: crate::gateway::GatewayRoutePatch,
    ) -> Result<tl_core::GatewayRoute, crate::gateway::GatewayStoreError> {
        self.0
            .update_gateway_route(
                workspace_id,
                id,
                tl_storage::GatewayRoutePatch {
                    display_name: patch.display_name,
                    provider_connection_id: patch.provider_connection_id,
                    agent_id: patch.agent_id,
                    enforcement_profile_id: patch.enforcement_profile_id,
                },
            )
            .await
            .map_err(gateway_store_error)
    }

    async fn resolve_gateway_route(
        &self,
        workspace_id: &str,
        route_id: &str,
    ) -> Result<crate::gateway::ResolvedGatewayRoute, crate::gateway::GatewayStoreError> {
        self.0
            .resolve_gateway_route(workspace_id, route_id)
            .await
            .map(|resolved| crate::gateway::ResolvedGatewayRoute {
                route: resolved.route,
                provider_connection: resolved.provider_connection,
                enforcement_profile: resolved.enforcement_profile,
                encrypted_api_key: resolved.encrypted_api_key,
            })
            .map_err(gateway_store_error)
    }
}

#[cfg(feature = "postgres")]
fn gateway_store_error(error: tl_storage::StorageError) -> crate::gateway::GatewayStoreError {
    match error {
        tl_storage::StorageError::NotFound => crate::gateway::GatewayStoreError::NotFound,
        other => crate::gateway::GatewayStoreError::Internal(other.to_string()),
    }
}

#[cfg(feature = "postgres")]
pub struct PostgresKnowledgeAdapter(pub Arc<KnowledgeRepo>);

#[cfg(feature = "postgres")]
impl PostgresKnowledgeAdapter {
    pub fn new(repo: Arc<KnowledgeRepo>) -> Arc<Self> {
        Arc::new(Self(repo))
    }
}

#[cfg(feature = "postgres")]
#[async_trait]
impl KnowledgeStore for PostgresKnowledgeAdapter {
    async fn list(
        &self,
        workspace_id: &str,
    ) -> Result<Vec<tl_core::KnowledgeSourceDocument>, crate::knowledge_sources::KnowledgeStoreError>
    {
        self.0
            .list(workspace_id)
            .await
            .map_err(|e| crate::knowledge_sources::KnowledgeStoreError::Internal(e.to_string()))?
            .into_iter()
            .map(knowledge_row_to_document)
            .collect()
    }

    async fn create(
        &self,
        workspace_id: &str,
        input: tl_core::CreateKnowledgeSourceRequest,
    ) -> Result<tl_core::KnowledgeSourceDocument, crate::knowledge_sources::KnowledgeStoreError>
    {
        let file = match input.file {
            Some(file) => {
                let data = crate::knowledge_sources::decode_file_data(&file.data_base64)?;
                Some(NewKnowledgeFile {
                    file_name: file.file_name,
                    media_type: file.media_type,
                    data,
                })
            }
            None => None,
        };
        let row = self
            .0
            .create(
                workspace_id,
                NewKnowledgeSource {
                    title: input.title,
                    kind: knowledge_kind_text(input.kind).to_string(),
                    location: input.location,
                    notes: input.notes,
                    file,
                },
            )
            .await
            .map_err(|e| crate::knowledge_sources::KnowledgeStoreError::Internal(e.to_string()))?;
        knowledge_row_to_document(row)
    }

    async fn get_file(
        &self,
        workspace_id: &str,
        source_id: &str,
    ) -> Result<tl_core::KnowledgeSourceFileResponse, crate::knowledge_sources::KnowledgeStoreError>
    {
        let row = self
            .0
            .get_file(workspace_id, source_id)
            .await
            .map_err(|e| match e {
                tl_storage::StorageError::NotFound => {
                    crate::knowledge_sources::KnowledgeStoreError::NotFound
                }
                other => crate::knowledge_sources::KnowledgeStoreError::Internal(other.to_string()),
            })?;
        Ok(tl_core::KnowledgeSourceFileResponse {
            file_name: row.file_name,
            media_type: row.media_type,
            byte_size: row.byte_size,
            data_base64: STANDARD.encode(row.data),
        })
    }
}

#[cfg(feature = "postgres")]
fn knowledge_row_to_document(
    row: tl_storage::KnowledgeSourceRow,
) -> Result<tl_core::KnowledgeSourceDocument, crate::knowledge_sources::KnowledgeStoreError> {
    Ok(tl_core::KnowledgeSourceDocument {
        id: row.id,
        title: row.title,
        kind: parse_knowledge_kind(&row.kind)?,
        location: row.location,
        status: parse_knowledge_status(&row.status)?,
        metadata: row.metadata,
        created_at: row.created_at.to_rfc3339(),
        updated_at: row.updated_at.to_rfc3339(),
        last_indexed_at: row.last_indexed_at.map(|ts| ts.to_rfc3339()),
    })
}

#[cfg(feature = "postgres")]
fn knowledge_kind_text(kind: tl_core::DashboardKnowledgeSourceKind) -> &'static str {
    match kind {
        tl_core::DashboardKnowledgeSourceKind::Url => "url",
        tl_core::DashboardKnowledgeSourceKind::File => "file",
        tl_core::DashboardKnowledgeSourceKind::Note => "note",
    }
}

#[cfg(feature = "postgres")]
fn parse_knowledge_kind(
    kind: &str,
) -> Result<tl_core::DashboardKnowledgeSourceKind, crate::knowledge_sources::KnowledgeStoreError> {
    match kind {
        "url" => Ok(tl_core::DashboardKnowledgeSourceKind::Url),
        "file" => Ok(tl_core::DashboardKnowledgeSourceKind::File),
        "note" => Ok(tl_core::DashboardKnowledgeSourceKind::Note),
        other => Err(crate::knowledge_sources::KnowledgeStoreError::Internal(
            format!("unknown knowledge source kind `{other}`"),
        )),
    }
}

#[cfg(feature = "postgres")]
fn parse_knowledge_status(
    status: &str,
) -> Result<tl_core::KnowledgeSourceStatus, crate::knowledge_sources::KnowledgeStoreError> {
    match status {
        "draft" => Ok(tl_core::KnowledgeSourceStatus::Draft),
        "indexing" => Ok(tl_core::KnowledgeSourceStatus::Indexing),
        "ready" => Ok(tl_core::KnowledgeSourceStatus::Ready),
        "failed" => Ok(tl_core::KnowledgeSourceStatus::Failed),
        other => Err(crate::knowledge_sources::KnowledgeStoreError::Internal(
            format!("unknown knowledge source status `{other}`"),
        )),
    }
}

#[cfg(feature = "postgres")]
fn policy_action(action: &tl_policy::Action) -> String {
    match action {
        tl_policy::Action::Allow => "allow",
        tl_policy::Action::Block => "block",
        tl_policy::Action::Rewrite => "rewrite",
        tl_policy::Action::Escalate => "escalate",
    }
    .to_string()
}

#[cfg(feature = "postgres")]
pub struct PostgresUserAdapter(pub Arc<UserRepo>);

#[cfg(feature = "postgres")]
impl PostgresUserAdapter {
    pub fn new(repo: Arc<UserRepo>) -> Arc<Self> {
        Arc::new(Self(repo))
    }
}

#[cfg(feature = "postgres")]
#[async_trait]
impl UserStore for PostgresUserAdapter {
    async fn create(
        &self,
        username: &str,
        password_hash: &str,
    ) -> Result<crate::auth_user::UserRecord, UserStoreError> {
        let row = self
            .0
            .create(username, password_hash)
            .await
            .map_err(|e| match e {
                tl_storage::StorageError::Conflict => UserStoreError::Conflict,
                other => UserStoreError::Internal(other.to_string()),
            })?;
        Ok(crate::auth_user::UserRecord {
            id: row.id,
            username: row.username,
            password_hash: row.password_hash,
            is_approved: row.is_approved,
        })
    }

    async fn find_by_username(
        &self,
        username: &str,
    ) -> Result<crate::auth_user::UserRecord, UserStoreError> {
        let row = self
            .0
            .find_by_username(username)
            .await
            .map_err(|e| match e {
                tl_storage::StorageError::NotFound => UserStoreError::NotFound,
                other => UserStoreError::Internal(other.to_string()),
            })?;
        Ok(crate::auth_user::UserRecord {
            id: row.id,
            username: row.username,
            password_hash: row.password_hash,
            is_approved: row.is_approved,
        })
    }

    async fn is_approved(&self, id: uuid::Uuid) -> Result<bool, UserStoreError> {
        self.0.is_approved(id).await.map_err(|e| match e {
            tl_storage::StorageError::NotFound => UserStoreError::NotFound,
            other => UserStoreError::Internal(other.to_string()),
        })
    }

    async fn ensure_oauth_identity(
        &self,
        provider: &str,
        provider_subject: &str,
        email: &str,
    ) -> Result<crate::auth_user::UserRecord, UserStoreError> {
        let row = self
            .0
            .ensure_oauth_identity(provider, provider_subject, email)
            .await
            .map_err(|e| match e {
                tl_storage::StorageError::NotFound => UserStoreError::NotFound,
                tl_storage::StorageError::Conflict => UserStoreError::Conflict,
                other => UserStoreError::Internal(other.to_string()),
            })?;
        Ok(crate::auth_user::UserRecord {
            id: row.id,
            username: row.username,
            password_hash: row.password_hash,
            is_approved: row.is_approved,
        })
    }

    async fn update_password(
        &self,
        id: uuid::Uuid,
        password_hash: &str,
    ) -> Result<(), UserStoreError> {
        self.0
            .update_password(id, password_hash)
            .await
            .map_err(|e| match e {
                tl_storage::StorageError::NotFound => UserStoreError::NotFound,
                other => UserStoreError::Internal(other.to_string()),
            })
    }
}
