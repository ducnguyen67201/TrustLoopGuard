#![cfg(feature = "postgres")]

use std::sync::Arc;

use anyhow::Result;
use tl_engine::LabelPolicyProvider;
use tl_engine::ProfileResolver;
use tl_engine::ToolMetadataProvider;
use tl_policy::Policy;
use tl_storage::{
    connect_postgres, migrate_postgres, spawn_writer, AgentRepo, AnalyticsRepo, DashboardAdminRepo,
    EnvironmentRepo, EscalationRepo, GatewayRepo, KnowledgeRepo, PolicyRepo, RedteamJobRepo,
    RedteamReportShareRepo, RunRepo, SourceLabelPolicyRepo, TeamRepo, ToolMetadataRepo, TraceRepo,
    TraceWrite, UserRepo, WriterConfig,
};
use tokio::sync::mpsc;

use crate::agents::{AgentStore, MemoryAgentStore};
use crate::analytics::{AnalyticsStore, MemoryAnalyticsStore};
use crate::auth_user::{MemoryUserStore, UserStore};
use crate::dashboard_admin::{ApiKeyStore, MemoryApiKeyStore, MemorySettingsStore, SettingsStore};
use crate::environments::{EnvironmentStore, MemoryEnvironmentStore};
use crate::gateway::{GatewayStore, MemoryGatewayStore};
use crate::human_review::{HumanReviewStore, MemoryHumanReviewStore};
use crate::knowledge_sources::{KnowledgeStore, MemoryKnowledgeStore};
use crate::label_policy::{LabelPolicyStore, MemoryLabelPolicyStore};
use crate::policies::{MemoryPolicyStore, PolicyStore};
use crate::redteam::{
    MemoryRedteamJobStore, MemoryRedteamReportShareStore, RedteamJobStore, RedteamReportShareStore,
};
use crate::runs::{MemoryRunStore, RunStore};
use crate::team::{MemoryTeamStore, TeamStore};
use crate::tool_metadata::{MemoryToolMetadataStore, ToolMetadataStore};
use crate::traces::{MemoryTraceStore, TraceStore};

use super::postgres_adapters::*;

#[cfg(feature = "postgres")]
#[allow(clippy::type_complexity)]
pub(super) async fn build_postgres_layer(
    database_url: Option<String>,
    fallback_policies: &[Policy],
) -> Result<(
    Arc<dyn AgentStore>,
    Arc<dyn ProfileResolver>,
    Arc<dyn PolicyStore>,
    Arc<dyn TraceStore>,
    Arc<dyn RunStore>,
    Arc<dyn AnalyticsStore>,
    Arc<dyn HumanReviewStore>,
    Arc<dyn KnowledgeStore>,
    Arc<dyn ApiKeyStore>,
    Arc<dyn EnvironmentStore>,
    Arc<dyn SettingsStore>,
    Arc<dyn UserStore>,
    Arc<dyn TeamStore>,
    Arc<dyn GatewayStore>,
    Arc<dyn ToolMetadataStore>,
    Arc<dyn ToolMetadataProvider>,
    Arc<dyn LabelPolicyStore>,
    Arc<dyn LabelPolicyProvider>,
    Option<mpsc::Sender<TraceWrite>>,
    Option<Arc<EscalationRepo>>,
    Arc<dyn RedteamJobStore>,
    Arc<dyn RedteamReportShareStore>,
)> {
    let url = database_url.or_else(|| std::env::var("DATABASE_URL").ok());

    let Some(url) = url else {
        tracing::warn!(
            "DATABASE_URL not set — running memory-only (no trace persistence, no profile durability)"
        );
        let mem = Arc::new(MemoryAgentStore::new());
        let tool_metadata = Arc::new(MemoryToolMetadataStore::new());
        let label_policy = Arc::new(MemoryLabelPolicyStore::new());
        return Ok((
            mem.clone() as Arc<dyn AgentStore>,
            mem as Arc<dyn ProfileResolver>,
            Arc::new(MemoryPolicyStore::with_policies(fallback_policies)) as Arc<dyn PolicyStore>,
            Arc::new(MemoryTraceStore) as Arc<dyn TraceStore>,
            Arc::new(MemoryRunStore::new()) as Arc<dyn RunStore>,
            Arc::new(MemoryAnalyticsStore::new()) as Arc<dyn AnalyticsStore>,
            Arc::new(MemoryHumanReviewStore::new()) as Arc<dyn HumanReviewStore>,
            Arc::new(MemoryKnowledgeStore::new()) as Arc<dyn KnowledgeStore>,
            Arc::new(MemoryApiKeyStore::new()) as Arc<dyn ApiKeyStore>,
            Arc::new(MemoryEnvironmentStore::new()) as Arc<dyn EnvironmentStore>,
            Arc::new(MemorySettingsStore::new()) as Arc<dyn SettingsStore>,
            Arc::new(MemoryUserStore::new()) as Arc<dyn UserStore>,
            Arc::new(MemoryTeamStore::new()) as Arc<dyn TeamStore>,
            Arc::new(MemoryGatewayStore::new()) as Arc<dyn GatewayStore>,
            tool_metadata.clone() as Arc<dyn ToolMetadataStore>,
            tool_metadata as Arc<dyn ToolMetadataProvider>,
            label_policy.clone() as Arc<dyn LabelPolicyStore>,
            label_policy as Arc<dyn LabelPolicyProvider>,
            None,
            None,
            Arc::new(MemoryRedteamJobStore::new()) as Arc<dyn RedteamJobStore>,
            Arc::new(MemoryRedteamReportShareStore::new()) as Arc<dyn RedteamReportShareStore>,
        ));
    };

    migrate_postgres(&url)
        .await
        .map_err(|e| anyhow::anyhow!("migrate: {e}"))?;
    let pool = connect_postgres(&url, 20)
        .await
        .map_err(|e| anyhow::anyhow!("connect Postgres: {e}"))?;
    tracing::info!("Postgres connected and migrated");

    let repo = Arc::new(AgentRepo::new(pool.clone()));
    let adapter = PostgresAgentAdapter::new(repo);
    let policy_repo = Arc::new(PolicyRepo::new(pool.clone()));
    let policy_adapter = PostgresPolicyAdapter::new(policy_repo);
    let trace_adapter = PostgresTraceAdapter::new(Arc::new(TraceRepo::new(pool.clone())));
    let run_adapter = PostgresRunAdapter::new(Arc::new(RunRepo::new(pool.clone())));
    let analytics_adapter =
        PostgresAnalyticsAdapter::new(Arc::new(AnalyticsRepo::new(pool.clone())));
    let human_review_adapter =
        PostgresHumanReviewAdapter::new(Arc::new(tl_storage::HumanReviewRepo::new(pool.clone())));
    let knowledge_adapter =
        PostgresKnowledgeAdapter::new(Arc::new(KnowledgeRepo::new(pool.clone())));
    let dashboard_admin_adapter =
        PostgresDashboardAdminAdapter::new(Arc::new(DashboardAdminRepo::new(pool.clone())));
    let environment_adapter =
        PostgresEnvironmentAdapter::new(Arc::new(EnvironmentRepo::new(pool.clone())));
    let user_repo = Arc::new(UserRepo::new(pool.clone()));
    let user_adapter = PostgresUserAdapter::new(user_repo);

    let team_adapter: Arc<dyn TeamStore> = Arc::new(crate::team::TeamRepoAdapter::new(
        TeamRepo::new(pool.clone()),
    ));
    let gateway_adapter = PostgresGatewayAdapter::new(Arc::new(GatewayRepo::new(pool.clone())));
    let tool_metadata_adapter =
        PostgresToolMetadataAdapter::new(Arc::new(ToolMetadataRepo::new(pool.clone())));
    let label_policy_adapter =
        PostgresLabelPolicyAdapter::new(Arc::new(SourceLabelPolicyRepo::new(pool.clone())));

    let redteam_adapter =
        PostgresRedteamJobAdapter::new(Arc::new(RedteamJobRepo::new(pool.clone())));
    let redteam_share_adapter =
        PostgresRedteamReportShareAdapter::new(Arc::new(RedteamReportShareRepo::new(pool.clone())));

    let (tx, _handle) = spawn_writer(pool.clone(), WriterConfig::default());
    tracing::info!("trace writer spawned");

    let escalation_repo = Arc::new(EscalationRepo::new(pool));

    Ok((
        adapter.clone() as Arc<dyn AgentStore>,
        adapter as Arc<dyn ProfileResolver>,
        policy_adapter as Arc<dyn PolicyStore>,
        trace_adapter as Arc<dyn TraceStore>,
        run_adapter as Arc<dyn RunStore>,
        analytics_adapter as Arc<dyn AnalyticsStore>,
        human_review_adapter as Arc<dyn HumanReviewStore>,
        knowledge_adapter as Arc<dyn KnowledgeStore>,
        dashboard_admin_adapter.clone() as Arc<dyn ApiKeyStore>,
        environment_adapter as Arc<dyn EnvironmentStore>,
        dashboard_admin_adapter as Arc<dyn SettingsStore>,
        user_adapter as Arc<dyn UserStore>,
        team_adapter,
        gateway_adapter as Arc<dyn GatewayStore>,
        tool_metadata_adapter.clone() as Arc<dyn ToolMetadataStore>,
        tool_metadata_adapter as Arc<dyn ToolMetadataProvider>,
        label_policy_adapter.clone() as Arc<dyn LabelPolicyStore>,
        label_policy_adapter as Arc<dyn LabelPolicyProvider>,
        Some(tx),
        Some(escalation_repo),
        redteam_adapter as Arc<dyn RedteamJobStore>,
        redteam_share_adapter as Arc<dyn RedteamReportShareStore>,
    ))
}
