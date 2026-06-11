use std::sync::Arc;

use async_trait::async_trait;
use tl_cache::MokaCache;
use tl_core::AgentProfile;
use tl_engine::{
    Engine, EventPipelineCtx, FuzzyChecker, HandlerCtx, NoOpFuzzyChecker, ProfileResolver,
};
use tl_llm::LlmRouter;
#[cfg(not(feature = "postgres"))]
use tl_policy::Policy;

use crate::agents::{AgentStore, MemoryAgentStore};
#[cfg(not(feature = "postgres"))]
use crate::analytics::AnalyticsStore;
use crate::analytics::MemoryAnalyticsStore;
use crate::auth_user::MemoryUserStore;
#[cfg(not(feature = "postgres"))]
use crate::auth_user::UserStore;
#[cfg(not(feature = "postgres"))]
use crate::dashboard_admin::{ApiKeyStore, SettingsStore};
use crate::dashboard_admin::{MemoryApiKeyStore, MemorySettingsStore};
#[cfg(not(feature = "postgres"))]
use crate::environments::EnvironmentStore;
use crate::environments::MemoryEnvironmentStore;
#[cfg(not(feature = "postgres"))]
use crate::gateway::GatewayStore;
use crate::gateway::MemoryGatewayStore;
#[cfg(not(feature = "postgres"))]
use crate::human_review::HumanReviewStore;
use crate::human_review::MemoryHumanReviewStore;
#[cfg(not(feature = "postgres"))]
use crate::knowledge_sources::KnowledgeStore;
use crate::knowledge_sources::MemoryKnowledgeStore;
#[cfg(not(feature = "postgres"))]
use crate::label_policy::LabelPolicyStore;
use crate::label_policy::MemoryLabelPolicyStore;
use crate::policies::{MemoryPolicyStore, PolicyStore};
use crate::runs::MemoryRunStore;
#[cfg(not(feature = "postgres"))]
use crate::runs::RunStore;
use crate::team::MemoryTeamStore;
#[cfg(not(feature = "postgres"))]
use crate::team::TeamStore;
use crate::tool_metadata::MemoryToolMetadataStore;
#[cfg(not(feature = "postgres"))]
use crate::tool_metadata::ToolMetadataStore;
use crate::traces::MemoryTraceStore;
#[cfg(not(feature = "postgres"))]
use crate::traces::TraceStore;

use super::app_state::AppState;

/// Build an in-memory `AppState` from the given engine. Useful in
/// tests and for callers that construct the engine themselves
/// (e.g. plugging a custom `TierRunner` for deterministic mocks).
/// Skips all I/O — no Postgres, no llm-routing, no policy directory.
pub fn memory_app_state(engine: Arc<Engine>) -> AppState {
    let mem = Arc::new(MemoryAgentStore::new());
    let agent_store: Arc<dyn AgentStore> = mem.clone();
    let profile_resolver: Arc<dyn ProfileResolver> = mem;
    let policy_store: Arc<dyn PolicyStore> =
        Arc::new(MemoryPolicyStore::with_policies(engine.policies()));
    let cache: Arc<MokaCache> = Arc::new(MokaCache::with_defaults());
    let fuzzy: Arc<dyn FuzzyChecker> = Arc::new(NoOpFuzzyChecker);
    let llm = Arc::new(LlmRouter::empty());
    let handler_ctx = HandlerCtx {
        profile_resolver,
        cache,
        fuzzy,
        llm,
    };
    // One shared registry instance backs both the control-plane CRUD
    // surface and the event pipeline's runtime resolution.
    let tool_metadata = Arc::new(MemoryToolMetadataStore::new());
    let label_policy = Arc::new(MemoryLabelPolicyStore::new());
    AppState {
        engine,
        handler_ctx,
        event_pipeline: Arc::new(EventPipelineCtx {
            tool_metadata: tool_metadata.clone(),
            label_resolver: Arc::new(tl_engine::PolicyLabelResolver::new(label_policy.clone())),
            provenance_resolver: Arc::new(tl_engine::ProvenancePropagator),
            checkers: vec![
                Arc::new(tl_engine::InformationFlowChecker),
                Arc::new(tl_engine::MemoryChecker),
                Arc::new(tl_engine::ParameterAuthChecker),
                Arc::new(tl_engine::ApprovalChecker),
            ],
            composer: Arc::new(tl_engine::ModeAwareDecisionComposer),
            ..EventPipelineCtx::no_op()
        }),
        #[cfg(feature = "postgres")]
        trace_tx: None,
        agent_store,
        policy_store,
        tool_metadata_store: tool_metadata,
        label_policy_store: label_policy,
        trace_store: Arc::new(MemoryTraceStore),
        run_store: Arc::new(MemoryRunStore::new()),
        analytics_store: Arc::new(MemoryAnalyticsStore::new()),
        human_review_store: Arc::new(MemoryHumanReviewStore::new()),
        knowledge_store: Arc::new(MemoryKnowledgeStore::new()),
        api_key_store: Arc::new(MemoryApiKeyStore::new()),
        environment_store: Arc::new(MemoryEnvironmentStore::new()),
        settings_store: Arc::new(MemorySettingsStore::new()),
        user_store: Arc::new(MemoryUserStore::new()),
        password_auth_enabled: true,
        hosted_user_approval_required: false,
        workspace_self_service_enabled: true,
        team_store: Arc::new(MemoryTeamStore::new()),
        gateway_store: Arc::new(MemoryGatewayStore::new()),
        jwt_signer: None,
        escalation_tx: None,
    }
}

#[cfg(not(feature = "postgres"))]
#[allow(clippy::type_complexity)]
pub(super) fn build_memory_layer(
    policies: &[Policy],
) -> (
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
    Arc<dyn tl_engine::ToolMetadataProvider>,
    Arc<dyn LabelPolicyStore>,
    Arc<dyn tl_engine::LabelPolicyProvider>,
) {
    let mem = Arc::new(MemoryAgentStore::new());
    let tool_metadata = Arc::new(MemoryToolMetadataStore::new());
    let label_policy = Arc::new(MemoryLabelPolicyStore::new());
    (
        mem.clone() as Arc<dyn AgentStore>,
        mem as Arc<dyn ProfileResolver>,
        Arc::new(MemoryPolicyStore::with_policies(policies)) as Arc<dyn PolicyStore>,
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
        tool_metadata as Arc<dyn tl_engine::ToolMetadataProvider>,
        label_policy.clone() as Arc<dyn LabelPolicyStore>,
        label_policy as Arc<dyn tl_engine::LabelPolicyProvider>,
    )
}

#[async_trait]
impl ProfileResolver for MemoryAgentStore {
    async fn resolve(&self, workspace_id: &str, agent_id: &str) -> Option<Arc<AgentProfile>> {
        AgentStore::get(self, workspace_id, agent_id).await.ok()
    }
}
