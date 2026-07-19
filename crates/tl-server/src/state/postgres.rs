#![cfg(feature = "postgres")]

use std::sync::Arc;

use anyhow::Result;
use tl_engine::LabelPolicyProvider;
use tl_engine::ProfileResolver;
use tl_engine::ToolMetadataProvider;
use tl_storage::{
    connect_postgres, migrate_postgres, spawn_writer, AgentRepo, AnalyticsRepo, AuthorizationRepo,
    BudgetAlertRepo, DashboardAdminRepo, EnvironmentRepo, EscalationRepo, FinancialRepo,
    GatewayRepo, GitHubIntegrationRepo, KnowledgeRepo, LlmPricingRepo, LlmUsageRepo,
    McpGatewayRepo, OAuthRepo, PolicyRepo, RedteamJobRepo, RedteamPlanRepo, RedteamReportShareRepo,
    RunRepo, TeamRepo, ToolMetadataRepo, TraceRepo, UserRepo, WriterConfig,
};

use crate::agents::{AgentStore, MemoryAgentStore};
use crate::analytics::{AnalyticsStore, MemoryAnalyticsStore};
use crate::auth_user::{MemoryUserStore, UserStore};
use crate::authorization::{AuthorizationStore, MemoryAuthorizationStore};
use crate::budget_alerts::{BudgetAlertStore, MemoryBudgetAlertStore};
use crate::dashboard_admin::{ApiKeyStore, MemoryApiKeyStore, MemorySettingsStore, SettingsStore};
use crate::environments::{EnvironmentStore, MemoryEnvironmentStore};
use crate::financial::{FinancialStore, MemoryFinancialStore};
use crate::gateway::{GatewayStore, MemoryGatewayStore};
use crate::github_integration::{GitHubIntegrationStore, MemoryGitHubIntegrationStore};
use crate::human_review::{HumanReviewStore, MemoryHumanReviewStore};
use crate::knowledge_sources::{KnowledgeStore, MemoryKnowledgeStore};
use crate::label_policy::{LabelPolicyStore, MemoryLabelPolicyStore};
use crate::llm_pricing::{LlmPricingStore, MemoryLlmPricingStore};
use crate::llm_usage::{LlmUsageStore, MemoryLlmUsageStore};
use crate::mcp_gateway::{McpGatewayStore, MemoryMcpGatewayStore};
use crate::oauth_store::{MemoryOAuthStore, OAuthStore};
use crate::policies::{MemoryPolicyStore, PolicyStore};
use crate::redteam::{
    MemoryRedteamJobStore, MemoryRedteamPlanStore, MemoryRedteamReportShareStore, RedteamJobStore,
    RedteamPlanStore, RedteamReportShareStore,
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
    fallback_policies: &super::LoadedPolicies,
) -> Result<(
    Arc<dyn AgentStore>,
    Arc<dyn ProfileResolver>,
    Arc<dyn PolicyStore>,
    Arc<dyn TraceStore>,
    Arc<dyn RunStore>,
    Arc<dyn AnalyticsStore>,
    Arc<dyn HumanReviewStore>,
    Arc<dyn FinancialStore>,
    Arc<dyn LlmUsageStore>,
    Arc<dyn LlmPricingStore>,
    Arc<dyn BudgetAlertStore>,
    Arc<dyn KnowledgeStore>,
    Arc<dyn ApiKeyStore>,
    Arc<dyn EnvironmentStore>,
    Arc<dyn SettingsStore>,
    Arc<dyn UserStore>,
    Arc<dyn TeamStore>,
    Arc<dyn GatewayStore>,
    Arc<dyn OAuthStore>,
    Arc<dyn McpGatewayStore>,
    Arc<dyn ToolMetadataStore>,
    Arc<dyn ToolMetadataProvider>,
    Arc<dyn AuthorizationStore>,
    Arc<dyn LabelPolicyStore>,
    Arc<dyn LabelPolicyProvider>,
    Option<Arc<EscalationRepo>>,
    Arc<dyn RedteamJobStore>,
    Arc<dyn RedteamPlanStore>,
    Arc<dyn RedteamReportShareStore>,
    Arc<dyn GitHubIntegrationStore>,
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
            Arc::new(MemoryPolicyStore::with_policy_sets(
                &fallback_policies.content,
                &fallback_policies.families,
            )) as Arc<dyn PolicyStore>,
            Arc::new(MemoryTraceStore::default()) as Arc<dyn TraceStore>,
            Arc::new(MemoryRunStore::new()) as Arc<dyn RunStore>,
            Arc::new(MemoryAnalyticsStore::new()) as Arc<dyn AnalyticsStore>,
            Arc::new(MemoryHumanReviewStore::new()) as Arc<dyn HumanReviewStore>,
            Arc::new(MemoryFinancialStore::new()) as Arc<dyn FinancialStore>,
            Arc::new(MemoryLlmUsageStore::new()) as Arc<dyn LlmUsageStore>,
            Arc::new(MemoryLlmPricingStore::new()) as Arc<dyn LlmPricingStore>,
            Arc::new(MemoryBudgetAlertStore::new()) as Arc<dyn BudgetAlertStore>,
            Arc::new(MemoryKnowledgeStore::new()) as Arc<dyn KnowledgeStore>,
            Arc::new(MemoryApiKeyStore::new()) as Arc<dyn ApiKeyStore>,
            Arc::new(MemoryEnvironmentStore::new()) as Arc<dyn EnvironmentStore>,
            Arc::new(MemorySettingsStore::new()) as Arc<dyn SettingsStore>,
            Arc::new(MemoryUserStore::new()) as Arc<dyn UserStore>,
            Arc::new(MemoryTeamStore::new()) as Arc<dyn TeamStore>,
            Arc::new(MemoryGatewayStore::new()) as Arc<dyn GatewayStore>,
            Arc::new(MemoryOAuthStore::default()) as Arc<dyn OAuthStore>,
            Arc::new(MemoryMcpGatewayStore::new()) as Arc<dyn McpGatewayStore>,
            tool_metadata.clone() as Arc<dyn ToolMetadataStore>,
            tool_metadata as Arc<dyn ToolMetadataProvider>,
            Arc::new(MemoryAuthorizationStore::new()) as Arc<dyn AuthorizationStore>,
            label_policy.clone() as Arc<dyn LabelPolicyStore>,
            label_policy as Arc<dyn LabelPolicyProvider>,
            None,
            Arc::new(MemoryRedteamJobStore::new()) as Arc<dyn RedteamJobStore>,
            Arc::new(MemoryRedteamPlanStore::new()) as Arc<dyn RedteamPlanStore>,
            Arc::new(MemoryRedteamReportShareStore::new()) as Arc<dyn RedteamReportShareStore>,
            Arc::new(MemoryGitHubIntegrationStore::new()) as Arc<dyn GitHubIntegrationStore>,
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
    let policy_adapter = PostgresPolicyAdapter::new(policy_repo.clone());
    let (trace_writer_tx, _trace_writer_handle) =
        spawn_writer(pool.clone(), WriterConfig::default());
    tracing::info!("trace writer spawned");
    let trace_adapter =
        PostgresTraceAdapter::new(Arc::new(TraceRepo::new(pool.clone())), trace_writer_tx);
    let run_adapter = PostgresRunAdapter::new(Arc::new(RunRepo::new(pool.clone())));
    let analytics_adapter =
        PostgresAnalyticsAdapter::new(Arc::new(AnalyticsRepo::new(pool.clone())));
    let human_review_adapter =
        PostgresHumanReviewAdapter::new(Arc::new(tl_storage::HumanReviewRepo::new(pool.clone())));
    let financial_adapter =
        PostgresFinancialAdapter::new(Arc::new(FinancialRepo::new(pool.clone())));
    let llm_usage_adapter = PostgresLlmUsageAdapter::new(Arc::new(LlmUsageRepo::new(pool.clone())));
    let budget_alert_adapter =
        PostgresBudgetAlertAdapter::new(Arc::new(BudgetAlertRepo::new(pool.clone())));
    let llm_pricing_adapter =
        PostgresLlmPricingAdapter::new(Arc::new(LlmPricingRepo::new(pool.clone())));
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
    let oauth_adapter = PostgresOAuthAdapter::new(Arc::new(OAuthRepo::new(pool.clone())));
    let mcp_gateway_adapter =
        PostgresMcpGatewayAdapter::new(Arc::new(McpGatewayRepo::new(pool.clone())));
    let tool_metadata_adapter =
        PostgresToolMetadataAdapter::new(Arc::new(ToolMetadataRepo::new(pool.clone())));
    let authorization_adapter =
        PostgresAuthorizationAdapter::new(Arc::new(AuthorizationRepo::new(pool.clone())));
    let label_policy_adapter = PostgresLabelPolicyAdapter::new(policy_repo);

    let redteam_adapter =
        PostgresRedteamJobAdapter::new(Arc::new(RedteamJobRepo::new(pool.clone())));
    let redteam_plan_adapter =
        PostgresRedteamPlanAdapter::new(Arc::new(RedteamPlanRepo::new(pool.clone())));
    let redteam_share_adapter =
        PostgresRedteamReportShareAdapter::new(Arc::new(RedteamReportShareRepo::new(pool.clone())));
    let github_integration_adapter =
        PostgresGitHubIntegrationAdapter::new(Arc::new(GitHubIntegrationRepo::new(pool.clone())));

    let escalation_repo = Arc::new(EscalationRepo::new(pool));

    Ok((
        adapter.clone() as Arc<dyn AgentStore>,
        adapter as Arc<dyn ProfileResolver>,
        policy_adapter as Arc<dyn PolicyStore>,
        trace_adapter as Arc<dyn TraceStore>,
        run_adapter as Arc<dyn RunStore>,
        analytics_adapter as Arc<dyn AnalyticsStore>,
        human_review_adapter as Arc<dyn HumanReviewStore>,
        financial_adapter as Arc<dyn FinancialStore>,
        llm_usage_adapter as Arc<dyn LlmUsageStore>,
        llm_pricing_adapter as Arc<dyn LlmPricingStore>,
        budget_alert_adapter as Arc<dyn BudgetAlertStore>,
        knowledge_adapter as Arc<dyn KnowledgeStore>,
        dashboard_admin_adapter.clone() as Arc<dyn ApiKeyStore>,
        environment_adapter as Arc<dyn EnvironmentStore>,
        dashboard_admin_adapter as Arc<dyn SettingsStore>,
        user_adapter as Arc<dyn UserStore>,
        team_adapter,
        gateway_adapter as Arc<dyn GatewayStore>,
        Arc::new(oauth_adapter) as Arc<dyn OAuthStore>,
        Arc::new(mcp_gateway_adapter) as Arc<dyn McpGatewayStore>,
        tool_metadata_adapter.clone() as Arc<dyn ToolMetadataStore>,
        tool_metadata_adapter as Arc<dyn ToolMetadataProvider>,
        authorization_adapter as Arc<dyn AuthorizationStore>,
        label_policy_adapter.clone() as Arc<dyn LabelPolicyStore>,
        label_policy_adapter as Arc<dyn LabelPolicyProvider>,
        Some(escalation_repo),
        redteam_adapter as Arc<dyn RedteamJobStore>,
        redteam_plan_adapter as Arc<dyn RedteamPlanStore>,
        redteam_share_adapter as Arc<dyn RedteamReportShareStore>,
        github_integration_adapter as Arc<dyn GitHubIntegrationStore>,
    ))
}
