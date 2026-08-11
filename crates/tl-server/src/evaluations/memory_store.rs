use std::collections::{HashMap, HashSet};

use async_trait::async_trait;
use tl_core::{
    AgentEvaluationPolicyAssignment, AgentEvaluationProfile, EvaluationJobSummary,
    EvaluationResultDetail, PolicyFamily, PutAgentEvaluationProfileRequest,
    RunEvaluationPolicyManifestSummary, RunParticipantRole, RunParticipantSummary,
};
use tokio::sync::RwLock;

use super::{EvaluationStore, EvaluationStoreError};

type ProfileKey = (String, String, String);
type RunKey = (String, String);
type ParticipantKey = (String, String, String);

#[derive(Debug, Default)]
pub struct MemoryEvaluationStore {
    profiles: RwLock<HashMap<ProfileKey, AgentEvaluationProfile>>,
    assignments: RwLock<HashMap<ProfileKey, Vec<AgentEvaluationPolicyAssignment>>>,
    participants: RwLock<HashMap<RunKey, Vec<RunParticipantSummary>>>,
    frozen_participants: RwLock<HashSet<ParticipantKey>>,
    manifests: RwLock<HashMap<RunKey, Vec<RunEvaluationPolicyManifestSummary>>>,
}

impl MemoryEvaluationStore {
    pub fn new() -> Self {
        Self::default()
    }
}

#[async_trait]
impl EvaluationStore for MemoryEvaluationStore {
    async fn get_profile(
        &self,
        workspace_id: &str,
        environment_id: &str,
        agent_id: &str,
    ) -> Result<Option<AgentEvaluationProfile>, EvaluationStoreError> {
        Ok(self
            .profiles
            .read()
            .await
            .get(&profile_key(workspace_id, environment_id, agent_id))
            .cloned())
    }

    async fn put_profile(
        &self,
        workspace_id: &str,
        environment_id: &str,
        agent_id: &str,
        input: PutAgentEvaluationProfileRequest,
    ) -> Result<AgentEvaluationProfile, EvaluationStoreError> {
        if input.quiet_period_ms > 300_000
            || !(1_000..=3_600_000).contains(&input.max_capture_wait_ms)
        {
            return Err(EvaluationStoreError::Validation(
                "evaluation capture timing is outside allowed bounds".into(),
            ));
        }
        let key = profile_key(workspace_id, environment_id, agent_id);
        let mut profiles = self.profiles.write().await;
        let current_version = profiles
            .get(&key)
            .map_or(0, |profile| profile.profile_version);
        if input
            .expected_profile_version
            .is_some_and(|expected| expected != current_version)
        {
            return Err(EvaluationStoreError::Conflict);
        }
        let profile = AgentEvaluationProfile {
            workspace_id: workspace_id.to_string(),
            environment_id: environment_id.to_string(),
            agent_id: agent_id.to_string(),
            enabled: input.enabled,
            capture_mode: input.capture_mode,
            content_mode: input.content_mode,
            quiet_period_ms: input.quiet_period_ms,
            max_capture_wait_ms: input.max_capture_wait_ms,
            on_incomplete: input.on_incomplete,
            profile_version: current_version + 1,
            updated_at: chrono::Utc::now().to_rfc3339(),
        };
        profiles.insert(key, profile.clone());
        Ok(profile)
    }

    async fn list_assignments(
        &self,
        workspace_id: &str,
        environment_id: &str,
        agent_id: &str,
    ) -> Result<Vec<AgentEvaluationPolicyAssignment>, EvaluationStoreError> {
        Ok(self
            .assignments
            .read()
            .await
            .get(&profile_key(workspace_id, environment_id, agent_id))
            .cloned()
            .unwrap_or_default())
    }

    async fn replace_assignments(
        &self,
        workspace_id: &str,
        environment_id: &str,
        agent_id: &str,
        assignments: Vec<AgentEvaluationPolicyAssignment>,
    ) -> Result<Vec<AgentEvaluationPolicyAssignment>, EvaluationStoreError> {
        if assignments.len() > 256 {
            return Err(EvaluationStoreError::Validation(
                "at most 256 evaluation assignments are allowed".into(),
            ));
        }
        let key = profile_key(workspace_id, environment_id, agent_id);
        if !self.profiles.read().await.contains_key(&key) {
            return Err(EvaluationStoreError::NotFound);
        }
        let mut seen = std::collections::HashSet::new();
        for assignment in &assignments {
            if assignment.policy_id.trim().is_empty()
                || !seen.insert(assignment.policy_id.trim().to_string())
                || !(1..=10_000).contains(&assignment.weight)
            {
                return Err(EvaluationStoreError::Validation(
                    "evaluation assignments must be unique and have valid weights".into(),
                ));
            }
        }
        self.assignments
            .write()
            .await
            .insert(key, assignments.clone());
        Ok(assignments)
    }

    async fn ensure_assignment(
        &self,
        workspace_id: &str,
        environment_id: &str,
        agent_id: &str,
        assignment: AgentEvaluationPolicyAssignment,
    ) -> Result<Vec<AgentEvaluationPolicyAssignment>, EvaluationStoreError> {
        if assignment.policy_id.trim().is_empty() || !(1..=10_000).contains(&assignment.weight) {
            return Err(EvaluationStoreError::Validation(
                "evaluation assignment must have a policy id and valid weight".into(),
            ));
        }
        let key = profile_key(workspace_id, environment_id, agent_id);
        if !self.profiles.read().await.contains_key(&key) {
            return Err(EvaluationStoreError::NotFound);
        }
        let mut assignments = self.assignments.write().await;
        let rows = assignments.entry(key).or_default();
        if !rows
            .iter()
            .any(|existing| existing.policy_id == assignment.policy_id)
        {
            if rows.len() >= 256 {
                return Err(EvaluationStoreError::Validation(
                    "at most 256 evaluation assignments are allowed".into(),
                ));
            }
            rows.push(assignment);
            rows.sort_by(|left, right| left.policy_id.cmp(&right.policy_id));
        }
        Ok(rows.clone())
    }

    async fn register_participant_and_freeze_manifest(
        &self,
        workspace_id: &str,
        environment_id: &str,
        run_id: &str,
        agent_id: &str,
        role: RunParticipantRole,
    ) -> Result<(), EvaluationStoreError> {
        let run_key = (workspace_id.to_string(), run_id.to_string());
        let mut participants = self.participants.write().await;
        let rows = participants.entry(run_key.clone()).or_default();
        if !rows.iter().any(|row| row.agent_id == agent_id) {
            rows.push(RunParticipantSummary {
                agent_id: agent_id.to_string(),
                role,
                joined_at: chrono::Utc::now().to_rfc3339(),
            });
        }
        drop(participants);

        if !self.frozen_participants.write().await.insert((
            workspace_id.to_string(),
            run_id.to_string(),
            agent_id.to_string(),
        )) {
            return Ok(());
        }

        let key = profile_key(workspace_id, environment_id, agent_id);
        if !self
            .profiles
            .read()
            .await
            .get(&key)
            .is_some_and(|profile| profile.enabled)
        {
            return Ok(());
        }
        let assignments = self
            .assignments
            .read()
            .await
            .get(&key)
            .cloned()
            .unwrap_or_default();
        let mut manifests = self.manifests.write().await;
        let rows = manifests.entry(run_key).or_default();
        for assignment in assignments.into_iter().filter(|item| item.enabled) {
            if rows
                .iter()
                .any(|item| item.agent_id == agent_id && item.policy_id == assignment.policy_id)
            {
                continue;
            }
            let version = assignment.policy_version.unwrap_or(1);
            rows.push(RunEvaluationPolicyManifestSummary {
                agent_id: agent_id.to_string(),
                policy_id: assignment.policy_id.clone(),
                policy_family: PolicyFamily::Evaluation,
                policy_version: version,
                policy_hash: format!("memory:{}:{version}", assignment.policy_id),
                weight: assignment.weight,
                critical: assignment.critical,
            });
        }
        Ok(())
    }

    async fn list_participants(
        &self,
        workspace_id: &str,
        run_id: &str,
    ) -> Result<Vec<RunParticipantSummary>, EvaluationStoreError> {
        Ok(self
            .participants
            .read()
            .await
            .get(&(workspace_id.to_string(), run_id.to_string()))
            .cloned()
            .unwrap_or_default())
    }

    async fn list_manifest(
        &self,
        workspace_id: &str,
        run_id: &str,
        agent_id: Option<&str>,
    ) -> Result<Vec<RunEvaluationPolicyManifestSummary>, EvaluationStoreError> {
        let mut rows = self
            .manifests
            .read()
            .await
            .get(&(workspace_id.to_string(), run_id.to_string()))
            .cloned()
            .unwrap_or_default();
        if let Some(agent_id) = agent_id {
            rows.retain(|row| row.agent_id == agent_id);
        }
        Ok(rows)
    }

    async fn list_results(
        &self,
        _workspace_id: &str,
        _environment_id: &str,
        _run_id: &str,
    ) -> Result<Vec<EvaluationResultDetail>, EvaluationStoreError> {
        Ok(Vec::new())
    }

    async fn list_jobs(
        &self,
        _workspace_id: &str,
        _environment_id: &str,
        _run_id: &str,
    ) -> Result<Vec<EvaluationJobSummary>, EvaluationStoreError> {
        Ok(Vec::new())
    }

    async fn request_reevaluation(
        &self,
        workspace_id: &str,
        _environment_id: &str,
        run_id: &str,
        agent_ids: Option<Vec<String>>,
    ) -> Result<(), EvaluationStoreError> {
        let participants = self
            .participants
            .read()
            .await
            .get(&(workspace_id.to_string(), run_id.to_string()))
            .cloned()
            .ok_or(EvaluationStoreError::NotFound)?;
        if agent_ids.as_ref().is_some_and(|requested| {
            requested.is_empty()
                || requested
                    .iter()
                    .any(|id| !participants.iter().any(|row| row.agent_id == *id))
        }) {
            return Err(EvaluationStoreError::NotFound);
        }
        Ok(())
    }
}

fn profile_key(workspace_id: &str, environment_id: &str, agent_id: &str) -> ProfileKey {
    (
        workspace_id.to_string(),
        environment_id.to_string(),
        agent_id.to_string(),
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use tl_core::{
        CaptureMode, ContentCaptureMode, MissingEvidenceBehavior, PutAgentEvaluationProfileRequest,
    };

    fn profile(enabled: bool) -> PutAgentEvaluationProfileRequest {
        PutAgentEvaluationProfileRequest {
            enabled,
            capture_mode: CaptureMode::BestEffort,
            content_mode: ContentCaptureMode::MetadataOnly,
            quiet_period_ms: 2_000,
            max_capture_wait_ms: 30_000,
            on_incomplete: MissingEvidenceBehavior::Inconclusive,
            expected_profile_version: None,
        }
    }

    fn assignment(policy_id: &str) -> AgentEvaluationPolicyAssignment {
        AgentEvaluationPolicyAssignment {
            policy_id: policy_id.into(),
            policy_version: Some(1),
            weight: 1,
            critical: false,
            enabled: true,
        }
    }

    #[tokio::test]
    async fn manifest_does_not_change_after_first_participation() {
        let store = MemoryEvaluationStore::new();
        store
            .put_profile("ws", "env", "agent", profile(true))
            .await
            .unwrap();
        store
            .replace_assignments("ws", "env", "agent", vec![assignment("policy-a")])
            .await
            .unwrap();
        store
            .register_participant_and_freeze_manifest(
                "ws",
                "env",
                "run",
                "agent",
                RunParticipantRole::Primary,
            )
            .await
            .unwrap();

        store
            .replace_assignments(
                "ws",
                "env",
                "agent",
                vec![assignment("policy-a"), assignment("policy-b")],
            )
            .await
            .unwrap();
        store
            .register_participant_and_freeze_manifest(
                "ws",
                "env",
                "run",
                "agent",
                RunParticipantRole::Primary,
            )
            .await
            .unwrap();

        let manifest = store
            .list_manifest("ws", "run", Some("agent"))
            .await
            .unwrap();
        assert_eq!(manifest.len(), 1);
        assert_eq!(manifest[0].policy_id, "policy-a");
    }

    #[tokio::test]
    async fn empty_first_manifest_stays_empty_after_configuration_changes() {
        let store = MemoryEvaluationStore::new();
        store
            .put_profile("ws", "env", "agent", profile(false))
            .await
            .unwrap();
        store
            .register_participant_and_freeze_manifest(
                "ws",
                "env",
                "run",
                "agent",
                RunParticipantRole::Primary,
            )
            .await
            .unwrap();

        store
            .put_profile("ws", "env", "agent", profile(true))
            .await
            .unwrap();
        store
            .replace_assignments("ws", "env", "agent", vec![assignment("policy-a")])
            .await
            .unwrap();
        store
            .register_participant_and_freeze_manifest(
                "ws",
                "env",
                "run",
                "agent",
                RunParticipantRole::Primary,
            )
            .await
            .unwrap();

        assert!(store
            .list_manifest("ws", "run", Some("agent"))
            .await
            .unwrap()
            .is_empty());
    }
}
