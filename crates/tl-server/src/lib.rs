//! HTTP routes and OpenAPI doc. Split from `main.rs` so `tl-codegen` can
//! pull `ApiDoc` without booting the runtime.

pub mod agents;
pub mod analytics;
pub mod api;
pub mod app;
pub mod auth;
pub mod auth_user;
pub mod authorization;
pub mod budget_alerts;
pub mod dashboard_admin;
pub mod environments;
pub mod escalation;
pub mod financial;
pub mod gateway;
pub mod github_integration;
pub mod human_review;
pub mod jwt;
pub mod knowledge_sources;
pub mod label_policy;
pub mod llm_pricing;
pub mod llm_usage;
pub mod mcp_gateway;
pub mod oauth;
pub mod oauth_store;
pub mod policies;
pub mod redteam;
pub mod runs;
pub(crate) mod services;
pub mod state;
pub mod team;
pub mod tool_metadata;
pub mod traces;
pub use agents::{AgentState, AgentStore, AgentStoreError, MemoryAgentStore};
pub use analytics::{AnalyticsState, AnalyticsStore, AnalyticsStoreError, MemoryAnalyticsStore};
pub use api::guard::health;
pub(crate) use app::error::log_api_error;
pub use app::openapi::ApiDoc;
pub use app::router::router;
pub use auth::{AuthConfig, EnvError as AuthEnvError};
pub use auth_user::{AuthUserState, MemoryUserStore, UserStore, UserStoreError};
pub use budget_alerts::{
    BudgetAlertRuntime, BudgetAlertStore, BudgetAlertStoreError, MemoryBudgetAlertStore,
};
pub use dashboard_admin::{ApiKeyStore, DashboardAdminState, SettingsStore};
pub use environments::{
    EnvironmentState, EnvironmentStore, EnvironmentStoreError, MemoryEnvironmentStore,
};
pub use escalation::{
    spawn_escalation_worker, spawn_webhook_delivery_worker, EscalationConfig, EscalationPayload,
    RetryPolicy, WebhookDelivery,
};
pub use financial::{
    AgenticPaymentBudgetReservationRequest, FinancialAuthorizationService,
    FinancialBudgetConstraint, FinancialBudgetReservationOutcome,
    FinancialBudgetReservationRequest, FinancialBudgetViolation, FinancialBudgetWindow,
    FinancialLedgerEntryKind, FinancialState, FinancialStore, FinancialStoreError,
    MemoryFinancialStore,
};
pub use gateway::{build_seal_key, GatewayState, GatewayStore, MemoryGatewayStore};
pub use github_integration::{
    GitHubIntegrationState, GitHubIntegrationStore, GitHubIntegrationStoreError,
    MemoryGitHubIntegrationStore,
};
pub use human_review::{HumanReviewStore, HumanReviewStoreError, MemoryHumanReviewStore};
pub use label_policy::{
    LabelPolicyState, LabelPolicyStore, LabelPolicyStoreError, MemoryLabelPolicyStore,
};
pub use llm_pricing::{
    LlmPricingState, LlmPricingStore, LlmPricingStoreError, LlmPricingTable, MemoryLlmPricingStore,
};
pub use llm_usage::{LlmUsageState, LlmUsageStore, LlmUsageStoreError, MemoryLlmUsageStore};
pub use policies::{GuardrailState, MemoryPolicyStore, PolicyState, PolicyStore, PolicyStoreError};
pub use redteam::{MemoryRedteamJobStore, RedteamJobStore, RedteamJobStoreError, RedteamState};
pub use runs::{MemoryRunStore, RunState, RunStore, RunStoreError};
pub use state::{build_app_state, memory_app_state, AppState, BuildOptions};
pub use team::{MemoryTeamStore, TeamState, TeamStore, TeamStoreError};
pub use tool_metadata::{
    MemoryToolMetadataStore, ToolMetadataState, ToolMetadataStore, ToolMetadataStoreError,
};

#[cfg(test)]
mod architecture_tests {
    use super::*;
    use axum::Router;
    use std::sync::Arc;
    use utoipa::OpenApi;

    #[test]
    fn server_shell_exports_stay_stable_during_module_split() {
        let _: fn(AppState, Option<Arc<AuthConfig>>, [u8; 32]) -> Router = router;
        let _ = health;
        let _ = ApiDoc::openapi();

        let _: fn(AppState, Option<Arc<AuthConfig>>, [u8; 32]) -> Router =
            crate::app::router::router;
        let _ = crate::app::openapi::ApiDoc::openapi();
        let _ = crate::api::guard::health;
        let _: Option<crate::state::app_state::AppState> = None;
        let _: Option<crate::state::app_state::BuildOptions> = None;
        let _ = crate::state::memory::memory_app_state;
    }
}
