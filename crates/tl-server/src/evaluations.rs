//! Agent-scoped post-run evaluation control plane and result access.

mod handlers;
mod memory_store;
#[cfg(feature = "postgres")]
mod worker;

use std::sync::Arc;

use async_trait::async_trait;
use tl_core::{
    AgentEvaluationPolicyAssignment, AgentEvaluationProfile, EvaluationJobSummary,
    EvaluationResultDetail, PutAgentEvaluationProfileRequest, RunEvaluationPolicyManifestSummary,
    RunParticipantRole, RunParticipantSummary,
};

pub use handlers::{
    __path_get_agent_evaluation_profile, __path_list_agent_evaluation_assignments,
    __path_list_run_evaluations, __path_put_agent_evaluation_assignments,
    __path_put_agent_evaluation_profile, __path_reevaluate_run, get_agent_evaluation_profile,
    list_agent_evaluation_assignments, list_run_evaluations, put_agent_evaluation_assignments,
    put_agent_evaluation_profile, reevaluate_run,
};
pub use memory_store::MemoryEvaluationStore;
#[cfg(feature = "postgres")]
pub use worker::{spawn_evaluation_worker, EvaluationWorkerConfig};

#[derive(Debug, thiserror::Error)]
pub enum EvaluationStoreError {
    #[error("not found")]
    NotFound,
    #[error("conflict")]
    Conflict,
    #[error("validation: {0}")]
    Validation(String),
    #[error("internal: {0}")]
    Internal(String),
}

#[async_trait]
pub trait EvaluationStore: Send + Sync {
    async fn get_profile(
        &self,
        workspace_id: &str,
        environment_id: &str,
        agent_id: &str,
    ) -> Result<Option<AgentEvaluationProfile>, EvaluationStoreError>;

    async fn put_profile(
        &self,
        workspace_id: &str,
        environment_id: &str,
        agent_id: &str,
        input: PutAgentEvaluationProfileRequest,
    ) -> Result<AgentEvaluationProfile, EvaluationStoreError>;

    async fn list_assignments(
        &self,
        workspace_id: &str,
        environment_id: &str,
        agent_id: &str,
    ) -> Result<Vec<AgentEvaluationPolicyAssignment>, EvaluationStoreError>;

    async fn replace_assignments(
        &self,
        workspace_id: &str,
        environment_id: &str,
        agent_id: &str,
        assignments: Vec<AgentEvaluationPolicyAssignment>,
    ) -> Result<Vec<AgentEvaluationPolicyAssignment>, EvaluationStoreError>;

    async fn ensure_assignment(
        &self,
        workspace_id: &str,
        environment_id: &str,
        agent_id: &str,
        assignment: AgentEvaluationPolicyAssignment,
    ) -> Result<Vec<AgentEvaluationPolicyAssignment>, EvaluationStoreError>;

    async fn register_participant_and_freeze_manifest(
        &self,
        workspace_id: &str,
        environment_id: &str,
        run_id: &str,
        agent_id: &str,
        role: RunParticipantRole,
    ) -> Result<(), EvaluationStoreError>;

    async fn list_participants(
        &self,
        workspace_id: &str,
        run_id: &str,
    ) -> Result<Vec<RunParticipantSummary>, EvaluationStoreError>;

    async fn list_manifest(
        &self,
        workspace_id: &str,
        run_id: &str,
        agent_id: Option<&str>,
    ) -> Result<Vec<RunEvaluationPolicyManifestSummary>, EvaluationStoreError>;

    async fn list_results(
        &self,
        workspace_id: &str,
        environment_id: &str,
        run_id: &str,
    ) -> Result<Vec<EvaluationResultDetail>, EvaluationStoreError>;

    async fn list_jobs(
        &self,
        workspace_id: &str,
        environment_id: &str,
        run_id: &str,
    ) -> Result<Vec<EvaluationJobSummary>, EvaluationStoreError>;

    async fn request_reevaluation(
        &self,
        workspace_id: &str,
        environment_id: &str,
        run_id: &str,
        agent_ids: Option<Vec<String>>,
    ) -> Result<(), EvaluationStoreError>;
}

#[derive(Clone)]
pub struct EvaluationState {
    pub store: Arc<dyn EvaluationStore>,
    pub environment_store: Arc<dyn crate::environments::EnvironmentStore>,
    pub team_store: Arc<dyn crate::team::TeamStore>,
}

pub(crate) fn evaluation_error_response(error: EvaluationStoreError) -> axum::response::Response {
    use axum::response::IntoResponse;
    let (status, code, message) = match error {
        EvaluationStoreError::NotFound => (
            axum::http::StatusCode::NOT_FOUND,
            tl_core::ApiErrorCode::NotFound,
            "evaluation resource not found".to_string(),
        ),
        EvaluationStoreError::Conflict => (
            axum::http::StatusCode::CONFLICT,
            tl_core::ApiErrorCode::Conflict,
            "evaluation configuration conflict".to_string(),
        ),
        EvaluationStoreError::Validation(message) => (
            axum::http::StatusCode::BAD_REQUEST,
            tl_core::ApiErrorCode::Invalid,
            message,
        ),
        EvaluationStoreError::Internal(message) => {
            tracing::error!(error = %message, "evaluation request failed");
            (
                axum::http::StatusCode::INTERNAL_SERVER_ERROR,
                tl_core::ApiErrorCode::Internal,
                "evaluation request failed".to_string(),
            )
        }
    };
    (
        status,
        axum::Json(tl_core::ApiError {
            code,
            message,
            retriable: code.default_retriable(),
            details: serde_json::Value::Null,
        }),
    )
        .into_response()
}
