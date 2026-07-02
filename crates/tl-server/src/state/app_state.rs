use std::sync::Arc;

use tl_engine::{Engine, EventPipelineCtx, HandlerCtx};
#[cfg(feature = "postgres")]
#[cfg(feature = "postgres")]
use tokio::sync::mpsc as tokio_mpsc;

use crate::agents::AgentStore;
use crate::analytics::AnalyticsStore;
use crate::auth_user::UserStore;
use crate::dashboard_admin::{ApiKeyStore, SettingsStore};
use crate::environments::EnvironmentStore;
use crate::escalation::EscalationPayload;
use crate::gateway::GatewayStore;
use crate::human_review::HumanReviewStore;
use crate::knowledge_sources::KnowledgeStore;
use crate::label_policy::LabelPolicyStore;
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
    /// Workspace source label policies (control-plane CRUD surface).
    /// The event pipeline reads the same backing store through its
    /// `LabelPolicyProvider` seam.
    pub label_policy_store: Arc<dyn LabelPolicyStore>,
    pub trace_store: Arc<dyn TraceStore>,
    pub run_store: Arc<dyn RunStore>,
    pub analytics_store: Arc<dyn AnalyticsStore>,
    pub human_review_store: Arc<dyn HumanReviewStore>,
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
    /// Hosted TrustLoopGuard staging/production requires a manual
    /// users.is_approved=true flag before dashboard users can call
    /// user-scoped routes.
    pub hosted_user_approval_required: bool,
    /// Self-service workspace creation is allowed for local and
    /// self-hosted installs, but disabled on hosted stage/prod.
    pub workspace_self_service_enabled: bool,
    /// Backing store for workspace team membership + invites.
    pub team_store: Arc<dyn TeamStore>,
    /// Gateway provider connections, enforcement profiles, and proxy routes.
    pub gateway_store: Arc<dyn GatewayStore>,
    /// HS256 signer used to mint user-session JWTs on signup/login
    /// and verify them on protected `/v1/*` routes. `None` when
    /// `TL_JWT_SECRET` is unset — the server runs without per-user
    /// auth, and the web falls back to `TL_API_KEY` + header
    /// forwarding for identity.
    pub jwt_signer: Option<Arc<crate::jwt::JwtSigner>>,
    /// Channel into the escalation webhook worker. `None` when no
    /// `TL_ESCALATION_WEBHOOK_URL` is configured — Escalate decisions
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
}

#[derive(Default)]
pub struct BuildOptions {
    /// Path to the policy YAML directory. Defaults to `TL_POLICY_DIR`
    /// env var, then `./policies`.
    pub policy_dir: Option<String>,
    /// Path to the LlmRouter TOML config. Defaults to `TL_LLM_CONFIG`
    /// env var, then `./config/llm-routing.toml`. If the file is
    /// missing the router boots empty (Tier 3 → Skipped).
    pub llm_config_path: Option<String>,
    /// Postgres connection string. Defaults to `DATABASE_URL`. When
    /// unset, the server runs memory-only.
    #[cfg(feature = "postgres")]
    pub database_url: Option<String>,
}
