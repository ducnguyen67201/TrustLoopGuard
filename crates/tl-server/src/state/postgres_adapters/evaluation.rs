use std::sync::Arc;

use async_trait::async_trait;
use tl_storage::EvaluationRepo;

use crate::evaluations::{EvaluationStore, EvaluationStoreError};

pub struct PostgresEvaluationAdapter(Arc<EvaluationRepo>);

impl PostgresEvaluationAdapter {
    pub fn new(repo: Arc<EvaluationRepo>) -> Arc<Self> {
        Arc::new(Self(repo))
    }
}

#[async_trait]
impl EvaluationStore for PostgresEvaluationAdapter {
    async fn get_profile(
        &self,
        workspace_id: &str,
        environment_id: &str,
        agent_id: &str,
    ) -> Result<Option<tl_core::AgentEvaluationProfile>, EvaluationStoreError> {
        self.0
            .get_profile(workspace_id, environment_id, agent_id)
            .await
            .map_err(map_error)
    }

    async fn put_profile(
        &self,
        workspace_id: &str,
        environment_id: &str,
        agent_id: &str,
        input: tl_core::PutAgentEvaluationProfileRequest,
    ) -> Result<tl_core::AgentEvaluationProfile, EvaluationStoreError> {
        self.0
            .put_profile(workspace_id, environment_id, agent_id, input)
            .await
            .map_err(map_error)
    }

    async fn list_assignments(
        &self,
        workspace_id: &str,
        environment_id: &str,
        agent_id: &str,
    ) -> Result<Vec<tl_core::AgentEvaluationPolicyAssignment>, EvaluationStoreError> {
        self.0
            .list_assignments(workspace_id, environment_id, agent_id)
            .await
            .map_err(map_error)
    }

    async fn replace_assignments(
        &self,
        workspace_id: &str,
        environment_id: &str,
        agent_id: &str,
        assignments: Vec<tl_core::AgentEvaluationPolicyAssignment>,
    ) -> Result<Vec<tl_core::AgentEvaluationPolicyAssignment>, EvaluationStoreError> {
        self.0
            .replace_assignments(workspace_id, environment_id, agent_id, assignments)
            .await
            .map_err(map_error)
    }

    async fn ensure_assignment(
        &self,
        workspace_id: &str,
        environment_id: &str,
        agent_id: &str,
        assignment: tl_core::AgentEvaluationPolicyAssignment,
    ) -> Result<Vec<tl_core::AgentEvaluationPolicyAssignment>, EvaluationStoreError> {
        self.0
            .ensure_assignment(workspace_id, environment_id, agent_id, assignment)
            .await
            .map_err(map_error)
    }

    async fn register_participant_and_freeze_manifest(
        &self,
        workspace_id: &str,
        environment_id: &str,
        run_id: &str,
        agent_id: &str,
        role: tl_core::RunParticipantRole,
    ) -> Result<(), EvaluationStoreError> {
        self.0
            .register_participant_and_freeze_manifest(
                workspace_id,
                environment_id,
                run_id,
                agent_id,
                role,
            )
            .await
            .map_err(map_error)
    }

    async fn list_participants(
        &self,
        workspace_id: &str,
        run_id: &str,
    ) -> Result<Vec<tl_core::RunParticipantSummary>, EvaluationStoreError> {
        self.0
            .list_participants(workspace_id, run_id)
            .await
            .map_err(map_error)
    }

    async fn list_manifest(
        &self,
        workspace_id: &str,
        run_id: &str,
        agent_id: Option<&str>,
    ) -> Result<Vec<tl_core::RunEvaluationPolicyManifestSummary>, EvaluationStoreError> {
        self.0
            .list_manifest(workspace_id, run_id, agent_id)
            .await
            .map_err(map_error)
    }

    async fn list_results(
        &self,
        workspace_id: &str,
        environment_id: &str,
        run_id: &str,
    ) -> Result<Vec<tl_core::EvaluationResultDetail>, EvaluationStoreError> {
        self.0
            .list_results(workspace_id, environment_id, run_id)
            .await
            .map_err(map_error)
    }

    async fn list_jobs(
        &self,
        workspace_id: &str,
        environment_id: &str,
        run_id: &str,
    ) -> Result<Vec<tl_core::EvaluationJobSummary>, EvaluationStoreError> {
        let run_id = uuid::Uuid::parse_str(run_id)
            .map_err(|_| EvaluationStoreError::Validation("invalid run id".into()))?;
        self.0
            .list_evaluation_jobs(workspace_id, environment_id, run_id)
            .await
            .map_err(map_error)
    }

    async fn request_reevaluation(
        &self,
        workspace_id: &str,
        environment_id: &str,
        run_id: &str,
        agent_ids: Option<Vec<String>>,
    ) -> Result<(), EvaluationStoreError> {
        let run_id = uuid::Uuid::parse_str(run_id)
            .map_err(|_| EvaluationStoreError::Validation("invalid run id".into()))?;
        self.0
            .request_reevaluation(workspace_id, environment_id, run_id, agent_ids.as_deref())
            .await
            .map_err(map_error)
    }
}

fn map_error(error: tl_storage::StorageError) -> EvaluationStoreError {
    match error {
        tl_storage::StorageError::NotFound => EvaluationStoreError::NotFound,
        tl_storage::StorageError::Conflict => EvaluationStoreError::Conflict,
        tl_storage::StorageError::Internal(message)
            if message.contains("must") || message.contains("allowed") =>
        {
            EvaluationStoreError::Validation(message)
        }
        tl_storage::StorageError::Internal(message) => EvaluationStoreError::Internal(message),
    }
}
