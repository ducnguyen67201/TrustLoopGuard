use std::sync::Arc;

use tl_engine::{Engine, EventPipelineCtx, HandlerCtx};
use tokio::sync::mpsc as tokio_mpsc;

use crate::agents::AgentStore;
use crate::analytics::AnalyticsStore;
use crate::auth_user::UserStore;
use crate::authorization::{AuthorizationCoordinator, AuthorizationStore};
use crate::budget_alerts::BudgetAlertStore;
use crate::dashboard_admin::{ApiKeyStore, SettingsStore};
use crate::environments::EnvironmentStore;
use crate::escalation::{EscalationPayload, WebhookDelivery};
use crate::evaluations::EvaluationStore;
use crate::financial::{FinancialExecutor, FinancialStore};
use crate::gateway::GatewayStore;
use crate::github_integration::{GitHubIntegrationMessage, GitHubIntegrationStore};
use crate::human_review::HumanReviewStore;
use crate::knowledge_sources::KnowledgeStore;
use crate::label_policy::LabelPolicyStore;
use crate::llm_pricing::LlmPricingStore;
use crate::llm_usage::LlmUsageStore;
use crate::mcp_gateway::McpGatewayStore;
use crate::oauth_store::OAuthStore;
use crate::otel::OtelStore;
use crate::policies::PolicyStore;
use crate::redteam::{DispatchJob, RedteamJobStore, RedteamPlanStore, RedteamReportShareStore};
use crate::runs::RunStore;
use crate::team::TeamStore;
use crate::tool_metadata::ToolMetadataStore;
use crate::traces::TraceStore;

#[derive(Clone)]
pub struct AppState {
    pub engine: Arc<Engine>,
    pub handler_ctx: HandlerCtx,
    /// Event pipeline stage chain. Observe-only: the decision passes
    /// through unchanged and the normalized `GuardEvent` — including
    /// tool-metadata resolution evidence — is collected for traces.
    pub event_pipeline: Arc<EventPipelineCtx>,
    pub agent_store: Arc<dyn AgentStore>,
    pub policy_store: Arc<dyn PolicyStore>,
    /// Workspace tool metadata registry (control-plane CRUD surface).
    /// The event pipeline reads the same backing store through its
    /// `ToolMetadataProvider` seam.
    pub tool_metadata_store: Arc<dyn ToolMetadataStore>,
    /// Canonical approval, grant, lease, and receipt state shared by every
    /// authorization domain.
    pub authorization_store: Arc<dyn AuthorizationStore>,
    /// Orchestrates typed policy findings, grants, approvals, leases, and
    /// receipts for every domain over the shared store.
    pub authorization_coordinator: Arc<AuthorizationCoordinator>,
    /// Workspace source label policies (control-plane CRUD surface).
    /// The event pipeline reads the same backing store through its
    /// `LabelPolicyProvider` seam.
    pub label_policy_store: Arc<dyn LabelPolicyStore>,
    pub trace_store: Arc<dyn TraceStore>,
    pub run_store: Arc<dyn RunStore>,
    /// Agent-scoped evaluation configuration, frozen manifests, and results.
    pub evaluation_store: Arc<dyn EvaluationStore>,
    pub otel_store: Arc<dyn OtelStore>,
    pub analytics_store: Arc<dyn AnalyticsStore>,
    pub human_review_store: Arc<dyn HumanReviewStore>,
    pub financial_store: Arc<dyn FinancialStore>,
    /// Optional domain executor override. Production defaults to the
    /// payment HTTP executor; tests and embedded deployments may supply a
    /// deterministic executor without changing authorization behavior.
    pub financial_executor: Option<Arc<dyn FinancialExecutor>>,
    /// LLM gateway metering log. The gateway budget hook writes one
    /// event per metered chat completion and sums spend windows here;
    /// `GET /v1/llm-usage` reads the same rows.
    pub llm_usage_store: Arc<dyn LlmUsageStore>,
    /// Workspace-editable model → price rows for gateway metering
    /// (`/v1/llm-pricing`). Built-in defaults in code fall back for
    /// models with no workspace row.
    pub llm_pricing_store: Arc<dyn LlmPricingStore>,
    /// Budget alert threshold configs + firing log. Both spend paths
    /// (financial ledger, LLM metering) evaluate against this store
    /// right after recording a spend.
    pub budget_alert_store: Arc<dyn BudgetAlertStore>,
    /// Channel into the generic webhook delivery worker that carries
    /// budget alert firings. `None` (memory test states) still records
    /// firings — sends are just skipped, mirroring `escalation_tx`.
    pub budget_alert_tx: Option<tokio_mpsc::Sender<WebhookDelivery>>,
    pub knowledge_store: Arc<dyn KnowledgeStore>,
    pub api_key_store: Arc<dyn ApiKeyStore>,
    pub environment_store: Arc<dyn EnvironmentStore>,
    pub settings_store: Arc<dyn SettingsStore>,
    /// Backing store for username/password accounts. Memory-only when
    /// the server runs without Postgres.
    pub user_store: Arc<dyn UserStore>,
    /// Username/password auth is a local-development bootstrap path.
    /// Staging/production use OAuth provider login plus Rust-owned
    /// identity linking.
    pub password_auth_enabled: bool,
    /// Self-service workspace creation is allowed for local and
    /// self-hosted installs, but disabled on hosted stage/prod.
    pub workspace_self_service_enabled: bool,
    /// Backing store for workspace team membership + invites.
    pub team_store: Arc<dyn TeamStore>,
    /// Gateway provider connections and proxy routes.
    pub gateway_store: Arc<dyn GatewayStore>,
    /// Durable OAuth registrations, one-time codes, and rotating refresh tokens.
    pub oauth_store: Arc<dyn OAuthStore>,
    /// Durable server catalog and per-user MCP tool entitlements.
    pub mcp_gateway_store: Arc<dyn McpGatewayStore>,
    /// HS256 signer used to mint user-session JWTs on signup/login
    /// and verify them on protected `/v1/*` routes. `None` when
    /// `TL_JWT_SECRET` is unset — the server runs without per-user
    /// auth, and the web falls back to `TL_API_KEY` + header
    /// forwarding for identity.
    pub jwt_signer: Option<Arc<crate::jwt::JwtSigner>>,
    /// Channel into the escalation webhook worker. `None` when no
    /// `TL_ESCALATION_WEBHOOK_URL` is configured — deferred decisions
    /// are still produced, just never delivered downstream.
    pub escalation_tx: Option<tokio_mpsc::Sender<EscalationPayload>>,
    /// Durable store for red-team dispatch jobs + per-attack results.
    pub redteam_job_store: Arc<dyn RedteamJobStore>,
    /// Durable store for saved, named attack plans (per agent).
    pub redteam_plan_store: Arc<dyn RedteamPlanStore>,
    /// Durable store for shareable red-team report tokens.
    pub redteam_report_share_store: Arc<dyn RedteamReportShareStore>,
    /// Channel into the in-process red-team dispatch worker. `None` when
    /// `REDTEAM_RUNNER_URL` is unset — `POST /v1/redteam/dispatch`
    /// returns `503`.
    pub redteam_dispatch_tx: Option<tokio_mpsc::Sender<DispatchJob>>,
    /// Durable GitHub-assisted installation state and lifecycle.
    pub github_integration_store: Arc<dyn GitHubIntegrationStore>,
    /// Channel into the GitHub integration worker. `None` when GitHub App
    /// config or the integration LLM is missing.
    pub github_integration_tx: Option<tokio_mpsc::Sender<GitHubIntegrationMessage>>,
}

#[derive(Default)]
pub struct BuildOptions {
    /// Path to the policy YAML directory. Defaults to `TL_POLICY_DIR`
    /// env var, then `./policies`.
    pub policy_dir: Option<String>,
    /// Postgres connection string. Defaults to `DATABASE_URL`. When
    /// unset, the server runs memory-only.
    #[cfg(feature = "postgres")]
    pub database_url: Option<String>,
}
