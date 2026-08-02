//! Unified authorization control plane.
//!
//! This module owns the single approval, grant, lease, and receipt surface.
//! Domain services propose typed subjects to the coordinator; they never
//! create their own approval or reusable-authority state machines.

pub mod adapters;
pub mod coordinator;
pub mod fingerprint;

pub use coordinator::{
    AuthorizationCoordinator, AuthorizationError, AuthorizationEvaluationRequest,
};

use std::{collections::HashMap, sync::Arc};

use async_trait::async_trait;
use axum::{
    extract::{Extension, Path, State},
    http::{HeaderMap, StatusCode},
    response::{IntoResponse, Response},
    Json,
};
use chrono::{DateTime, Duration, Utc};
use sha2::{Digest, Sha256};
use tl_core::{
    ApiErrorCode, ApprovalDecision, ApprovalStatus, AuthorizationApproval,
    AuthorizationApprovalListResponse, AuthorizationDomain, AuthorizationGrant,
    AuthorizationGrantListResponse, AuthorizationGrantSource, AuthorizationLease,
    AuthorizationReceipt, AuthorizationReceiptListResponse, CompleteAuthorizationLeaseRequest,
    CreateAuthorizationGrantRequest, DecideAuthorizationApprovalRequest,
    DecideAuthorizationApprovalResponse, GrantMode, GrantStatus, LeaseStatus,
};
use tokio::sync::RwLock;
use uuid::Uuid;

use crate::{
    app::error::api_error_response,
    auth::{InternalServiceContext, WorkspaceKeyContext},
    jwt::UserContext,
    team::TeamStore,
};

#[derive(Debug, Clone)]
pub struct NewAuthorizationApproval {
    pub workspace_id: String,
    pub environment_id: String,
    pub envelope: tl_core::ApprovalEnvelope,
    pub approver_roles: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct NewAuthorizationIntent {
    pub workspace_id: String,
    pub environment_id: String,
    pub id: String,
    pub domain: AuthorizationDomain,
    pub subject_id: String,
    pub idempotency_key: String,
    pub principal_id: String,
    pub operation: String,
    pub fingerprint: String,
    pub fingerprint_version: i32,
    pub subject_snapshot: serde_json::Value,
    pub expires_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone)]
struct MemoryAuthorizationIntent {
    id: String,
    workspace_id: String,
    environment_id: String,
    domain: AuthorizationDomain,
    subject_id: String,
    principal_id: String,
    fingerprint: String,
    effect: tl_core::AuthorizationEffect,
    status: tl_core::AuthorizationIntentStatus,
    reason: String,
    trace_id: Option<String>,
}

#[derive(Debug, thiserror::Error)]
pub enum AuthorizationStoreError {
    #[error("not found")]
    NotFound,
    #[error("conflict: {0}")]
    Conflict(String),
    #[error("invalid: {0}")]
    Invalid(String),
    #[error("internal: {0}")]
    Internal(String),
}

#[async_trait]
pub trait AuthorizationStore: Send + Sync {
    async fn create_or_get_intent(
        &self,
        input: NewAuthorizationIntent,
    ) -> Result<String, AuthorizationStoreError>;
    async fn record_decision(
        &self,
        workspace_id: &str,
        environment_id: &str,
        intent_id: &str,
        effect: tl_core::AuthorizationEffect,
        status: tl_core::AuthorizationIntentStatus,
        reason: &str,
        trace_id: &str,
    ) -> Result<(), AuthorizationStoreError>;
    async fn create_or_get_approval(
        &self,
        input: NewAuthorizationApproval,
    ) -> Result<AuthorizationApproval, AuthorizationStoreError>;
    async fn get_approval(
        &self,
        workspace_id: &str,
        environment_id: &str,
        approval_id: &str,
    ) -> Result<AuthorizationApproval, AuthorizationStoreError>;
    async fn list_approvals(
        &self,
        workspace_id: &str,
        environment_id: Option<&str>,
    ) -> Result<Vec<AuthorizationApproval>, AuthorizationStoreError>;
    async fn decide_approval(
        &self,
        workspace_id: &str,
        environment_id: &str,
        approval_id: &str,
        actor_id: &str,
        request: DecideAuthorizationApprovalRequest,
    ) -> Result<DecideAuthorizationApprovalResponse, AuthorizationStoreError>;
    async fn create_grant(
        &self,
        workspace_id: &str,
        environment_id: &str,
        actor_id: &str,
        request: CreateAuthorizationGrantRequest,
    ) -> Result<AuthorizationGrant, AuthorizationStoreError>;
    async fn get_grant(
        &self,
        workspace_id: &str,
        environment_id: &str,
        grant_id: &str,
    ) -> Result<AuthorizationGrant, AuthorizationStoreError>;
    async fn list_grants(
        &self,
        workspace_id: &str,
        environment_id: Option<&str>,
    ) -> Result<Vec<AuthorizationGrant>, AuthorizationStoreError>;
    async fn revoke_grant(
        &self,
        workspace_id: &str,
        environment_id: &str,
        grant_id: &str,
        actor_id: &str,
    ) -> Result<AuthorizationGrant, AuthorizationStoreError>;
    async fn claim_lease(
        &self,
        workspace_id: &str,
        environment_id: &str,
        intent_id: &str,
        grant_id: Option<&str>,
        attempt_id: &str,
        fingerprint: &str,
    ) -> Result<AuthorizationLease, AuthorizationStoreError>;
    async fn get_lease_by_attempt(
        &self,
        workspace_id: &str,
        environment_id: &str,
        intent_id: &str,
        attempt_id: &str,
    ) -> Result<Option<AuthorizationLease>, AuthorizationStoreError>;
    async fn complete_lease(
        &self,
        workspace_id: &str,
        environment_id: &str,
        lease_id: &str,
        request: CompleteAuthorizationLeaseRequest,
    ) -> Result<AuthorizationLease, AuthorizationStoreError>;
    async fn get_lease_principal(
        &self,
        workspace_id: &str,
        environment_id: &str,
        lease_id: &str,
    ) -> Result<String, AuthorizationStoreError>;
    async fn write_receipt(
        &self,
        workspace_id: &str,
        environment_id: &str,
        receipt: AuthorizationReceipt,
    ) -> Result<(), AuthorizationStoreError>;
    async fn get_receipt(
        &self,
        workspace_id: &str,
        environment_id: &str,
        receipt_id: &str,
    ) -> Result<AuthorizationReceipt, AuthorizationStoreError>;
    async fn get_receipt_principal(
        &self,
        workspace_id: &str,
        environment_id: &str,
        receipt_id: &str,
    ) -> Result<String, AuthorizationStoreError>;
    async fn list_receipts(
        &self,
        workspace_id: &str,
        environment_id: Option<&str>,
    ) -> Result<Vec<AuthorizationReceipt>, AuthorizationStoreError>;
}

#[derive(Debug, Default)]
pub struct MemoryAuthorizationStore {
    intents: RwLock<HashMap<(String, String, String), MemoryAuthorizationIntent>>,
    approvals: RwLock<HashMap<(String, String, String), AuthorizationApproval>>,
    grants: RwLock<HashMap<(String, String, String), AuthorizationGrant>>,
    leases: RwLock<HashMap<(String, String, String), AuthorizationLease>>,
    receipts: RwLock<HashMap<(String, String, String), AuthorizationReceipt>>,
}

impl MemoryAuthorizationStore {
    pub fn new() -> Self {
        Self::default()
    }
}

#[async_trait]
impl AuthorizationStore for MemoryAuthorizationStore {
    async fn create_or_get_intent(
        &self,
        input: NewAuthorizationIntent,
    ) -> Result<String, AuthorizationStoreError> {
        let mut intents = self.intents.write().await;
        if let Some(existing) = intents.values().find(|intent| {
            intent.workspace_id == input.workspace_id
                && intent.environment_id == input.environment_id
                && intent.domain == input.domain
                && intent.subject_id == input.subject_id
        }) {
            return if existing.fingerprint == input.fingerprint {
                Ok(existing.id.clone())
            } else {
                Err(AuthorizationStoreError::Conflict(
                    "the durable subject changed for an existing intent".into(),
                ))
            };
        }
        let intent = MemoryAuthorizationIntent {
            id: input.id,
            workspace_id: input.workspace_id.clone(),
            environment_id: input.environment_id.clone(),
            domain: input.domain,
            subject_id: input.subject_id,
            principal_id: input.principal_id,
            fingerprint: input.fingerprint,
            effect: tl_core::AuthorizationEffect::Permit,
            status: tl_core::AuthorizationIntentStatus::Evaluating,
            reason: "authorization evaluation started".into(),
            trace_id: None,
        };
        let id = intent.id.clone();
        intents.insert(
            (input.workspace_id, input.environment_id, id.clone()),
            intent,
        );
        Ok(id)
    }

    async fn record_decision(
        &self,
        workspace_id: &str,
        environment_id: &str,
        intent_id: &str,
        effect: tl_core::AuthorizationEffect,
        status: tl_core::AuthorizationIntentStatus,
        reason: &str,
        trace_id: &str,
    ) -> Result<(), AuthorizationStoreError> {
        let mut intents = self.intents.write().await;
        let intent = intents
            .get_mut(&key(workspace_id, environment_id, intent_id))
            .ok_or(AuthorizationStoreError::NotFound)?;
        intent.effect = effect;
        intent.status = status;
        intent.reason = reason.to_string();
        intent.trace_id = Some(trace_id.to_string());
        Ok(())
    }

    async fn create_or_get_approval(
        &self,
        input: NewAuthorizationApproval,
    ) -> Result<AuthorizationApproval, AuthorizationStoreError> {
        let envelope_hash = hash_envelope(&input.envelope)?;
        let mut approvals = self.approvals.write().await;
        if let Some(existing) = approvals.values().find(|approval| {
            approval.workspace_id == input.workspace_id
                && approval.environment_id == input.environment_id
                && approval.intent_id == input.envelope.intent_id
                && approval.status == ApprovalStatus::Pending
        }) {
            return if existing.envelope_hash == envelope_hash {
                Ok(existing.clone())
            } else {
                Err(AuthorizationStoreError::Conflict(
                    "the pending intent changed after review was requested".into(),
                ))
            };
        }
        let now = Utc::now().to_rfc3339();
        let approval = AuthorizationApproval {
            id: Uuid::now_v7().to_string(),
            workspace_id: input.workspace_id.clone(),
            environment_id: input.environment_id.clone(),
            intent_id: input.envelope.intent_id.clone(),
            status: ApprovalStatus::Pending,
            envelope: input.envelope.clone(),
            envelope_hash,
            approver_roles: input.approver_roles,
            decided_by: None,
            decided_at: None,
            decision_reason: None,
            grant_id: None,
            expires_at: input.envelope.expires_at.clone(),
            created_at: now.clone(),
            updated_at: now,
        };
        approvals.insert(
            (
                input.workspace_id,
                input.environment_id,
                approval.id.clone(),
            ),
            approval.clone(),
        );
        Ok(approval)
    }

    async fn get_approval(
        &self,
        workspace_id: &str,
        environment_id: &str,
        approval_id: &str,
    ) -> Result<AuthorizationApproval, AuthorizationStoreError> {
        let approval_key = key(workspace_id, environment_id, approval_id);
        let mut approvals = self.approvals.write().await;
        let approval = approvals
            .get_mut(&approval_key)
            .ok_or(AuthorizationStoreError::NotFound)?;
        expire_approval(approval);
        Ok(approval.clone())
    }

    async fn list_approvals(
        &self,
        workspace_id: &str,
        environment_id: Option<&str>,
    ) -> Result<Vec<AuthorizationApproval>, AuthorizationStoreError> {
        let mut approvals = self.approvals.write().await;
        approvals.values_mut().for_each(expire_approval);
        let mut rows = approvals
            .values()
            .filter(|approval| {
                approval.workspace_id == workspace_id
                    && environment_id
                        .map_or(true, |environment| approval.environment_id == environment)
            })
            .cloned()
            .collect::<Vec<_>>();
        rows.sort_by(|left, right| right.created_at.cmp(&left.created_at));
        Ok(rows)
    }

    async fn decide_approval(
        &self,
        workspace_id: &str,
        environment_id: &str,
        approval_id: &str,
        actor_id: &str,
        request: DecideAuthorizationApprovalRequest,
    ) -> Result<DecideAuthorizationApprovalResponse, AuthorizationStoreError> {
        let approval_key = key(workspace_id, environment_id, approval_id);
        let mut approvals = self.approvals.write().await;
        let approval = approvals
            .get_mut(&approval_key)
            .ok_or(AuthorizationStoreError::NotFound)?;
        expire_approval(approval);
        if approval.status != ApprovalStatus::Pending {
            return Err(AuthorizationStoreError::Conflict(
                "approval is no longer pending".into(),
            ));
        }
        let recomputed = hash_envelope(&approval.envelope)?;
        if request.envelope_hash != approval.envelope_hash || request.envelope_hash != recomputed {
            return Err(AuthorizationStoreError::Conflict(
                "approval envelope changed; refresh before deciding".into(),
            ));
        }
        let now = Utc::now();
        approval.status = match request.decision {
            ApprovalDecision::Approve => ApprovalStatus::Approved,
            ApprovalDecision::Deny => ApprovalStatus::Denied,
        };
        approval.decided_by = Some(actor_id.to_string());
        approval.decided_at = Some(now.to_rfc3339());
        approval.decision_reason = request.reason.clone();
        approval.updated_at = now.to_rfc3339();

        let grant = if request.decision == ApprovalDecision::Approve {
            let (scope, exact_fingerprint, max_uses) = match request.mode {
                GrantMode::ExactOnce => (
                    None,
                    Some(approval.envelope.exact_fingerprint.clone()),
                    Some(1),
                ),
                GrantMode::Scoped => {
                    let requested = request.scope.clone().ok_or_else(|| {
                        AuthorizationStoreError::Invalid(
                            "scope is required for a scoped approval".into(),
                        )
                    })?;
                    if approval.envelope.proposed_scope.as_ref() != Some(&requested) {
                        return Err(AuthorizationStoreError::Conflict(
                            "requested scope differs from the reviewed scope".into(),
                        ));
                    }
                    (Some(requested), None, None)
                }
            };
            let grant = AuthorizationGrant {
                id: Uuid::now_v7().to_string(),
                workspace_id: workspace_id.to_string(),
                environment_id: environment_id.to_string(),
                principal_id: approval.envelope.principal_id.clone(),
                domain: approval.envelope.domain,
                capability: approval.envelope.capability.clone(),
                mode: request.mode,
                status: GrantStatus::Active,
                source: AuthorizationGrantSource::ReviewerApproval,
                scope,
                exact_fingerprint,
                fingerprint_version: approval.envelope.fingerprint_version,
                source_approval_id: Some(approval.id.clone()),
                requirement_ids: approval.envelope.requirement_ids.clone(),
                max_uses,
                use_count: 0,
                starts_at: request.starts_at,
                expires_at: Some(bounded_grant_expiry(
                    request.expires_at.as_deref(),
                    &approval.expires_at,
                )?),
                created_by: actor_id.to_string(),
                created_at: now.to_rfc3339(),
                updated_at: now.to_rfc3339(),
            };
            self.grants
                .write()
                .await
                .insert(key(workspace_id, environment_id, &grant.id), grant.clone());
            approval.grant_id = Some(grant.id.clone());
            Some(grant)
        } else {
            None
        };
        Ok(DecideAuthorizationApprovalResponse {
            approval: approval.clone(),
            grant,
        })
    }

    async fn create_grant(
        &self,
        workspace_id: &str,
        environment_id: &str,
        actor_id: &str,
        request: CreateAuthorizationGrantRequest,
    ) -> Result<AuthorizationGrant, AuthorizationStoreError> {
        if !scope_matches_domain(&request.scope, request.domain) {
            return Err(AuthorizationStoreError::Invalid(
                "grant scope does not match its domain".into(),
            ));
        }
        let now = Utc::now().to_rfc3339();
        let grant = AuthorizationGrant {
            id: Uuid::now_v7().to_string(),
            workspace_id: workspace_id.to_string(),
            environment_id: environment_id.to_string(),
            principal_id: request.principal_id,
            domain: request.domain,
            capability: request.capability,
            mode: GrantMode::Scoped,
            status: GrantStatus::Active,
            source: AuthorizationGrantSource::UserIntent,
            scope: Some(request.scope),
            exact_fingerprint: None,
            fingerprint_version: 1,
            source_approval_id: None,
            requirement_ids: request.requirement_ids,
            max_uses: request.max_uses,
            use_count: 0,
            starts_at: request.starts_at,
            expires_at: request.expires_at,
            created_by: actor_id.to_string(),
            created_at: now.clone(),
            updated_at: now,
        };
        self.grants
            .write()
            .await
            .insert(key(workspace_id, environment_id, &grant.id), grant.clone());
        Ok(grant)
    }

    async fn get_grant(
        &self,
        workspace_id: &str,
        environment_id: &str,
        grant_id: &str,
    ) -> Result<AuthorizationGrant, AuthorizationStoreError> {
        let key = key(workspace_id, environment_id, grant_id);
        let mut grants = self.grants.write().await;
        let grant = grants
            .get_mut(&key)
            .ok_or(AuthorizationStoreError::NotFound)?;
        expire_grant(grant);
        Ok(grant.clone())
    }

    async fn list_grants(
        &self,
        workspace_id: &str,
        environment_id: Option<&str>,
    ) -> Result<Vec<AuthorizationGrant>, AuthorizationStoreError> {
        let mut grants = self.grants.write().await;
        grants.values_mut().for_each(expire_grant);
        let mut rows = grants
            .values()
            .filter(|grant| {
                grant.workspace_id == workspace_id
                    && environment_id
                        .map_or(true, |environment| grant.environment_id == environment)
            })
            .cloned()
            .collect::<Vec<_>>();
        rows.sort_by(|left, right| right.created_at.cmp(&left.created_at));
        Ok(rows)
    }

    async fn revoke_grant(
        &self,
        workspace_id: &str,
        environment_id: &str,
        grant_id: &str,
        actor_id: &str,
    ) -> Result<AuthorizationGrant, AuthorizationStoreError> {
        let key = key(workspace_id, environment_id, grant_id);
        let mut grants = self.grants.write().await;
        let grant = grants
            .get_mut(&key)
            .ok_or(AuthorizationStoreError::NotFound)?;
        if grant.status != GrantStatus::Active {
            return Err(AuthorizationStoreError::Conflict(
                "grant is not active".into(),
            ));
        }
        grant.status = GrantStatus::Revoked;
        grant.updated_at = Utc::now().to_rfc3339();
        grant.created_by = actor_id.to_string();
        Ok(grant.clone())
    }

    async fn claim_lease(
        &self,
        workspace_id: &str,
        environment_id: &str,
        intent_id: &str,
        grant_id: Option<&str>,
        attempt_id: &str,
        fingerprint: &str,
    ) -> Result<AuthorizationLease, AuthorizationStoreError> {
        let mut leases = self.leases.write().await;
        if let Some(existing) = leases
            .iter()
            .find_map(|((workspace, environment, _), lease)| {
                (workspace == workspace_id
                    && environment == environment_id
                    && lease.intent_id == intent_id
                    && lease.attempt_id == attempt_id)
                    .then_some(lease)
            })
        {
            if existing.fingerprint != fingerprint
                || existing.grant_id.as_deref() != grant_id
                || !matches!(
                    existing.status,
                    LeaseStatus::Claimed | LeaseStatus::Consumed
                )
            {
                return Err(AuthorizationStoreError::Conflict(
                    "attempt id was already used with different authority or subject state".into(),
                ));
            }
            return Ok(existing.clone());
        }
        let intent = self
            .intents
            .read()
            .await
            .get(&key(workspace_id, environment_id, intent_id))
            .cloned()
            .ok_or(AuthorizationStoreError::NotFound)?;
        if intent.fingerprint != fingerprint {
            return Err(AuthorizationStoreError::Conflict(
                "lease fingerprint differs from the durable intent".into(),
            ));
        }
        if let Some(grant_id) = grant_id {
            let mut grants = self.grants.write().await;
            let grant = grants
                .get_mut(&key(workspace_id, environment_id, grant_id))
                .ok_or(AuthorizationStoreError::NotFound)?;
            expire_grant(grant);
            if grant.status != GrantStatus::Active
                || grant.max_uses.is_some_and(|max| grant.use_count >= max)
            {
                return Err(AuthorizationStoreError::Conflict(
                    "grant is no longer usable".into(),
                ));
            }
            grant.use_count += 1;
            if grant.max_uses.is_some_and(|max| grant.use_count >= max) {
                grant.status = GrantStatus::Exhausted;
            }
            grant.updated_at = Utc::now().to_rfc3339();
        }
        let now = Utc::now();
        let lease = AuthorizationLease {
            id: Uuid::now_v7().to_string(),
            intent_id: intent_id.to_string(),
            grant_id: grant_id.map(str::to_string),
            attempt_id: attempt_id.to_string(),
            fingerprint: fingerprint.to_string(),
            status: LeaseStatus::Claimed,
            claimed_at: now.to_rfc3339(),
            completed_at: None,
            expires_at: (now + Duration::minutes(5)).to_rfc3339(),
        };
        leases.insert(key(workspace_id, environment_id, &lease.id), lease.clone());
        Ok(lease)
    }

    async fn get_lease_by_attempt(
        &self,
        workspace_id: &str,
        environment_id: &str,
        intent_id: &str,
        attempt_id: &str,
    ) -> Result<Option<AuthorizationLease>, AuthorizationStoreError> {
        Ok(self
            .leases
            .read()
            .await
            .iter()
            .find(|((workspace, environment, _), lease)| {
                workspace == workspace_id
                    && environment == environment_id
                    && lease.intent_id == intent_id
                    && lease.attempt_id == attempt_id
            })
            .map(|(_, lease)| lease.clone()))
    }

    async fn complete_lease(
        &self,
        workspace_id: &str,
        environment_id: &str,
        lease_id: &str,
        request: CompleteAuthorizationLeaseRequest,
    ) -> Result<AuthorizationLease, AuthorizationStoreError> {
        if !matches!(
            request.status,
            LeaseStatus::Consumed | LeaseStatus::Canceled
        ) {
            return Err(AuthorizationStoreError::Invalid(
                "a lease can only complete as consumed or canceled".into(),
            ));
        }
        let key = key(workspace_id, environment_id, lease_id);
        let mut leases = self.leases.write().await;
        let lease = leases
            .get_mut(&key)
            .ok_or(AuthorizationStoreError::NotFound)?;
        if lease.status == request.status {
            return Ok(lease.clone());
        }
        if lease.status != LeaseStatus::Claimed {
            return Err(AuthorizationStoreError::Conflict(
                "lease is no longer claimable".into(),
            ));
        }
        lease.status = request.status;
        lease.completed_at = Some(Utc::now().to_rfc3339());
        Ok(lease.clone())
    }

    async fn get_lease_principal(
        &self,
        workspace_id: &str,
        environment_id: &str,
        lease_id: &str,
    ) -> Result<String, AuthorizationStoreError> {
        let intent_id = self
            .leases
            .read()
            .await
            .get(&key(workspace_id, environment_id, lease_id))
            .map(|lease| lease.intent_id.clone())
            .ok_or(AuthorizationStoreError::NotFound)?;
        self.intents
            .read()
            .await
            .get(&key(workspace_id, environment_id, &intent_id))
            .map(|intent| intent.principal_id.clone())
            .ok_or(AuthorizationStoreError::NotFound)
    }

    async fn write_receipt(
        &self,
        workspace_id: &str,
        environment_id: &str,
        receipt: AuthorizationReceipt,
    ) -> Result<(), AuthorizationStoreError> {
        self.receipts
            .write()
            .await
            .insert(key(workspace_id, environment_id, &receipt.id), receipt);
        Ok(())
    }

    async fn get_receipt(
        &self,
        workspace_id: &str,
        environment_id: &str,
        receipt_id: &str,
    ) -> Result<AuthorizationReceipt, AuthorizationStoreError> {
        self.receipts
            .read()
            .await
            .get(&key(workspace_id, environment_id, receipt_id))
            .cloned()
            .ok_or(AuthorizationStoreError::NotFound)
    }

    async fn get_receipt_principal(
        &self,
        workspace_id: &str,
        environment_id: &str,
        receipt_id: &str,
    ) -> Result<String, AuthorizationStoreError> {
        let receipt = self
            .receipts
            .read()
            .await
            .get(&key(workspace_id, environment_id, receipt_id))
            .cloned()
            .ok_or(AuthorizationStoreError::NotFound)?;
        if let Some(principal_id) = receipt.principal_id {
            return Ok(principal_id);
        }
        let intent_id = receipt.intent_id.ok_or(AuthorizationStoreError::NotFound)?;
        self.intents
            .read()
            .await
            .get(&key(workspace_id, environment_id, &intent_id))
            .map(|intent| intent.principal_id.clone())
            .ok_or(AuthorizationStoreError::NotFound)
    }

    async fn list_receipts(
        &self,
        workspace_id: &str,
        environment_id: Option<&str>,
    ) -> Result<Vec<AuthorizationReceipt>, AuthorizationStoreError> {
        let mut receipts = self
            .receipts
            .read()
            .await
            .iter()
            .filter(|((workspace, environment, _), _)| {
                workspace == workspace_id
                    && environment_id.map_or(true, |selected| environment == selected)
            })
            .map(|(_, receipt)| receipt.clone())
            .collect::<Vec<_>>();
        receipts.sort_by(|left, right| right.created_at.cmp(&left.created_at));
        Ok(receipts)
    }
}

#[derive(Clone)]
pub struct AuthorizationState {
    pub store: Arc<dyn AuthorizationStore>,
    pub coordinator: Arc<AuthorizationCoordinator>,
    pub team_store: Arc<dyn TeamStore>,
}

#[utoipa::path(get, path = "/v1/authorization/approvals", tag = "authorization", responses((status = 200, body = AuthorizationApprovalListResponse)))]
pub async fn list_approvals(
    State(state): State<AuthorizationState>,
    user: Option<Extension<UserContext>>,
    internal: Option<Extension<InternalServiceContext>>,
    runtime_key: Option<Extension<WorkspaceKeyContext>>,
    headers: HeaderMap,
) -> Response {
    let (workspace_id, _) = match authorize_admin(
        &state,
        &headers,
        user,
        internal,
        runtime_key,
        "view authorization approvals",
    )
    .await
    {
        Ok(value) => value,
        Err(response) => return response,
    };
    let environment = header(&headers, "x-featherlane-ai-environment-id");
    match state
        .store
        .list_approvals(&workspace_id, environment.as_deref())
        .await
    {
        Ok(approvals) => Json(AuthorizationApprovalListResponse { approvals }).into_response(),
        Err(error) => store_error(error),
    }
}

#[utoipa::path(get, path = "/v1/authorization/approvals/{id}", tag = "authorization", params(("id" = String, Path, description = "Approval id")), responses((status = 200, body = AuthorizationApproval), (status = 404, body = tl_core::ApiError)))]
pub async fn get_approval(
    State(state): State<AuthorizationState>,
    Path(id): Path<String>,
    user: Option<Extension<UserContext>>,
    internal: Option<Extension<InternalServiceContext>>,
    runtime_key: Option<Extension<WorkspaceKeyContext>>,
    headers: HeaderMap,
) -> Response {
    let workspace_id = match crate::policies::workspace_id_from_headers(&headers) {
        Ok(workspace_id) => workspace_id,
        Err(response) => return response,
    };
    let environment_id = environment_id(&headers);
    match state
        .store
        .get_approval(&workspace_id, &environment_id, &id)
        .await
    {
        Ok(approval) => {
            if let Some(Extension(key)) = runtime_key {
                let principal_id = key.principal_id.as_deref().unwrap_or(&key.api_key_id);
                if key.workspace_id != workspace_id
                    || key.environment_id != environment_id
                    || principal_id != approval.envelope.principal_id
                {
                    return api_error_response(
                        StatusCode::FORBIDDEN,
                        ApiErrorCode::Forbidden,
                        "approval is outside the runtime key scope".into(),
                    );
                }
            } else if let Err(response) = authorize_admin(
                &state,
                &headers,
                user,
                internal,
                None,
                "view authorization approvals",
            )
            .await
            {
                return response;
            }
            Json(approval).into_response()
        }
        Err(error) => store_error(error),
    }
}

#[utoipa::path(post, path = "/v1/authorization/approvals/{id}/decide", tag = "authorization", params(("id" = String, Path, description = "Approval id")), request_body = DecideAuthorizationApprovalRequest, responses((status = 200, body = DecideAuthorizationApprovalResponse), (status = 409, body = tl_core::ApiError)))]
pub async fn decide_approval(
    State(state): State<AuthorizationState>,
    Path(id): Path<String>,
    user: Option<Extension<UserContext>>,
    internal: Option<Extension<InternalServiceContext>>,
    runtime_key: Option<Extension<WorkspaceKeyContext>>,
    headers: HeaderMap,
    Json(request): Json<DecideAuthorizationApprovalRequest>,
) -> Response {
    let (workspace_id, actor_id) = match authorize_admin(
        &state,
        &headers,
        user,
        internal,
        runtime_key,
        "decide authorization approvals",
    )
    .await
    {
        Ok(value) => value,
        Err(response) => return response,
    };
    let environment_id = environment_id(&headers);
    match state
        .coordinator
        .decide_approval(&workspace_id, &environment_id, &id, &actor_id, request)
        .await
    {
        Ok(response) => Json(response).into_response(),
        Err(error) => coordinator_error(error),
    }
}

#[utoipa::path(get, path = "/v1/authorization/grants", tag = "authorization", responses((status = 200, body = AuthorizationGrantListResponse)))]
pub async fn list_grants(
    State(state): State<AuthorizationState>,
    user: Option<Extension<UserContext>>,
    internal: Option<Extension<InternalServiceContext>>,
    runtime_key: Option<Extension<WorkspaceKeyContext>>,
    headers: HeaderMap,
) -> Response {
    let (workspace_id, _) = match authorize_admin(
        &state,
        &headers,
        user,
        internal,
        runtime_key,
        "view authorization grants",
    )
    .await
    {
        Ok(value) => value,
        Err(response) => return response,
    };
    let environment = header(&headers, "x-featherlane-ai-environment-id");
    match state
        .store
        .list_grants(&workspace_id, environment.as_deref())
        .await
    {
        Ok(grants) => Json(AuthorizationGrantListResponse { grants }).into_response(),
        Err(error) => store_error(error),
    }
}

#[utoipa::path(post, path = "/v1/authorization/grants", tag = "authorization", request_body = CreateAuthorizationGrantRequest, responses((status = 201, body = AuthorizationGrant), (status = 400, body = tl_core::ApiError)))]
pub async fn create_grant(
    State(state): State<AuthorizationState>,
    user: Option<Extension<UserContext>>,
    internal: Option<Extension<InternalServiceContext>>,
    runtime_key: Option<Extension<WorkspaceKeyContext>>,
    headers: HeaderMap,
    Json(request): Json<CreateAuthorizationGrantRequest>,
) -> Response {
    let (workspace_id, actor_id) = match authorize_admin(
        &state,
        &headers,
        user,
        internal,
        runtime_key,
        "create authorization grants",
    )
    .await
    {
        Ok(value) => value,
        Err(response) => return response,
    };
    let environment_id = environment_id(&headers);
    match state
        .coordinator
        .create_user_intent_grant(&workspace_id, &environment_id, &actor_id, request)
        .await
    {
        Ok(grant) => (StatusCode::CREATED, Json(grant)).into_response(),
        Err(error) => coordinator_error(error),
    }
}

#[utoipa::path(post, path = "/v1/authorization/grants/{id}/revoke", tag = "authorization", params(("id" = String, Path, description = "Grant id")), responses((status = 200, body = AuthorizationGrant), (status = 409, body = tl_core::ApiError)))]
pub async fn revoke_grant(
    State(state): State<AuthorizationState>,
    Path(id): Path<String>,
    user: Option<Extension<UserContext>>,
    internal: Option<Extension<InternalServiceContext>>,
    runtime_key: Option<Extension<WorkspaceKeyContext>>,
    headers: HeaderMap,
) -> Response {
    let (workspace_id, actor_id) = match authorize_admin(
        &state,
        &headers,
        user,
        internal,
        runtime_key,
        "revoke authorization grants",
    )
    .await
    {
        Ok(value) => value,
        Err(response) => return response,
    };
    let environment_id = environment_id(&headers);
    match state
        .coordinator
        .revoke_grant(&workspace_id, &environment_id, &id, &actor_id)
        .await
    {
        Ok(grant) => Json(grant).into_response(),
        Err(error) => coordinator_error(error),
    }
}

#[utoipa::path(post, path = "/v1/authorization/leases/{id}/complete", tag = "authorization", params(("id" = String, Path, description = "Lease id")), request_body = CompleteAuthorizationLeaseRequest, responses((status = 200, body = AuthorizationLease), (status = 409, body = tl_core::ApiError)))]
pub async fn complete_lease(
    State(state): State<AuthorizationState>,
    Path(id): Path<String>,
    user: Option<Extension<UserContext>>,
    _internal: Option<Extension<InternalServiceContext>>,
    runtime_key: Option<Extension<WorkspaceKeyContext>>,
    headers: HeaderMap,
    Json(request): Json<CompleteAuthorizationLeaseRequest>,
) -> Response {
    let workspace_id = match crate::policies::workspace_id_from_headers(&headers) {
        Ok(workspace_id) => workspace_id,
        Err(response) => return response,
    };
    let environment_id = environment_id(&headers);
    if let Some(Extension(key)) = runtime_key {
        let principal_id = key.principal_id.as_deref().unwrap_or(&key.api_key_id);
        let owner = match state
            .store
            .get_lease_principal(&workspace_id, &environment_id, &id)
            .await
        {
            Ok(owner) => owner,
            Err(error) => return store_error(error),
        };
        if key.workspace_id != workspace_id
            || key.environment_id != environment_id
            || principal_id != owner
        {
            return api_error_response(
                StatusCode::FORBIDDEN,
                ApiErrorCode::Forbidden,
                "lease is outside the runtime key scope".into(),
            );
        }
    } else if user.is_some() {
        return api_error_response(
            StatusCode::FORBIDDEN,
            ApiErrorCode::Forbidden,
            "runtime authorization is required to complete a lease".into(),
        );
    }
    match state
        .coordinator
        .complete_lease(&workspace_id, &environment_id, &id, request)
        .await
    {
        Ok(lease) => Json(lease).into_response(),
        Err(error) => coordinator_error(error),
    }
}

#[utoipa::path(get, path = "/v1/authorization/receipts", tag = "authorization", responses((status = 200, body = AuthorizationReceiptListResponse), (status = 403, body = tl_core::ApiError)))]
pub async fn list_receipts(
    State(state): State<AuthorizationState>,
    user: Option<Extension<UserContext>>,
    internal: Option<Extension<InternalServiceContext>>,
    headers: HeaderMap,
) -> Response {
    let workspace_id = match crate::policies::workspace_id_from_headers(&headers) {
        Ok(workspace_id) => workspace_id,
        Err(response) => return response,
    };
    let environment_id = environment_id(&headers);
    if let Err(response) = authorize_admin(
        &state,
        &headers,
        user,
        internal,
        None,
        "list authorization receipts",
    )
    .await
    {
        return response;
    }
    match state
        .store
        .list_receipts(&workspace_id, Some(&environment_id))
        .await
    {
        Ok(receipts) => Json(AuthorizationReceiptListResponse { receipts }).into_response(),
        Err(error) => store_error(error),
    }
}

#[utoipa::path(get, path = "/v1/authorization/receipts/{id}", tag = "authorization", params(("id" = String, Path, description = "Receipt id")), responses((status = 200, body = AuthorizationReceipt), (status = 404, body = tl_core::ApiError)))]
pub async fn get_receipt(
    State(state): State<AuthorizationState>,
    Path(id): Path<String>,
    user: Option<Extension<UserContext>>,
    internal: Option<Extension<InternalServiceContext>>,
    runtime_key: Option<Extension<WorkspaceKeyContext>>,
    headers: HeaderMap,
) -> Response {
    let workspace_id = match crate::policies::workspace_id_from_headers(&headers) {
        Ok(workspace_id) => workspace_id,
        Err(response) => return response,
    };
    let environment_id = environment_id(&headers);
    if let Some(Extension(key)) = runtime_key {
        let principal_id = key.principal_id.as_deref().unwrap_or(&key.api_key_id);
        let owner = match state
            .store
            .get_receipt_principal(&workspace_id, &environment_id, &id)
            .await
        {
            Ok(owner) => owner,
            Err(error) => return store_error(error),
        };
        if key.workspace_id != workspace_id
            || key.environment_id != environment_id
            || principal_id != owner
        {
            return api_error_response(
                StatusCode::FORBIDDEN,
                ApiErrorCode::Forbidden,
                "receipt is outside the runtime key scope".into(),
            );
        }
    } else if let Err(response) = authorize_admin(
        &state,
        &headers,
        user,
        internal,
        None,
        "view authorization receipts",
    )
    .await
    {
        return response;
    }
    match state
        .store
        .get_receipt(&workspace_id, &environment_id, &id)
        .await
    {
        Ok(receipt) => Json(receipt).into_response(),
        Err(error) => store_error(error),
    }
}

async fn authorize_admin(
    state: &AuthorizationState,
    headers: &HeaderMap,
    user: Option<Extension<UserContext>>,
    internal: Option<Extension<InternalServiceContext>>,
    runtime_key: Option<Extension<WorkspaceKeyContext>>,
    action: &str,
) -> Result<(String, String), Response> {
    crate::dashboard_admin::authorize_workspace_admin(
        &state.team_store,
        headers,
        user,
        internal,
        runtime_key,
        action,
    )
    .await
    .map(|(workspace_id, actor_id)| {
        (
            workspace_id,
            actor_id
                .map(|id| id.to_string())
                .unwrap_or_else(|| "internal-service".into()),
        )
    })
}

pub(crate) fn hash_envelope(
    envelope: &tl_core::ApprovalEnvelope,
) -> Result<String, AuthorizationStoreError> {
    let value = serde_json::to_value(envelope)
        .map_err(|error| AuthorizationStoreError::Internal(error.to_string()))?;
    let canonical = canonical_json(&value);
    let digest = Sha256::digest(canonical.as_bytes());
    Ok(format!(
        "sha256:v1:{}",
        digest
            .iter()
            .map(|byte| format!("{byte:02x}"))
            .collect::<String>()
    ))
}

fn scope_matches_domain(
    scope: &tl_core::AuthorizationGrantScope,
    domain: AuthorizationDomain,
) -> bool {
    matches!(
        (scope, domain),
        (
            tl_core::AuthorizationGrantScope::Action(_),
            AuthorizationDomain::Tool
        ) | (
            tl_core::AuthorizationGrantScope::Action(_),
            AuthorizationDomain::Content
        ) | (
            tl_core::AuthorizationGrantScope::Financial(_),
            AuthorizationDomain::Financial
        )
    )
}

fn bounded_grant_expiry(
    requested: Option<&str>,
    approval_expiry: &str,
) -> Result<String, AuthorizationStoreError> {
    let approval = DateTime::parse_from_rfc3339(approval_expiry)
        .map_err(|_| AuthorizationStoreError::Invalid("invalid approval expiry".into()))?;
    let Some(requested) = requested else {
        return Ok(approval_expiry.to_string());
    };
    let requested_at = DateTime::parse_from_rfc3339(requested)
        .map_err(|_| AuthorizationStoreError::Invalid("invalid grant expiry".into()))?;
    if requested_at > approval {
        return Err(AuthorizationStoreError::Invalid(
            "grant expiry cannot exceed the reviewed approval expiry".into(),
        ));
    }
    Ok(requested.to_string())
}

pub(crate) fn canonical_json(value: &serde_json::Value) -> String {
    match value {
        serde_json::Value::Null => "null".into(),
        serde_json::Value::Bool(value) => value.to_string(),
        serde_json::Value::Number(value) => value.to_string(),
        serde_json::Value::String(value) => {
            serde_json::to_string(value).expect("string serializes")
        }
        serde_json::Value::Array(values) => format!(
            "[{}]",
            values
                .iter()
                .map(canonical_json)
                .collect::<Vec<_>>()
                .join(",")
        ),
        serde_json::Value::Object(values) => {
            let mut entries = values.iter().collect::<Vec<_>>();
            entries.sort_by_key(|(key, _)| *key);
            format!(
                "{{{}}}",
                entries
                    .into_iter()
                    .map(|(key, value)| format!(
                        "{}:{}",
                        serde_json::to_string(key).expect("key serializes"),
                        canonical_json(value)
                    ))
                    .collect::<Vec<_>>()
                    .join(",")
            )
        }
    }
}

fn expire_approval(approval: &mut AuthorizationApproval) {
    let expired = DateTime::parse_from_rfc3339(&approval.expires_at)
        .map(|value| value < Utc::now())
        .unwrap_or(true);
    if expired && approval.status == ApprovalStatus::Pending {
        approval.status = ApprovalStatus::Expired;
        approval.updated_at = Utc::now().to_rfc3339();
    }
}

fn expire_grant(grant: &mut AuthorizationGrant) {
    let expired = grant.expires_at.as_deref().is_some_and(|expires_at| {
        DateTime::parse_from_rfc3339(expires_at)
            .map(|value| value < Utc::now())
            .unwrap_or(true)
    });
    if expired && grant.status == GrantStatus::Active {
        grant.status = GrantStatus::Expired;
        grant.updated_at = Utc::now().to_rfc3339();
    }
}

fn key(workspace_id: &str, environment_id: &str, id: &str) -> (String, String, String) {
    (workspace_id.into(), environment_id.into(), id.into())
}

fn environment_id(headers: &HeaderMap) -> String {
    header(headers, "x-featherlane-ai-environment-id").unwrap_or_else(|| "production".into())
}

fn header(headers: &HeaderMap, name: &str) -> Option<String> {
    headers
        .get(name)
        .and_then(|value| value.to_str().ok())
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(str::to_string)
}

fn store_error(error: AuthorizationStoreError) -> Response {
    match error {
        AuthorizationStoreError::NotFound => api_error_response(
            StatusCode::NOT_FOUND,
            ApiErrorCode::NotFound,
            "authorization resource was not found".into(),
        ),
        AuthorizationStoreError::Conflict(message) => {
            api_error_response(StatusCode::CONFLICT, ApiErrorCode::Invalid, message)
        }
        AuthorizationStoreError::Invalid(message) => api_error_response(
            StatusCode::UNPROCESSABLE_ENTITY,
            ApiErrorCode::Unprocessable,
            message,
        ),
        AuthorizationStoreError::Internal(message) => {
            tracing::error!(error = %message, "authorization store failed");
            api_error_response(
                StatusCode::INTERNAL_SERVER_ERROR,
                ApiErrorCode::Internal,
                "authorization storage failed".into(),
            )
        }
    }
}

fn coordinator_error(error: AuthorizationError) -> Response {
    match error {
        AuthorizationError::Store(error) => store_error(error),
        AuthorizationError::Invalid(message)
        | AuthorizationError::Adapter(adapters::AuthorizationAdapterError::Invalid(message)) => {
            api_error_response(
                StatusCode::UNPROCESSABLE_ENTITY,
                ApiErrorCode::Unprocessable,
                message,
            )
        }
        AuthorizationError::Conflict(message) => {
            api_error_response(StatusCode::CONFLICT, ApiErrorCode::Invalid, message)
        }
        AuthorizationError::Adapter(error) => api_error_response(
            StatusCode::UNPROCESSABLE_ENTITY,
            ApiErrorCode::Unprocessable,
            error.to_string(),
        ),
        AuthorizationError::Policy(message) => {
            tracing::error!(error = %message, "authorization policy resolution failed");
            api_error_response(
                StatusCode::INTERNAL_SERVER_ERROR,
                ApiErrorCode::Internal,
                "authorization policy resolution failed".into(),
            )
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tl_core::{
        ActionGrantScope, ApprovalEnvelope, AuthorizationCapabilityId, AuthorizationDomainEvidence,
        AuthorizationGrantScope, AuthorizationReceipt, SideEffectClass,
    };

    fn authorization_state(store: Arc<MemoryAuthorizationStore>) -> AuthorizationState {
        let policy_store: Arc<dyn crate::policies::PolicyStore> =
            Arc::new(crate::policies::MemoryPolicyStore::new());
        AuthorizationState {
            store: store.clone(),
            coordinator: Arc::new(AuthorizationCoordinator::new(
                store,
                policy_store,
                Arc::new(adapters::AuthorizationAdapterRegistry::new()),
            )),
            team_store: Arc::new(crate::team::MemoryTeamStore::new()),
        }
    }

    fn runtime_key(principal_id: &str) -> Extension<WorkspaceKeyContext> {
        Extension(WorkspaceKeyContext {
            api_key_id: "key-1".into(),
            workspace_id: "ws".into(),
            environment_id: "production".into(),
            principal_id: Some(principal_id.into()),
        })
    }

    fn scoped_headers() -> HeaderMap {
        let mut headers = HeaderMap::new();
        headers.insert("x-featherlane-ai-workspace-id", "ws".parse().unwrap());
        headers.insert(
            "x-featherlane-ai-environment-id",
            "production".parse().unwrap(),
        );
        headers
    }

    fn envelope() -> ApprovalEnvelope {
        ApprovalEnvelope {
            schema: "authorization-envelope:v1".into(),
            intent_id: Uuid::now_v7().to_string(),
            domain: AuthorizationDomain::Tool,
            capability: AuthorizationCapabilityId::parse("tool:mail/send").unwrap(),
            principal_id: "agent-1".into(),
            subject_id: "invocation-1".into(),
            subject_hash: "sha256:v1:subject".into(),
            exact_fingerprint: "sha256:v1:exact".into(),
            fingerprint_version: 1,
            requirement_ids: vec!["approval:mail/send".into()],
            proposed_scope: Some(AuthorizationGrantScope::Action(ActionGrantScope {
                operations: vec!["mail/send".into()],
                side_effects: vec![SideEffectClass::ExternalCommunication],
                server_id: Some("mail".into()),
                tool_name: Some("send".into()),
                schema_hash: Some("sha256:v1:schema".into()),
                parameters: Some(serde_json::json!({"to": "a@example.com"})),
                allowed_destinations: vec!["a@example.com".into()],
                maximum_data_confidentiality: None,
                minimum_source_trust: None,
            })),
            policy_versions: vec![],
            issued_at: Utc::now().to_rfc3339(),
            expires_at: (Utc::now() + Duration::minutes(15)).to_rfc3339(),
        }
    }

    fn intent(id: &str, fingerprint: &str) -> NewAuthorizationIntent {
        NewAuthorizationIntent {
            workspace_id: "ws".into(),
            environment_id: "production".into(),
            id: id.into(),
            domain: AuthorizationDomain::Tool,
            subject_id: "invocation-1".into(),
            idempotency_key: "invocation-1".into(),
            principal_id: "agent-1".into(),
            operation: "mail/send".into(),
            fingerprint: fingerprint.into(),
            fingerprint_version: 1,
            subject_snapshot: serde_json::json!({"to": "a@example.com"}),
            expires_at: None,
        }
    }

    fn grant_request(max_uses: Option<u32>) -> CreateAuthorizationGrantRequest {
        CreateAuthorizationGrantRequest {
            principal_id: "agent-1".into(),
            domain: AuthorizationDomain::Tool,
            capability: AuthorizationCapabilityId::parse("tool:mail/send").unwrap(),
            scope: AuthorizationGrantScope::Action(ActionGrantScope {
                operations: vec!["mail/send".into()],
                side_effects: vec![SideEffectClass::ExternalCommunication],
                server_id: Some("mail".into()),
                tool_name: Some("send".into()),
                schema_hash: Some("sha256:v1:schema".into()),
                parameters: Some(serde_json::json!({"to": "a@example.com"})),
                allowed_destinations: vec!["a@example.com".into()],
                maximum_data_confidentiality: None,
                minimum_source_trust: None,
            }),
            requirement_ids: vec!["approval:mail/send".into()],
            max_uses,
            starts_at: None,
            expires_at: None,
        }
    }

    #[tokio::test]
    async fn durable_intent_is_idempotent_but_rejects_subject_mutation() {
        let store = MemoryAuthorizationStore::new();
        let id = Uuid::now_v7().to_string();

        assert_eq!(
            store
                .create_or_get_intent(intent(&id, "sha256:v1:one"))
                .await
                .unwrap(),
            id
        );
        assert_eq!(
            store
                .create_or_get_intent(intent(&id, "sha256:v1:one"))
                .await
                .unwrap(),
            id
        );
        assert!(matches!(
            store
                .create_or_get_intent(intent(&id, "sha256:v1:changed"))
                .await,
            Err(AuthorizationStoreError::Conflict(_))
        ));
    }

    #[tokio::test]
    async fn lease_retry_is_stable_and_consumes_a_bounded_grant_once() {
        let store = MemoryAuthorizationStore::new();
        let intent_id = Uuid::now_v7().to_string();
        store
            .create_or_get_intent(intent(&intent_id, "sha256:v1:subject"))
            .await
            .unwrap();
        let grant = store
            .create_grant("ws", "production", "user-1", grant_request(Some(1)))
            .await
            .unwrap();

        let first = store
            .claim_lease(
                "ws",
                "production",
                &intent_id,
                Some(&grant.id),
                "attempt-1",
                "sha256:v1:subject",
            )
            .await
            .unwrap();
        let retry = store
            .claim_lease(
                "ws",
                "production",
                &intent_id,
                Some(&grant.id),
                "attempt-1",
                "sha256:v1:subject",
            )
            .await
            .unwrap();

        assert_eq!(retry.id, first.id);
        let exhausted = store
            .get_grant("ws", "production", &grant.id)
            .await
            .unwrap();
        assert_eq!(exhausted.use_count, 1);
        assert_eq!(exhausted.status, GrantStatus::Exhausted);
        assert!(matches!(
            store
                .claim_lease(
                    "ws",
                    "production",
                    &intent_id,
                    Some(&grant.id),
                    "attempt-2",
                    "sha256:v1:subject",
                )
                .await,
            Err(AuthorizationStoreError::Conflict(_))
        ));
    }

    #[tokio::test]
    async fn lease_retry_rejects_changed_authority_or_fingerprint() {
        let store = MemoryAuthorizationStore::new();
        let intent_id = Uuid::now_v7().to_string();
        store
            .create_or_get_intent(intent(&intent_id, "sha256:v1:subject"))
            .await
            .unwrap();
        let grant = store
            .create_grant("ws", "production", "user-1", grant_request(Some(2)))
            .await
            .unwrap();
        store
            .claim_lease(
                "ws",
                "production",
                &intent_id,
                Some(&grant.id),
                "attempt-1",
                "sha256:v1:subject",
            )
            .await
            .unwrap();

        assert!(matches!(
            store
                .claim_lease(
                    "ws",
                    "production",
                    &intent_id,
                    None,
                    "attempt-1",
                    "sha256:v1:subject",
                )
                .await,
            Err(AuthorizationStoreError::Conflict(_))
        ));
        assert!(matches!(
            store
                .claim_lease(
                    "ws",
                    "production",
                    &intent_id,
                    Some(&grant.id),
                    "attempt-1",
                    "sha256:v1:changed",
                )
                .await,
            Err(AuthorizationStoreError::Conflict(_))
        ));
    }

    #[tokio::test]
    async fn runtime_reads_and_lease_completion_are_principal_scoped() {
        let store = Arc::new(MemoryAuthorizationStore::new());
        let state = authorization_state(store.clone());
        let intent_id = Uuid::now_v7().to_string();
        store
            .create_or_get_intent(intent(&intent_id, "sha256:v1:subject"))
            .await
            .unwrap();
        let mut reviewed = envelope();
        reviewed.intent_id = intent_id.clone();
        let approval = store
            .create_or_get_approval(NewAuthorizationApproval {
                workspace_id: "ws".into(),
                environment_id: "production".into(),
                envelope: reviewed,
                approver_roles: vec!["admin".into()],
            })
            .await
            .unwrap();
        let lease = store
            .claim_lease(
                "ws",
                "production",
                &intent_id,
                None,
                "attempt-1",
                "sha256:v1:subject",
            )
            .await
            .unwrap();
        let receipt = AuthorizationReceipt {
            id: Uuid::now_v7().to_string(),
            intent_id: Some(intent_id),
            trace_id: Some("trace-1".into()),
            principal_id: Some("principal".into()),
            operation: Some("tool:test".into()),
            run_id: None,
            domain: AuthorizationDomain::Tool,
            effect: tl_core::AuthorizationEffect::Permit,
            intent_status: Some(tl_core::AuthorizationIntentStatus::Authorized),
            subject_hash: "sha256:v1:subject".into(),
            reason: "authorized".into(),
            findings: vec![],
            policy_versions: vec![],
            approval_id: Some(approval.id.clone()),
            grant_id: None,
            lease_id: Some(lease.id.clone()),
            domain_evidence: AuthorizationDomainEvidence::Tool(serde_json::json!({})),
            created_at: Utc::now().to_rfc3339(),
        };
        store
            .write_receipt("ws", "production", receipt.clone())
            .await
            .unwrap();

        let approval_response = get_approval(
            State(state.clone()),
            Path(approval.id),
            None,
            None,
            Some(runtime_key("agent-2")),
            scoped_headers(),
        )
        .await;
        assert_eq!(approval_response.status(), StatusCode::FORBIDDEN);

        let receipt_response = get_receipt(
            State(state.clone()),
            Path(receipt.id),
            None,
            None,
            Some(runtime_key("agent-2")),
            scoped_headers(),
        )
        .await;
        assert_eq!(receipt_response.status(), StatusCode::FORBIDDEN);

        let completion_response = complete_lease(
            State(state),
            Path(lease.id),
            None,
            None,
            Some(runtime_key("agent-2")),
            scoped_headers(),
            Json(CompleteAuthorizationLeaseRequest {
                status: LeaseStatus::Consumed,
                outcome: serde_json::json!({}),
            }),
        )
        .await;
        assert_eq!(completion_response.status(), StatusCode::FORBIDDEN);
    }

    #[tokio::test]
    async fn decision_hash_is_bound_and_mints_one_grant() {
        let store = MemoryAuthorizationStore::new();
        let approval = store
            .create_or_get_approval(NewAuthorizationApproval {
                workspace_id: "ws".into(),
                environment_id: "production".into(),
                envelope: envelope(),
                approver_roles: vec!["admin".into()],
            })
            .await
            .unwrap();
        let decided = store
            .decide_approval(
                "ws",
                "production",
                &approval.id,
                "user-1",
                DecideAuthorizationApprovalRequest {
                    decision: ApprovalDecision::Approve,
                    mode: GrantMode::ExactOnce,
                    envelope_hash: approval.envelope_hash,
                    scope: None,
                    starts_at: None,
                    expires_at: None,
                    reason: None,
                },
            )
            .await
            .unwrap();
        assert_eq!(decided.approval.status, ApprovalStatus::Approved);
        assert_eq!(decided.grant.unwrap().max_uses, Some(1));
    }
}
