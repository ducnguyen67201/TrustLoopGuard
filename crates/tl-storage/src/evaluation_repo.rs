//! Durable agent evaluation configuration, frozen manifests, snapshots, jobs,
//! and results.

use std::collections::HashSet;

use diesel::dsl::{max, now};
use diesel::prelude::*;
use diesel::upsert::excluded;
use diesel_async::{AsyncConnection, RunQueryDsl};
use sha2::{Digest, Sha256};
use tl_core::{
    AgentEvaluationPolicyAssignment, AgentEvaluationProfile, CaptureMode, ContentCaptureMode,
    EvaluationFinding, EvaluationFindingStatus, EvaluationResultDetail, EvaluationResultSummary,
    EvaluationVerdict, MissingEvidenceBehavior, PolicyFamily, PutAgentEvaluationProfileRequest,
    RunCaptureStatus, RunEvaluationPolicyManifestSummary, RunParticipantRole,
    RunParticipantSummary, Severity,
};
use tl_policy::{AnyPolicy, FamilyPolicy};
use uuid::Uuid;

use crate::models::{
    AgentEvaluationPolicyAssignmentRecord, AgentEvaluationProfileRecord, EvaluationFindingRecord,
    EvaluationResultRecord, RunEvaluationPolicyManifestRecord, RunParticipantRecord,
};
use crate::postgres::{DbConnection, DbPool};
use crate::schema::{
    agent_evaluation_policy_assignments, agent_evaluation_profiles, agents, entity_versions,
    evaluation_findings, evaluation_results, policies, run_evaluation_policy_manifest,
    run_participants, runs, workspace_environments,
};
use crate::StorageError;

#[derive(Clone)]
pub struct EvaluationRepo {
    pub(crate) pool: DbPool,
}

#[derive(Debug, Insertable)]
#[diesel(table_name = agent_evaluation_policy_assignments)]
struct NewAssignment {
    workspace_id: String,
    environment_id: String,
    agent_id: String,
    policy_id: String,
    policy_version: Option<i32>,
    weight: i32,
    critical: bool,
    enabled: bool,
}

#[derive(Debug, Insertable)]
#[diesel(table_name = run_evaluation_policy_manifest)]
struct NewManifestEntry {
    workspace_id: String,
    environment_id: String,
    run_id: Uuid,
    agent_id: String,
    policy_id: String,
    policy_family: String,
    policy_version: i32,
    policy_hash: String,
    policy_yaml: String,
    weight: i32,
    critical: bool,
    evidence_requirements: serde_json::Value,
}

impl EvaluationRepo {
    pub fn new(pool: DbPool) -> Self {
        Self { pool }
    }

    pub fn pool(&self) -> &DbPool {
        &self.pool
    }

    pub async fn get_profile(
        &self,
        workspace_id: &str,
        environment_id: &str,
        agent_id: &str,
    ) -> Result<Option<AgentEvaluationProfile>, StorageError> {
        let mut conn = self.connection().await?;
        agent_evaluation_profiles::table
            .filter(agent_evaluation_profiles::workspace_id.eq(workspace_id))
            .filter(agent_evaluation_profiles::environment_id.eq(environment_id))
            .filter(agent_evaluation_profiles::agent_id.eq(agent_id))
            .select(AgentEvaluationProfileRecord::as_select())
            .first::<AgentEvaluationProfileRecord>(&mut conn)
            .await
            .optional()
            .map_err(|error| StorageError::Internal(format!("evaluation profile get: {error}")))?
            .map(profile_from_record)
            .transpose()
    }

    pub async fn put_profile(
        &self,
        workspace_id: &str,
        environment_id: &str,
        agent_id: &str,
        input: PutAgentEvaluationProfileRequest,
    ) -> Result<AgentEvaluationProfile, StorageError> {
        validate_profile_input(&input)?;
        let mut conn = self.connection().await?;
        conn.transaction::<(), StorageError, _>(async |conn| {
            require_agent_and_environment(conn, workspace_id, environment_id, agent_id).await?;
            let current_version = agent_evaluation_profiles::table
                .filter(agent_evaluation_profiles::workspace_id.eq(workspace_id))
                .filter(agent_evaluation_profiles::environment_id.eq(environment_id))
                .filter(agent_evaluation_profiles::agent_id.eq(agent_id))
                .select(agent_evaluation_profiles::profile_version)
                .first::<i32>(conn)
                .await
                .optional()?;
            if let Some(expected) = input.expected_profile_version {
                if current_version.unwrap_or(0) != expected {
                    return Err(StorageError::Conflict);
                }
            }
            let next_version = current_version.unwrap_or(0) + 1;
            diesel::insert_into(agent_evaluation_profiles::table)
                .values((
                    agent_evaluation_profiles::workspace_id.eq(workspace_id),
                    agent_evaluation_profiles::environment_id.eq(environment_id),
                    agent_evaluation_profiles::agent_id.eq(agent_id),
                    agent_evaluation_profiles::enabled.eq(input.enabled),
                    agent_evaluation_profiles::capture_mode
                        .eq(capture_mode_text(input.capture_mode)),
                    agent_evaluation_profiles::content_mode
                        .eq(content_mode_text(input.content_mode)),
                    agent_evaluation_profiles::quiet_period_ms.eq(input.quiet_period_ms as i64),
                    agent_evaluation_profiles::max_capture_wait_ms
                        .eq(input.max_capture_wait_ms as i64),
                    agent_evaluation_profiles::on_incomplete
                        .eq(missing_evidence_text(input.on_incomplete)),
                    agent_evaluation_profiles::profile_version.eq(next_version),
                ))
                .on_conflict((
                    agent_evaluation_profiles::workspace_id,
                    agent_evaluation_profiles::environment_id,
                    agent_evaluation_profiles::agent_id,
                ))
                .do_update()
                .set((
                    agent_evaluation_profiles::enabled
                        .eq(excluded(agent_evaluation_profiles::enabled)),
                    agent_evaluation_profiles::capture_mode
                        .eq(excluded(agent_evaluation_profiles::capture_mode)),
                    agent_evaluation_profiles::content_mode
                        .eq(excluded(agent_evaluation_profiles::content_mode)),
                    agent_evaluation_profiles::quiet_period_ms
                        .eq(excluded(agent_evaluation_profiles::quiet_period_ms)),
                    agent_evaluation_profiles::max_capture_wait_ms
                        .eq(excluded(agent_evaluation_profiles::max_capture_wait_ms)),
                    agent_evaluation_profiles::on_incomplete
                        .eq(excluded(agent_evaluation_profiles::on_incomplete)),
                    agent_evaluation_profiles::profile_version
                        .eq(excluded(agent_evaluation_profiles::profile_version)),
                    agent_evaluation_profiles::updated_at.eq(now),
                ))
                .execute(conn)
                .await?;
            Ok(())
        })
        .await?;
        self.get_profile(workspace_id, environment_id, agent_id)
            .await?
            .ok_or_else(|| {
                StorageError::Internal("evaluation profile disappeared after put".into())
            })
    }

    pub async fn list_assignments(
        &self,
        workspace_id: &str,
        environment_id: &str,
        agent_id: &str,
    ) -> Result<Vec<AgentEvaluationPolicyAssignment>, StorageError> {
        let mut conn = self.connection().await?;
        let records = agent_evaluation_policy_assignments::table
            .filter(agent_evaluation_policy_assignments::workspace_id.eq(workspace_id))
            .filter(agent_evaluation_policy_assignments::environment_id.eq(environment_id))
            .filter(agent_evaluation_policy_assignments::agent_id.eq(agent_id))
            .select(AgentEvaluationPolicyAssignmentRecord::as_select())
            .order(agent_evaluation_policy_assignments::policy_id.asc())
            .load::<AgentEvaluationPolicyAssignmentRecord>(&mut conn)
            .await
            .map_err(|error| {
                StorageError::Internal(format!("evaluation assignments list: {error}"))
            })?;
        records.into_iter().map(assignment_from_record).collect()
    }

    pub async fn replace_assignments(
        &self,
        workspace_id: &str,
        environment_id: &str,
        agent_id: &str,
        assignments: Vec<AgentEvaluationPolicyAssignment>,
    ) -> Result<Vec<AgentEvaluationPolicyAssignment>, StorageError> {
        validate_assignments(&assignments)?;
        let mut conn = self.connection().await?;
        conn.transaction::<(), StorageError, _>(async |conn| {
            require_agent_and_environment(conn, workspace_id, environment_id, agent_id).await?;
            let profile_exists = agent_evaluation_profiles::table
                .filter(agent_evaluation_profiles::workspace_id.eq(workspace_id))
                .filter(agent_evaluation_profiles::environment_id.eq(environment_id))
                .filter(agent_evaluation_profiles::agent_id.eq(agent_id))
                .select(agent_evaluation_profiles::agent_id)
                .first::<String>(conn)
                .await
                .optional()?;
            if profile_exists.is_none() {
                return Err(StorageError::NotFound);
            }

            for assignment in &assignments {
                let (_, policy_yaml) = resolve_evaluation_policy_version(
                    conn,
                    workspace_id,
                    &assignment.policy_id,
                    assignment.policy_version,
                )
                .await?;
                build_evidence_requirements(conn, workspace_id, &policy_yaml).await?;
            }
            diesel::delete(
                agent_evaluation_policy_assignments::table
                    .filter(agent_evaluation_policy_assignments::workspace_id.eq(workspace_id))
                    .filter(agent_evaluation_policy_assignments::environment_id.eq(environment_id))
                    .filter(agent_evaluation_policy_assignments::agent_id.eq(agent_id)),
            )
            .execute(conn)
            .await?;
            if !assignments.is_empty() {
                let rows = assignments
                    .iter()
                    .map(|assignment| NewAssignment {
                        workspace_id: workspace_id.to_string(),
                        environment_id: environment_id.to_string(),
                        agent_id: agent_id.to_string(),
                        policy_id: assignment.policy_id.trim().to_string(),
                        policy_version: assignment.policy_version,
                        weight: assignment.weight as i32,
                        critical: assignment.critical,
                        enabled: assignment.enabled,
                    })
                    .collect::<Vec<_>>();
                diesel::insert_into(agent_evaluation_policy_assignments::table)
                    .values(&rows)
                    .execute(conn)
                    .await?;
            }
            Ok(())
        })
        .await?;
        self.list_assignments(workspace_id, environment_id, agent_id)
            .await
    }

    pub async fn register_participant_and_freeze_manifest(
        &self,
        workspace_id: &str,
        environment_id: &str,
        run_id: &str,
        agent_id: &str,
        role: RunParticipantRole,
    ) -> Result<(), StorageError> {
        let run_id = Uuid::parse_str(run_id)
            .map_err(|error| StorageError::Internal(format!("run_id parse: {error}")))?;
        let mut conn = self.connection().await?;
        conn.transaction::<(), StorageError, _>(async |conn| {
            let run_environment = diesel::update(
                runs::table
                    .filter(runs::workspace_id.eq(workspace_id))
                    .filter(runs::id.eq(run_id))
                    .filter(runs::environment_id.eq(environment_id)),
            )
            .set(runs::updated_at.eq(runs::updated_at))
            .returning(runs::environment_id)
            .get_result::<String>(conn)
            .await
            .optional()?;
            if run_environment.is_none() {
                return Err(StorageError::NotFound);
            }
            require_agent_and_environment(conn, workspace_id, environment_id, agent_id).await?;
            diesel::insert_into(run_participants::table)
                .values((
                    run_participants::workspace_id.eq(workspace_id),
                    run_participants::environment_id.eq(environment_id),
                    run_participants::run_id.eq(run_id),
                    run_participants::agent_id.eq(agent_id),
                    run_participants::role.eq(participant_role_text(role)),
                ))
                .on_conflict((
                    run_participants::workspace_id,
                    run_participants::run_id,
                    run_participants::agent_id,
                ))
                .do_nothing()
                .execute(conn)
                .await?;

            let manifest_frozen_at = run_participants::table
                .filter(run_participants::workspace_id.eq(workspace_id))
                .filter(run_participants::run_id.eq(run_id))
                .filter(run_participants::agent_id.eq(agent_id))
                .select(run_participants::manifest_frozen_at)
                .for_update()
                .first::<Option<chrono::DateTime<chrono::Utc>>>(conn)
                .await?;
            if manifest_frozen_at.is_some() {
                return Ok(());
            }

            let enabled = agent_evaluation_profiles::table
                .filter(agent_evaluation_profiles::workspace_id.eq(workspace_id))
                .filter(agent_evaluation_profiles::environment_id.eq(environment_id))
                .filter(agent_evaluation_profiles::agent_id.eq(agent_id))
                .select(agent_evaluation_profiles::enabled)
                .first::<bool>(conn)
                .await
                .optional()?
                .unwrap_or(false);
            let assignments = if enabled {
                agent_evaluation_policy_assignments::table
                    .filter(agent_evaluation_policy_assignments::workspace_id.eq(workspace_id))
                    .filter(agent_evaluation_policy_assignments::environment_id.eq(environment_id))
                    .filter(agent_evaluation_policy_assignments::agent_id.eq(agent_id))
                    .filter(agent_evaluation_policy_assignments::enabled.eq(true))
                    .select(AgentEvaluationPolicyAssignmentRecord::as_select())
                    .load::<AgentEvaluationPolicyAssignmentRecord>(conn)
                    .await?
            } else {
                Vec::new()
            };
            let mut manifest = Vec::with_capacity(assignments.len());
            for assignment in assignments {
                let (version, yaml) = resolve_evaluation_policy_version(
                    conn,
                    workspace_id,
                    &assignment.policy_id,
                    assignment.policy_version,
                )
                .await?;
                let policy = tl_policy::load_any_str(&yaml).map_err(|error| {
                    StorageError::Internal(format!("evaluation policy version parse: {error}"))
                })?;
                if policy.family() != PolicyFamily::Evaluation {
                    return Err(StorageError::Internal(format!(
                        "assigned policy `{}` is not evaluation family",
                        assignment.policy_id
                    )));
                }
                let evidence_requirements =
                    build_evidence_requirements(conn, workspace_id, &yaml).await?;
                manifest.push(NewManifestEntry {
                    workspace_id: workspace_id.to_string(),
                    environment_id: environment_id.to_string(),
                    run_id,
                    agent_id: agent_id.to_string(),
                    policy_id: assignment.policy_id,
                    policy_family: PolicyFamily::Evaluation.as_str().to_string(),
                    policy_version: version,
                    policy_hash: hash_text(&yaml),
                    policy_yaml: yaml,
                    weight: assignment.weight,
                    critical: assignment.critical,
                    evidence_requirements,
                });
            }
            if !manifest.is_empty() {
                diesel::insert_into(run_evaluation_policy_manifest::table)
                    .values(&manifest)
                    .on_conflict_do_nothing()
                    .execute(conn)
                    .await?;
            }
            diesel::update(
                run_participants::table
                    .filter(run_participants::workspace_id.eq(workspace_id))
                    .filter(run_participants::run_id.eq(run_id))
                    .filter(run_participants::agent_id.eq(agent_id)),
            )
            .set(run_participants::manifest_frozen_at.eq(now))
            .execute(conn)
            .await?;
            Ok(())
        })
        .await
    }

    pub async fn list_participants(
        &self,
        workspace_id: &str,
        run_id: &str,
    ) -> Result<Vec<RunParticipantSummary>, StorageError> {
        let run_id = parse_uuid(run_id)?;
        let mut conn = self.connection().await?;
        let records = run_participants::table
            .filter(run_participants::workspace_id.eq(workspace_id))
            .filter(run_participants::run_id.eq(run_id))
            .select(RunParticipantRecord::as_select())
            .order((
                run_participants::joined_at.asc(),
                run_participants::agent_id.asc(),
            ))
            .load::<RunParticipantRecord>(&mut conn)
            .await?;
        records.into_iter().map(participant_from_record).collect()
    }

    pub async fn list_manifest(
        &self,
        workspace_id: &str,
        run_id: &str,
        agent_id: Option<&str>,
    ) -> Result<Vec<RunEvaluationPolicyManifestSummary>, StorageError> {
        let run_id = parse_uuid(run_id)?;
        let mut conn = self.connection().await?;
        let mut query = run_evaluation_policy_manifest::table
            .filter(run_evaluation_policy_manifest::workspace_id.eq(workspace_id))
            .filter(run_evaluation_policy_manifest::run_id.eq(run_id))
            .into_boxed();
        if let Some(agent_id) = agent_id {
            query = query.filter(run_evaluation_policy_manifest::agent_id.eq(agent_id));
        }
        let records = query
            .select(RunEvaluationPolicyManifestRecord::as_select())
            .order((
                run_evaluation_policy_manifest::agent_id.asc(),
                run_evaluation_policy_manifest::policy_id.asc(),
            ))
            .load::<RunEvaluationPolicyManifestRecord>(&mut conn)
            .await?;
        records.into_iter().map(manifest_from_record).collect()
    }

    pub async fn list_results(
        &self,
        workspace_id: &str,
        environment_id: &str,
        run_id: &str,
    ) -> Result<Vec<EvaluationResultDetail>, StorageError> {
        let run_id = parse_uuid(run_id)?;
        let mut conn = self.connection().await?;
        let results = evaluation_results::table
            .filter(evaluation_results::workspace_id.eq(workspace_id))
            .filter(evaluation_results::environment_id.eq(environment_id))
            .filter(evaluation_results::run_id.eq(run_id))
            .select(EvaluationResultRecord::as_select())
            .order(evaluation_results::created_at.desc())
            .load::<EvaluationResultRecord>(&mut conn)
            .await?;
        let mut details = Vec::with_capacity(results.len());
        for result in results {
            let findings = evaluation_findings::table
                .filter(evaluation_findings::workspace_id.eq(workspace_id))
                .filter(evaluation_findings::result_id.eq(result.id))
                .select(EvaluationFindingRecord::as_select())
                .order(evaluation_findings::policy_id.asc())
                .load::<EvaluationFindingRecord>(&mut conn)
                .await?;
            details.push(EvaluationResultDetail {
                result: result_from_record(result)?,
                findings: findings
                    .into_iter()
                    .map(finding_from_record)
                    .collect::<Result<Vec<_>, _>>()?,
            });
        }
        Ok(details)
    }

    pub(crate) async fn connection(&self) -> Result<DbConnection<'_>, StorageError> {
        self.pool
            .get()
            .await
            .map_err(|error| StorageError::Internal(format!("db pool: {error}")))
    }
}

async fn require_agent_and_environment(
    conn: &mut DbConnection<'_>,
    workspace_id: &str,
    environment_id: &str,
    agent_id: &str,
) -> Result<(), StorageError> {
    let agent = agents::table
        .filter(agents::workspace_id.eq(workspace_id))
        .filter(agents::id.eq(agent_id))
        .filter(agents::deleted_at.is_null())
        .select(agents::id)
        // Serialize evaluation-profile creation and replacement on the
        // durable agent row, including the initial version-0 -> version-1
        // transition where no profile row exists yet.
        .for_update()
        .first::<String>(conn)
        .await
        .optional()?;
    let environment = workspace_environments::table
        .filter(workspace_environments::workspace_id.eq(workspace_id))
        .filter(workspace_environments::id.eq(environment_id))
        .filter(workspace_environments::deleted_at.is_null())
        .select(workspace_environments::id)
        .first::<String>(conn)
        .await
        .optional()?;
    if agent.is_none() || environment.is_none() {
        return Err(StorageError::NotFound);
    }
    Ok(())
}

async fn require_evaluation_policy_version(
    conn: &mut DbConnection<'_>,
    workspace_id: &str,
    policy_id: &str,
    policy_version: Option<i32>,
) -> Result<(), StorageError> {
    let family = policies::table
        .filter(policies::workspace_id.eq(workspace_id))
        .filter(policies::id.eq(policy_id))
        .filter(policies::deleted_at.is_null())
        .select(policies::family)
        .first::<Option<String>>(conn)
        .await
        .optional()?;
    if family.flatten().as_deref() != Some(PolicyFamily::Evaluation.as_str()) {
        return Err(StorageError::NotFound);
    }
    if let Some(version) = policy_version {
        entity_versions::table
            .filter(entity_versions::workspace_id.eq(workspace_id))
            .filter(entity_versions::entity_type.eq("policy"))
            .filter(entity_versions::entity_id.eq(policy_id))
            .filter(entity_versions::version.eq(version))
            .select(entity_versions::version)
            .first::<i32>(conn)
            .await
            .optional()?
            .ok_or(StorageError::NotFound)?;
    }
    Ok(())
}

async fn resolve_evaluation_policy_version(
    conn: &mut DbConnection<'_>,
    workspace_id: &str,
    policy_id: &str,
    requested: Option<i32>,
) -> Result<(i32, String), StorageError> {
    require_evaluation_policy_version(conn, workspace_id, policy_id, requested).await?;
    let version = match requested {
        Some(version) => version,
        None => entity_versions::table
            .filter(entity_versions::workspace_id.eq(workspace_id))
            .filter(entity_versions::entity_type.eq("policy"))
            .filter(entity_versions::entity_id.eq(policy_id))
            .select(max(entity_versions::version))
            .first::<Option<i32>>(conn)
            .await?
            .ok_or(StorageError::NotFound)?,
    };
    let yaml = entity_versions::table
        .filter(entity_versions::workspace_id.eq(workspace_id))
        .filter(entity_versions::entity_type.eq("policy"))
        .filter(entity_versions::entity_id.eq(policy_id))
        .filter(entity_versions::version.eq(version))
        .select(entity_versions::content)
        .first::<String>(conn)
        .await?;
    Ok((version, yaml))
}

async fn build_evidence_requirements(
    conn: &mut DbConnection<'_>,
    workspace_id: &str,
    evaluation_policy_yaml: &str,
) -> Result<serde_json::Value, StorageError> {
    let policy = tl_policy::load_any_str(evaluation_policy_yaml).map_err(|error| {
        StorageError::Internal(format!("evaluation policy version must parse: {error}"))
    })?;
    let AnyPolicy::Family(FamilyPolicy::Evaluation(policy)) = policy else {
        return Err(StorageError::Internal(
            "assigned policy must use family: evaluation".into(),
        ));
    };
    let replay_policies = match &policy.grader {
        tl_policy::family_ast::EvaluationGrader::PolicyReplay { policy_ids } => {
            let mut frozen = Vec::with_capacity(policy_ids.len());
            for policy_id in policy_ids {
                let (policy_version, policy_yaml) =
                    resolve_replay_policy_version(conn, workspace_id, policy_id).await?;
                let parsed = tl_policy::load_any_str(&policy_yaml).map_err(|error| {
                    StorageError::Internal(format!(
                        "policy_replay source `{policy_id}` must parse: {error}"
                    ))
                })?;
                match parsed {
                    AnyPolicy::Content(content) if !content.r#match.uses_semantic() => {}
                    AnyPolicy::Content(_) => {
                        return Err(StorageError::Internal(format!(
                            "policy_replay source `{policy_id}` must use deterministic literal or regex matchers"
                        )))
                    }
                    AnyPolicy::Family(FamilyPolicy::Evaluation(_)) => {
                        return Err(StorageError::Internal(format!(
                            "policy_replay source `{policy_id}` must not reference an evaluation policy"
                        )))
                    }
                    AnyPolicy::Family(_) => {
                        return Err(StorageError::Internal(format!(
                            "policy_replay source `{policy_id}` must be a deterministic content policy"
                        )))
                    }
                }
                frozen.push(serde_json::json!({
                    "policy_id": policy_id,
                    "policy_version": policy_version,
                    "policy_hash": hash_text(&policy_yaml),
                    "policy_yaml": policy_yaml,
                }));
            }
            frozen
        }
        _ => Vec::new(),
    };
    Ok(serde_json::json!({
        "scope": policy.scope,
        "on_missing_evidence": policy.on_missing_evidence,
        "grader": policy.grader,
        "replay_policies": replay_policies,
    }))
}

async fn resolve_replay_policy_version(
    conn: &mut DbConnection<'_>,
    workspace_id: &str,
    policy_id: &str,
) -> Result<(i32, String), StorageError> {
    policies::table
        .filter(policies::workspace_id.eq(workspace_id))
        .filter(policies::id.eq(policy_id))
        .filter(policies::deleted_at.is_null())
        .select(policies::id)
        .first::<String>(conn)
        .await
        .optional()?
        .ok_or(StorageError::NotFound)?;
    let version = entity_versions::table
        .filter(entity_versions::workspace_id.eq(workspace_id))
        .filter(entity_versions::entity_type.eq("policy"))
        .filter(entity_versions::entity_id.eq(policy_id))
        .select(max(entity_versions::version))
        .first::<Option<i32>>(conn)
        .await?
        .ok_or(StorageError::NotFound)?;
    let yaml = entity_versions::table
        .filter(entity_versions::workspace_id.eq(workspace_id))
        .filter(entity_versions::entity_type.eq("policy"))
        .filter(entity_versions::entity_id.eq(policy_id))
        .filter(entity_versions::version.eq(version))
        .select(entity_versions::content)
        .first::<String>(conn)
        .await?;
    Ok((version, yaml))
}

fn validate_profile_input(input: &PutAgentEvaluationProfileRequest) -> Result<(), StorageError> {
    if input.quiet_period_ms > 300_000 {
        return Err(StorageError::Internal(
            "quiet_period_ms must be at most 300000".into(),
        ));
    }
    if !(1_000..=3_600_000).contains(&input.max_capture_wait_ms) {
        return Err(StorageError::Internal(
            "max_capture_wait_ms must be between 1000 and 3600000".into(),
        ));
    }
    Ok(())
}

fn validate_assignments(
    assignments: &[AgentEvaluationPolicyAssignment],
) -> Result<(), StorageError> {
    if assignments.len() > 256 {
        return Err(StorageError::Internal(
            "at most 256 evaluation assignments are allowed".into(),
        ));
    }
    let mut ids = HashSet::new();
    for assignment in assignments {
        let id = assignment.policy_id.trim();
        if id.is_empty() || id.len() > 128 {
            return Err(StorageError::Internal(
                "evaluation policy_id must contain between 1 and 128 bytes".into(),
            ));
        }
        if !ids.insert(id) {
            return Err(StorageError::Conflict);
        }
        if !(1..=10_000).contains(&assignment.weight) {
            return Err(StorageError::Internal(
                "evaluation assignment weight must be between 1 and 10000".into(),
            ));
        }
        if assignment.policy_version.is_some_and(|value| value < 1) {
            return Err(StorageError::Internal(
                "evaluation policy_version must be positive".into(),
            ));
        }
    }
    Ok(())
}

fn profile_from_record(
    record: AgentEvaluationProfileRecord,
) -> Result<AgentEvaluationProfile, StorageError> {
    Ok(AgentEvaluationProfile {
        workspace_id: record.workspace_id,
        environment_id: record.environment_id,
        agent_id: record.agent_id,
        enabled: record.enabled,
        capture_mode: parse_capture_mode(&record.capture_mode)?,
        content_mode: parse_content_mode(&record.content_mode)?,
        quiet_period_ms: record.quiet_period_ms as u64,
        max_capture_wait_ms: record.max_capture_wait_ms as u64,
        on_incomplete: parse_missing_evidence(&record.on_incomplete)?,
        profile_version: record.profile_version,
        updated_at: record.updated_at.to_rfc3339(),
    })
}

fn assignment_from_record(
    record: AgentEvaluationPolicyAssignmentRecord,
) -> Result<AgentEvaluationPolicyAssignment, StorageError> {
    Ok(AgentEvaluationPolicyAssignment {
        policy_id: record.policy_id,
        policy_version: record.policy_version,
        weight: record.weight as u32,
        critical: record.critical,
        enabled: record.enabled,
    })
}

fn participant_from_record(
    record: RunParticipantRecord,
) -> Result<RunParticipantSummary, StorageError> {
    Ok(RunParticipantSummary {
        agent_id: record.agent_id,
        role: match record.role.as_str() {
            "primary" => RunParticipantRole::Primary,
            "participant" => RunParticipantRole::Participant,
            other => {
                return Err(StorageError::Internal(format!(
                    "unknown run participant role `{other}`"
                )))
            }
        },
        joined_at: record.joined_at.to_rfc3339(),
    })
}

fn manifest_from_record(
    record: RunEvaluationPolicyManifestRecord,
) -> Result<RunEvaluationPolicyManifestSummary, StorageError> {
    let policy_family = match record.policy_family.as_str() {
        "evaluation" => PolicyFamily::Evaluation,
        other => {
            return Err(StorageError::Internal(format!(
                "unknown manifest policy family `{other}`"
            )))
        }
    };
    Ok(RunEvaluationPolicyManifestSummary {
        agent_id: record.agent_id,
        policy_id: record.policy_id,
        policy_family,
        policy_version: record.policy_version,
        policy_hash: record.policy_hash,
        weight: record.weight as u32,
        critical: record.critical,
    })
}

fn result_from_record(
    record: EvaluationResultRecord,
) -> Result<EvaluationResultSummary, StorageError> {
    Ok(EvaluationResultSummary {
        id: record.id.to_string(),
        run_id: record.run_id.to_string(),
        agent_id: record.agent_id,
        snapshot_hash: record.snapshot_hash,
        manifest_hash: record.manifest_hash,
        evaluator_version: record.evaluator_version,
        verdict: match record.verdict.as_str() {
            "passed" => EvaluationVerdict::Passed,
            "failed" => EvaluationVerdict::Failed,
            "inconclusive" => EvaluationVerdict::Inconclusive,
            "error" => EvaluationVerdict::Error,
            "not_configured" => EvaluationVerdict::NotConfigured,
            other => {
                return Err(StorageError::Internal(format!(
                    "unknown evaluation verdict `{other}`"
                )))
            }
        },
        score_bps: record.score_bps.map(|value| value as u32),
        capture_status: parse_capture_status(&record.capture_status)?,
        llm_audit: record.llm_audit,
        created_at: record.created_at.to_rfc3339(),
    })
}

fn finding_from_record(record: EvaluationFindingRecord) -> Result<EvaluationFinding, StorageError> {
    Ok(EvaluationFinding {
        policy_id: record.policy_id,
        policy_version: record.policy_version,
        agent_id: record.agent_id,
        severity: match record.severity.as_str() {
            "low" => Severity::Low,
            "medium" => Severity::Medium,
            "high" => Severity::High,
            "critical" => Severity::Critical,
            other => {
                return Err(StorageError::Internal(format!(
                    "unknown evaluation severity `{other}`"
                )))
            }
        },
        critical: record.critical,
        status: match record.status.as_str() {
            "passed" => EvaluationFindingStatus::Passed,
            "failed" => EvaluationFindingStatus::Failed,
            "inconclusive" => EvaluationFindingStatus::Inconclusive,
            "error" => EvaluationFindingStatus::Error,
            "not_applicable" => EvaluationFindingStatus::NotApplicable,
            other => {
                return Err(StorageError::Internal(format!(
                    "unknown evaluation finding status `{other}`"
                )))
            }
        },
        score_bps: record.score_bps.map(|value| value as u32),
        reason: record.reason,
        evidence: serde_json::from_value(record.evidence).map_err(|error| {
            StorageError::Internal(format!("evaluation evidence decode: {error}"))
        })?,
    })
}

pub fn hash_text(value: &str) -> String {
    let digest = Sha256::digest(value.as_bytes());
    format!(
        "sha256:v1:{}",
        digest
            .iter()
            .map(|byte| format!("{byte:02x}"))
            .collect::<String>()
    )
}

pub fn capture_mode_text(value: CaptureMode) -> &'static str {
    match value {
        CaptureMode::BestEffort => "best_effort",
        CaptureMode::Durable => "durable",
    }
}

pub fn content_mode_text(value: ContentCaptureMode) -> &'static str {
    match value {
        ContentCaptureMode::MetadataOnly => "metadata_only",
        ContentCaptureMode::Redacted => "redacted",
        ContentCaptureMode::EncryptedArtifactRef => "encrypted_artifact_ref",
    }
}

pub fn missing_evidence_text(value: MissingEvidenceBehavior) -> &'static str {
    match value {
        MissingEvidenceBehavior::Inconclusive => "inconclusive",
        MissingEvidenceBehavior::Fail => "fail",
    }
}

fn parse_capture_mode(value: &str) -> Result<CaptureMode, StorageError> {
    match value {
        "best_effort" => Ok(CaptureMode::BestEffort),
        "durable" => Ok(CaptureMode::Durable),
        other => Err(StorageError::Internal(format!(
            "unknown capture mode `{other}`"
        ))),
    }
}

fn parse_content_mode(value: &str) -> Result<ContentCaptureMode, StorageError> {
    match value {
        "metadata_only" => Ok(ContentCaptureMode::MetadataOnly),
        "redacted" => Ok(ContentCaptureMode::Redacted),
        "encrypted_artifact_ref" => Ok(ContentCaptureMode::EncryptedArtifactRef),
        other => Err(StorageError::Internal(format!(
            "unknown content capture mode `{other}`"
        ))),
    }
}

fn parse_missing_evidence(value: &str) -> Result<MissingEvidenceBehavior, StorageError> {
    match value {
        "inconclusive" => Ok(MissingEvidenceBehavior::Inconclusive),
        "fail" => Ok(MissingEvidenceBehavior::Fail),
        other => Err(StorageError::Internal(format!(
            "unknown missing-evidence behavior `{other}`"
        ))),
    }
}

fn parse_capture_status(value: &str) -> Result<RunCaptureStatus, StorageError> {
    match value {
        "open" => Ok(RunCaptureStatus::Open),
        "waiting" => Ok(RunCaptureStatus::Waiting),
        "complete" => Ok(RunCaptureStatus::Complete),
        "incomplete" => Ok(RunCaptureStatus::Incomplete),
        other => Err(StorageError::Internal(format!(
            "unknown capture status `{other}`"
        ))),
    }
}

fn participant_role_text(value: RunParticipantRole) -> &'static str {
    match value {
        RunParticipantRole::Primary => "primary",
        RunParticipantRole::Participant => "participant",
    }
}

fn parse_uuid(value: &str) -> Result<Uuid, StorageError> {
    Uuid::parse_str(value).map_err(|error| StorageError::Internal(format!("uuid parse: {error}")))
}

impl std::fmt::Debug for EvaluationRepo {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("EvaluationRepo").finish_non_exhaustive()
    }
}
