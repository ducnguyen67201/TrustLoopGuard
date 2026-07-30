//! Single orchestration chain for every authorization domain.

use std::sync::Arc;

use chrono::{DateTime, Duration, Utc};
use tl_core::{
    ApprovalEnvelope, ApprovalStatus, AuthorityRequirement, AuthorizationApprovalSummary,
    AuthorizationClaim, AuthorizationDecision, AuthorizationEffect, AuthorizationFinding,
    AuthorizationGrantRef, AuthorizationIntentStatus, AuthorizationReceipt, AuthorizationSubject,
    GrantStatus,
};
use uuid::Uuid;

use super::adapters::{AuthorizationAdapterError, AuthorizationAdapterRegistry};
use super::{
    AuthorizationStore, AuthorizationStoreError, NewAuthorizationApproval, NewAuthorizationIntent,
};
use crate::policies::{PolicyStore, PolicyStoreError};

#[derive(Debug, thiserror::Error)]
pub enum AuthorizationError {
    #[error(transparent)]
    Store(#[from] AuthorizationStoreError),
    #[error(transparent)]
    Adapter(#[from] AuthorizationAdapterError),
    #[error("policy store unavailable: {0}")]
    Policy(String),
    #[error("invalid authorization request: {0}")]
    Invalid(String),
    #[error("authorization conflict: {0}")]
    Conflict(String),
}

impl From<PolicyStoreError> for AuthorizationError {
    fn from(error: PolicyStoreError) -> Self {
        Self::Policy(error.to_string())
    }
}

#[derive(Debug, Clone)]
pub struct AuthorizationEvaluationRequest {
    pub workspace_id: String,
    pub environment_id: String,
    pub principal_id: String,
    pub subject: AuthorizationSubject,
    pub findings: Vec<AuthorizationFinding>,
    pub requirements: Vec<AuthorityRequirement>,
    pub policy_versions: Vec<String>,
    pub claim: Option<AuthorizationClaim>,
    /// Coordinator-owned attempt id for executable paths that do not need a
    /// caller grant. A claim's attempt id takes precedence when present.
    pub attempt_id: Option<String>,
    pub trace_id: String,
    pub run_id: Option<String>,
    pub transformed_value: Option<serde_json::Value>,
    pub intent_expires_at: Option<DateTime<Utc>>,
    /// False when upstream evidence is temporarily unavailable and cannot
    /// produce a stable executable-subject fingerprint.
    pub persist_intent: bool,
}

#[derive(Clone)]
pub struct AuthorizationCoordinator {
    store: Arc<dyn AuthorizationStore>,
    policies: Arc<dyn PolicyStore>,
    adapters: Arc<AuthorizationAdapterRegistry>,
}

impl AuthorizationCoordinator {
    pub fn new(
        store: Arc<dyn AuthorizationStore>,
        policies: Arc<dyn PolicyStore>,
        adapters: Arc<AuthorizationAdapterRegistry>,
    ) -> Self {
        Self {
            store,
            policies,
            adapters,
        }
    }

    pub fn store(&self) -> &Arc<dyn AuthorizationStore> {
        &self.store
    }

    pub async fn evaluate(
        &self,
        request: AuthorizationEvaluationRequest,
    ) -> Result<AuthorizationDecision, AuthorizationError> {
        let started = std::time::Instant::now();
        validate_evaluation_request(&request)?;
        let adapter = self.adapters.for_subject(&request.subject);
        let subject = adapter.normalize(&request.subject)?;
        let capability = adapter.capability(&subject)?;
        let fingerprint = adapter.fingerprint(
            &request.workspace_id,
            &request.environment_id,
            &request.principal_id,
            &subject,
        )?;
        let enabled = self
            .policies
            .list_enabled(&request.workspace_id, &request.environment_id)
            .await?;
        let enabled_families = self
            .policies
            .list_enabled_families(&request.workspace_id, &request.environment_id)
            .await?;
        let boundary = adapter.policy_boundary(
            &request.principal_id,
            &subject,
            &enabled,
            &enabled_families,
        )?;

        let mut findings = request.findings;
        findings.extend(boundary.findings);
        findings.extend(adapter.pure_findings(&subject)?);
        findings.extend(adapter.stateful_findings(&subject).await?);
        let mut requirements = request.requirements;
        requirements.extend(boundary.requirements);
        let mut policy_versions = request.policy_versions;
        policy_versions.extend(boundary.policy_versions);
        policy_versions.extend(enabled.iter().map(|policy| policy.id.to_string()));
        policy_versions.extend(
            enabled_families
                .iter()
                .map(|policy| policy.id().to_string()),
        );
        policy_versions.sort();
        policy_versions.dedup();

        let executable = subject.domain() != tl_core::AuthorizationDomain::Content;
        let subject_id = subject_id(&subject);
        let intent_id = if executable && request.persist_intent {
            let suggested_id = deterministic_intent_id(
                &request.workspace_id,
                &request.environment_id,
                subject.domain(),
                &subject_id,
            );
            Some(
                self.store
                    .create_or_get_intent(NewAuthorizationIntent {
                        workspace_id: request.workspace_id.clone(),
                        environment_id: request.environment_id.clone(),
                        id: suggested_id,
                        domain: subject.domain(),
                        subject_id: subject_id.clone(),
                        idempotency_key: subject_id.clone(),
                        principal_id: request.principal_id.clone(),
                        operation: operation(&subject),
                        fingerprint: fingerprint.clone(),
                        fingerprint_version: 1,
                        subject_snapshot: serde_json::to_value(&subject)
                            .map_err(|error| AuthorizationError::Invalid(error.to_string()))?,
                        expires_at: request.intent_expires_at,
                    })
                    .await?,
            )
        } else {
            None
        };

        let attempt_id = request
            .claim
            .as_ref()
            .map(|claim| claim.attempt_id.as_str())
            .or(request.attempt_id.as_deref());
        let existing_lease = match (attempt_id, intent_id.as_deref()) {
            (Some(attempt_id), Some(intent_id)) => {
                let lease = self
                    .store
                    .get_lease_by_attempt(
                        &request.workspace_id,
                        &request.environment_id,
                        intent_id,
                        attempt_id,
                    )
                    .await?;
                if lease
                    .as_ref()
                    .is_some_and(|lease| lease.fingerprint != fingerprint)
                {
                    return Err(AuthorizationError::Conflict(
                        "attempt id was already used for a different subject".into(),
                    ));
                }
                lease
            }
            _ => None,
        };

        // Runtime authority is explicit: only the grant named by the claim may
        // satisfy requirements or be consumed by this attempt.
        let mut grants = if let Some(claim) = &request.claim {
            let grant = self
                .store
                .get_grant(
                    &request.workspace_id,
                    &request.environment_id,
                    &claim.grant_id,
                )
                .await?;
            if grant.principal_id != request.principal_id
                || grant.domain != subject.domain()
                || (!grant_is_current(&grant) && existing_lease.is_none())
            {
                return Err(AuthorizationError::Conflict(
                    "authorization claim does not name current authority for this subject".into(),
                ));
            }
            vec![grant]
        } else {
            Vec::new()
        };
        if let (Some(claim), Some(existing)) = (&request.claim, &existing_lease) {
            if existing.grant_id.as_deref() != Some(claim.grant_id.as_str()) {
                return Err(AuthorizationError::Conflict(
                    "attempt id belongs to a different grant".into(),
                ));
            }
            if let Ok(mut consumed) = self
                .store
                .get_grant(
                    &request.workspace_id,
                    &request.environment_id,
                    &claim.grant_id,
                )
                .await
            {
                consumed.status = GrantStatus::Active;
                consumed.use_count = consumed.use_count.saturating_sub(1);
                if !grants.iter().any(|grant| grant.id == consumed.id) {
                    grants.push(consumed);
                }
            }
        }

        let composition =
            tl_engine::compose_findings(&findings, &requirements, &grants, &subject, &fingerprint);
        let reason = if composition.effect == AuthorizationEffect::Permit {
            "current policy and authority permit the subject".into()
        } else {
            findings
                .iter()
                .find(|finding| finding.effect == composition.effect)
                .map(|finding| finding.reason.clone())
                .unwrap_or_else(|| "authorization requirements were not satisfied".into())
        };
        let status = intent_id
            .as_ref()
            .map(|_| intent_status(composition.effect));
        let applied_grant = composition
            .applied_grant_id
            .as_deref()
            .and_then(|id| grants.iter().find(|grant| grant.id == id));

        if let Some(claim) = request.claim.as_ref() {
            if applied_grant.map(|grant| grant.id.as_str()) != Some(claim.grant_id.as_str()) {
                return Err(AuthorizationError::Conflict(
                    "claim does not satisfy every current authority requirement".into(),
                ));
            }
        }

        let approval = if composition.effect == AuthorizationEffect::RequireApproval {
            let intent_id = intent_id.as_deref().ok_or_else(|| {
                AuthorizationError::Invalid("content observations cannot create approvals".into())
            })?;
            let issued_at = Utc::now();
            let expires_at = request
                .intent_expires_at
                .unwrap_or_else(|| issued_at + Duration::minutes(15));
            let remaining = composition.remaining_requirement_ids.clone();
            let approver_roles = requirements
                .iter()
                .filter(|requirement| remaining.contains(&requirement.id))
                .flat_map(|requirement| requirement.approver_roles.iter().cloned())
                .collect::<std::collections::BTreeSet<_>>()
                .into_iter()
                .collect::<Vec<_>>();
            let scoped_allowed = !remaining.is_empty()
                && requirements
                    .iter()
                    .filter(|requirement| remaining.contains(&requirement.id))
                    .all(|requirement| requirement.reusable_allowed);
            let stored = self
                .store
                .create_or_get_approval(NewAuthorizationApproval {
                    workspace_id: request.workspace_id.clone(),
                    environment_id: request.environment_id.clone(),
                    envelope: ApprovalEnvelope {
                        schema: "authorization-envelope:v1".into(),
                        intent_id: intent_id.to_string(),
                        domain: subject.domain(),
                        capability: capability.clone(),
                        principal_id: request.principal_id.clone(),
                        subject_id: subject_id.clone(),
                        subject_hash: fingerprint.clone(),
                        exact_fingerprint: fingerprint.clone(),
                        fingerprint_version: 1,
                        requirement_ids: remaining,
                        proposed_scope: if scoped_allowed {
                            adapter.proposed_scope(&subject)?
                        } else {
                            None
                        },
                        policy_versions: policy_versions.clone(),
                        issued_at: issued_at.to_rfc3339(),
                        expires_at: expires_at.to_rfc3339(),
                    },
                    approver_roles,
                })
                .await?;
            Some(AuthorizationApprovalSummary {
                id: stored.id,
                status: ApprovalStatus::Pending,
                envelope_hash: stored.envelope_hash,
                expires_at: stored.expires_at,
                poll_after_ms: 1_000,
            })
        } else {
            None
        };

        let lease = if composition.effect.is_executable() {
            adapter.precommit(&subject).await?;
            if let Some(existing) = existing_lease {
                Some(existing)
            } else if let (Some(attempt_id), Some(intent_id)) = (attempt_id, intent_id.as_deref()) {
                Some(
                    self.store
                        .claim_lease(
                            &request.workspace_id,
                            &request.environment_id,
                            intent_id,
                            applied_grant.map(|grant| grant.id.as_str()),
                            attempt_id,
                            &fingerprint,
                        )
                        .await?,
                )
            } else {
                None
            }
        } else {
            None
        };

        if let (Some(intent_id), Some(status)) = (intent_id.as_deref(), status) {
            self.store
                .record_decision(
                    &request.workspace_id,
                    &request.environment_id,
                    intent_id,
                    composition.effect,
                    status,
                    &reason,
                    &request.trace_id,
                )
                .await?;
        }

        let receipt_id = Uuid::now_v7().to_string();
        let receipt = AuthorizationReceipt {
            id: receipt_id.clone(),
            intent_id: intent_id.clone(),
            trace_id: Some(request.trace_id.clone()),
            principal_id: Some(request.principal_id.clone()),
            operation: Some(operation(&subject)),
            run_id: request.run_id.clone(),
            domain: subject.domain(),
            effect: composition.effect,
            intent_status: status,
            subject_hash: fingerprint,
            reason: reason.clone(),
            findings: findings.clone(),
            policy_versions,
            approval_id: approval.as_ref().map(|approval| approval.id.clone()),
            grant_id: applied_grant.map(|grant| grant.id.clone()),
            lease_id: lease.as_ref().map(|lease| lease.id.clone()),
            domain_evidence: adapter.domain_evidence(&subject)?,
            created_at: Utc::now().to_rfc3339(),
        };
        self.store
            .write_receipt(&request.workspace_id, &request.environment_id, receipt)
            .await?;

        Ok(AuthorizationDecision {
            trace_id: request.trace_id,
            intent_id,
            domain: subject.domain(),
            effect: composition.effect,
            status,
            reason,
            findings,
            transformed_value: request.transformed_value,
            approval,
            applied_grant: applied_grant.map(|grant| AuthorizationGrantRef {
                id: grant.id.clone(),
                capability: grant.capability.clone(),
                mode: grant.mode,
                source: grant.source,
            }),
            lease,
            receipt_id: Some(receipt_id),
            latency_ms: started.elapsed().as_millis() as u64,
        })
    }

    pub async fn decide_approval(
        &self,
        workspace_id: &str,
        environment_id: &str,
        approval_id: &str,
        actor_id: &str,
        request: tl_core::DecideAuthorizationApprovalRequest,
    ) -> Result<tl_core::DecideAuthorizationApprovalResponse, AuthorizationError> {
        Ok(self
            .store
            .decide_approval(workspace_id, environment_id, approval_id, actor_id, request)
            .await?)
    }

    pub async fn create_user_intent_grant(
        &self,
        workspace_id: &str,
        environment_id: &str,
        actor_id: &str,
        request: tl_core::CreateAuthorizationGrantRequest,
    ) -> Result<tl_core::AuthorizationGrant, AuthorizationError> {
        if request.principal_id.trim().is_empty()
            || request.requirement_ids.is_empty()
            || request
                .requirement_ids
                .iter()
                .any(|requirement| requirement.trim().is_empty())
            || request.max_uses == Some(0)
        {
            return Err(AuthorizationError::Invalid(
                "principal, requirement ids, and positive usage bounds are required".into(),
            ));
        }
        let starts_at = request
            .starts_at
            .as_deref()
            .map(DateTime::parse_from_rfc3339)
            .transpose()
            .map_err(|_| AuthorizationError::Invalid("invalid grant start time".into()))?;
        let expires_at = request
            .expires_at
            .as_deref()
            .map(DateTime::parse_from_rfc3339)
            .transpose()
            .map_err(|_| AuthorizationError::Invalid("invalid grant expiry time".into()))?;
        if starts_at
            .as_ref()
            .zip(expires_at.as_ref())
            .is_some_and(|(starts, expires)| starts >= expires)
        {
            return Err(AuthorizationError::Invalid(
                "grant start time must precede expiry".into(),
            ));
        }
        let adapter = self.adapters.for_domain(request.domain);
        if !scope_matches_adapter(adapter, &request.scope) {
            return Err(AuthorizationError::Invalid(
                "grant scope does not match its authorization domain".into(),
            ));
        }
        Ok(self
            .store
            .create_grant(workspace_id, environment_id, actor_id, request)
            .await?)
    }

    pub async fn revoke_grant(
        &self,
        workspace_id: &str,
        environment_id: &str,
        grant_id: &str,
        actor_id: &str,
    ) -> Result<tl_core::AuthorizationGrant, AuthorizationError> {
        Ok(self
            .store
            .revoke_grant(workspace_id, environment_id, grant_id, actor_id)
            .await?)
    }

    pub async fn complete_lease(
        &self,
        workspace_id: &str,
        environment_id: &str,
        lease_id: &str,
        request: tl_core::CompleteAuthorizationLeaseRequest,
    ) -> Result<tl_core::AuthorizationLease, AuthorizationError> {
        Ok(self
            .store
            .complete_lease(workspace_id, environment_id, lease_id, request)
            .await?)
    }
}

fn validate_evaluation_request(
    request: &AuthorizationEvaluationRequest,
) -> Result<(), AuthorizationError> {
    if request.workspace_id.trim().is_empty()
        || request.environment_id.trim().is_empty()
        || request.principal_id.trim().is_empty()
        || request.trace_id.trim().is_empty()
    {
        return Err(AuthorizationError::Invalid(
            "workspace, environment, principal, and trace id are required".into(),
        ));
    }
    if request
        .requirements
        .iter()
        .any(|requirement| requirement.id.trim().is_empty())
    {
        return Err(AuthorizationError::Invalid(
            "authority requirement ids cannot be empty".into(),
        ));
    }
    Ok(())
}

fn subject_id(subject: &AuthorizationSubject) -> String {
    match subject {
        AuthorizationSubject::Content { event_kind, .. } => format!("content:{event_kind:?}"),
        AuthorizationSubject::Tool { invocation_id, .. } => invocation_id.clone(),
        AuthorizationSubject::Financial { action_id, .. } => action_id.clone(),
    }
}

fn operation(subject: &AuthorizationSubject) -> String {
    match subject {
        AuthorizationSubject::Content { event_kind, .. } => format!("{event_kind:?}"),
        AuthorizationSubject::Tool { operation, .. } => operation.clone(),
        AuthorizationSubject::Financial { action, .. } => action.operation.clone(),
    }
}

fn deterministic_intent_id(
    workspace_id: &str,
    environment_id: &str,
    domain: tl_core::AuthorizationDomain,
    subject_id: &str,
) -> String {
    Uuid::new_v5(
        &Uuid::NAMESPACE_URL,
        format!("tlg:{workspace_id}:{environment_id}:{domain:?}:{subject_id}").as_bytes(),
    )
    .to_string()
}

fn intent_status(effect: AuthorizationEffect) -> AuthorizationIntentStatus {
    match effect {
        AuthorizationEffect::Permit | AuthorizationEffect::Transform => {
            AuthorizationIntentStatus::Authorized
        }
        AuthorizationEffect::Deny => AuthorizationIntentStatus::Denied,
        AuthorizationEffect::RequireApproval => AuthorizationIntentStatus::PendingApproval,
        AuthorizationEffect::Defer => AuthorizationIntentStatus::Deferred,
    }
}

fn grant_is_current(grant: &tl_core::AuthorizationGrant) -> bool {
    if grant.status != GrantStatus::Active
        || grant
            .max_uses
            .is_some_and(|maximum| grant.use_count >= maximum)
    {
        return false;
    }
    let now = Utc::now();
    let started = grant.starts_at.as_deref().map_or(true, |value| {
        DateTime::parse_from_rfc3339(value)
            .map(|value| value.with_timezone(&Utc) <= now)
            .unwrap_or(false)
    });
    let unexpired = grant.expires_at.as_deref().map_or(true, |value| {
        DateTime::parse_from_rfc3339(value)
            .map(|value| value.with_timezone(&Utc) > now)
            .unwrap_or(false)
    });
    started && unexpired
}

fn scope_matches_adapter(
    adapter: &dyn super::adapters::AuthorizationAdapter,
    scope: &tl_core::AuthorizationGrantScope,
) -> bool {
    matches!(
        (adapter.domain(), scope),
        (
            tl_core::AuthorizationDomain::Content | tl_core::AuthorizationDomain::Tool,
            tl_core::AuthorizationGrantScope::Action(_)
        ) | (
            tl_core::AuthorizationDomain::Financial,
            tl_core::AuthorizationGrantScope::Financial(_)
        )
    )
}

impl std::fmt::Debug for AuthorizationCoordinator {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("AuthorizationCoordinator")
            .finish_non_exhaustive()
    }
}
