//! Decision-log storage. Trait-first so we can swap in-memory for Postgres
//! without touching the engine or server.
//!
//! Postgres types (`PostgresStore`, `migrate`) are gated behind the
//! `postgres` feature so the default tl-storage build doesn't compile
//! database dependencies.

use async_trait::async_trait;
use tl_core::Decision;

#[derive(Debug, Clone, thiserror::Error)]
pub enum StorageError {
    #[error("not found")]
    NotFound,
    #[error("conflict")]
    Conflict,
    #[error("internal: {0}")]
    Internal(String),
}

#[cfg(feature = "postgres")]
impl From<diesel::result::Error> for StorageError {
    fn from(e: diesel::result::Error) -> Self {
        match e {
            diesel::result::Error::NotFound => StorageError::NotFound,
            diesel::result::Error::DatabaseError(
                diesel::result::DatabaseErrorKind::UniqueViolation,
                _,
            ) => StorageError::Conflict,
            other => StorageError::Internal(other.to_string()),
        }
    }
}

#[async_trait]
pub trait DecisionStore: Send + Sync {
    async fn put(&self, decision: &Decision) -> Result<(), StorageError>;
    async fn get(&self, trace_id: &str) -> Result<Decision, StorageError>;
}

pub mod memory_store;
pub use memory_store::MemoryStore;

#[cfg(feature = "postgres")]
pub mod agent_repo;
#[cfg(feature = "postgres")]
pub mod analytics_repo;
#[cfg(feature = "postgres")]
pub mod authorization_repo;
#[cfg(feature = "postgres")]
pub mod budget_alert_repo;
#[cfg(feature = "postgres")]
pub mod dashboard_admin_repo;
#[cfg(feature = "postgres")]
pub mod environment_repo;
#[cfg(feature = "postgres")]
pub mod escalations;
#[cfg(feature = "postgres")]
pub mod evaluation_repo;
#[cfg(feature = "postgres")]
pub mod evaluation_worker_repo;
#[cfg(feature = "postgres")]
pub mod financial_repo;
#[cfg(feature = "postgres")]
pub mod gateway_repo;
#[cfg(feature = "postgres")]
pub mod github_integration_repo;
#[cfg(feature = "postgres")]
pub mod human_review_repo;
#[cfg(feature = "postgres")]
pub mod knowledge_repo;
#[cfg(feature = "postgres")]
pub mod llm_pricing_repo;
#[cfg(feature = "postgres")]
pub mod llm_usage_repo;
#[cfg(feature = "postgres")]
pub mod mcp_gateway_repo;
#[cfg(feature = "postgres")]
pub mod models;
#[cfg(feature = "postgres")]
pub mod oauth_repo;
#[cfg(feature = "postgres")]
pub mod otel_repo;
#[cfg(feature = "postgres")]
pub mod policy_repo;
#[cfg(feature = "postgres")]
pub mod postgres;
#[cfg(feature = "postgres")]
pub mod redteam_job_repo;
#[cfg(feature = "postgres")]
pub mod redteam_plan_repo;
#[cfg(feature = "postgres")]
pub mod redteam_report_share_repo;
#[cfg(feature = "postgres")]
pub mod run_repo;
#[cfg(feature = "postgres")]
pub mod schema;
#[cfg(feature = "postgres")]
pub mod team_repo;
#[cfg(feature = "postgres")]
pub mod tool_metadata_repo;
#[cfg(feature = "postgres")]
pub mod user_repo;
#[cfg(feature = "postgres")]
pub mod writer;

#[cfg(feature = "postgres")]
pub use agent_repo::AgentRepo;
#[cfg(feature = "postgres")]
pub use analytics_repo::AnalyticsRepo;
#[cfg(feature = "postgres")]
pub use authorization_repo::{
    AuthorizationRepo, CreateAuthorizationApproval, CreateAuthorizationIntent,
};
#[cfg(feature = "postgres")]
pub use budget_alert_repo::{
    BudgetAlertRepo, NewBudgetAlertConfigParams, NewBudgetAlertFiringParams,
    StoredBudgetAlertConfig, StoredBudgetAlertFiring, UpdateBudgetAlertConfigParams,
};
#[cfg(feature = "postgres")]
pub use dashboard_admin_repo::DashboardAdminRepo;
#[cfg(feature = "postgres")]
pub use environment_repo::EnvironmentRepo;
#[cfg(feature = "postgres")]
pub use escalations::{EscalationRepo, EscalationRow};
#[cfg(feature = "postgres")]
pub use evaluation_repo::EvaluationRepo;
#[cfg(feature = "postgres")]
pub use evaluation_worker_repo::{
    CaptureAdvanceResult, EvaluationJobWork, FrozenEvaluationPolicy, PersistEvaluationFinding,
    PersistEvaluationResult,
};
#[cfg(feature = "postgres")]
pub use financial_repo::{
    FinancialBudgetConstraint, FinancialBudgetViolation, FinancialBudgetWindow,
    FinancialLedgerEntryKind, FinancialRepo, ReserveAgenticPaymentBudgetRequest,
    ReserveFinancialActionBudgetRequest, ReserveFinancialActionBudgetResult, StoredFinancialAction,
    StoredFinancialActionEvent, StoredFinancialActionOutcome, StoredFinancialLedgerEntry,
    StoredFinancialReceipt,
};
#[cfg(feature = "postgres")]
pub use gateway_repo::{
    GatewayProviderConnectionSecret, GatewayRepo, GatewayRoutePatch, ResolvedGatewayRoute,
};
#[cfg(feature = "postgres")]
pub use github_integration_repo::{
    CreateConnection as GitHubCreateConnection, CreateJob as GitHubCreateJob,
    GitHubIntegrationRepo, JobTransition as GitHubJobTransition,
    NewInstallationState as NewGitHubInstallationState,
    UpsertInstallation as GitHubUpsertInstallation,
};
#[cfg(feature = "postgres")]
pub use human_review_repo::{HumanReviewAnalyticsFilter, HumanReviewRepo};
#[cfg(feature = "postgres")]
pub use knowledge_repo::{
    KnowledgeFileRow, KnowledgeRepo, KnowledgeSourceRow, NewKnowledgeFile, NewKnowledgeSource,
};
#[cfg(feature = "postgres")]
pub use llm_pricing_repo::{LlmPricingRepo, StoredLlmModelPrice};
#[cfg(feature = "postgres")]
pub use llm_usage_repo::{
    LlmBudgetCapsNanos, LlmBudgetWindow, LlmBudgetWindowSnapshot, LlmUsageBucketRow,
    LlmUsageEventFilter, LlmUsageGroupBy, LlmUsageRepo, NewLlmBudgetReservationParams,
    NewLlmUsageEventParams, ReserveLlmBudgetResult, StoredLlmUsageEvent,
};
#[cfg(feature = "postgres")]
pub use mcp_gateway_repo::{
    CatalogToolInput, CredentialPatch, EntitledMcpTool, McpConnectionPatch, McpConnectionSecret,
    McpGatewayRepo, NewMcpConnection,
};
#[cfg(feature = "postgres")]
pub use models::UserRecord;
#[cfg(feature = "postgres")]
pub use oauth_repo::{
    NewOAuthAuthorizationCode, NewOAuthRefreshToken, OAuthRepo, StoredOAuthAuthorizationCode,
    StoredOAuthClient, StoredOAuthRefreshToken,
};
#[cfg(feature = "postgres")]
pub use otel_repo::{OtelIngestBatch, OtelIngestResult, OtelRepo};
#[cfg(feature = "postgres")]
pub use policy_repo::{PolicyRepo, PolicyRow};
#[cfg(feature = "postgres")]
pub use postgres::{
    connect as connect_postgres, migrate as migrate_postgres, DbPool, PostgresStore,
};
#[cfg(feature = "postgres")]
pub use redteam_job_repo::{
    JobCounts, RedteamAttackRecordFilter, RedteamJobFilter, RedteamJobRepo,
};
#[cfg(feature = "postgres")]
pub use redteam_plan_repo::RedteamPlanRepo;
#[cfg(feature = "postgres")]
pub use redteam_report_share_repo::{NewShare, RedteamReportShareRepo, ReportShareRow};
#[cfg(feature = "postgres")]
pub use run_repo::{RunFilter, RunRepo};
#[cfg(feature = "postgres")]
pub use team_repo::{AddMemberOutcome, TeamRepo, WorkspaceDeletionOutcome};
#[cfg(feature = "postgres")]
pub use tool_metadata_repo::{StoredToolMetadata, ToolMetadataRepo};
#[cfg(feature = "postgres")]
pub use trace_repo::{TraceRepo, TraceRow};
#[cfg(feature = "postgres")]
pub mod trace_repo;
#[cfg(feature = "postgres")]
pub use user_repo::UserRepo;
#[cfg(feature = "postgres")]
pub use writer::{spawn_writer, TraceWrite, WriterConfig};
