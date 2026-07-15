use std::sync::Arc;

use async_trait::async_trait;
use tl_storage::{
    AuthorizationRepo, CreateAuthorizationApproval as StoredAuthorizationApproval,
    CreateAuthorizationIntent as StoredAuthorizationIntent, StorageError,
};

use crate::authorization::{
    hash_envelope, AuthorizationStore, AuthorizationStoreError, NewAuthorizationApproval,
    NewAuthorizationIntent,
};

pub struct PostgresAuthorizationAdapter(pub Arc<AuthorizationRepo>);

impl PostgresAuthorizationAdapter {
    pub fn new(repo: Arc<AuthorizationRepo>) -> Arc<Self> {
        Arc::new(Self(repo))
    }
}

#[async_trait]
impl AuthorizationStore for PostgresAuthorizationAdapter {
    async fn create_or_get_intent(
        &self,
        input: NewAuthorizationIntent,
    ) -> Result<String, AuthorizationStoreError> {
        self.0
            .create_or_get_intent(StoredAuthorizationIntent {
                workspace_id: input.workspace_id,
                environment_id: input.environment_id,
                id: input.id,
                domain: input.domain,
                subject_id: input.subject_id,
                idempotency_key: input.idempotency_key,
                principal_id: input.principal_id,
                operation: input.operation,
                fingerprint: input.fingerprint,
                fingerprint_version: input.fingerprint_version,
                subject_snapshot: input.subject_snapshot,
                expires_at: input.expires_at,
            })
            .await
            .map_err(authorization_store_error)
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
        self.0
            .record_decision(
                workspace_id,
                environment_id,
                intent_id,
                effect,
                status,
                reason,
                trace_id,
            )
            .await
            .map_err(authorization_store_error)
    }

    async fn create_or_get_approval(
        &self,
        input: NewAuthorizationApproval,
    ) -> Result<tl_core::AuthorizationApproval, AuthorizationStoreError> {
        let envelope_hash = hash_envelope(&input.envelope)?;
        self.0
            .create_or_get_approval(StoredAuthorizationApproval {
                workspace_id: input.workspace_id,
                environment_id: input.environment_id,
                envelope: input.envelope,
                envelope_hash,
                approver_roles: input.approver_roles,
            })
            .await
            .map_err(authorization_store_error)
    }

    async fn get_approval(
        &self,
        workspace_id: &str,
        environment_id: &str,
        approval_id: &str,
    ) -> Result<tl_core::AuthorizationApproval, AuthorizationStoreError> {
        self.0
            .get_approval(workspace_id, environment_id, approval_id)
            .await
            .map_err(authorization_store_error)
    }

    async fn list_approvals(
        &self,
        workspace_id: &str,
        environment_id: Option<&str>,
    ) -> Result<Vec<tl_core::AuthorizationApproval>, AuthorizationStoreError> {
        self.0
            .list_approvals(workspace_id, environment_id)
            .await
            .map_err(authorization_store_error)
    }

    async fn decide_approval(
        &self,
        workspace_id: &str,
        environment_id: &str,
        approval_id: &str,
        actor_id: &str,
        request: tl_core::DecideAuthorizationApprovalRequest,
    ) -> Result<tl_core::DecideAuthorizationApprovalResponse, AuthorizationStoreError> {
        let approval = self
            .0
            .get_approval(workspace_id, environment_id, approval_id)
            .await
            .map_err(authorization_store_error)?;
        if request.envelope_hash != hash_envelope(&approval.envelope)? {
            return Err(AuthorizationStoreError::Conflict(
                "approval envelope changed; refresh before deciding".into(),
            ));
        }
        self.0
            .decide_approval(workspace_id, environment_id, approval_id, actor_id, request)
            .await
            .map_err(authorization_store_error)
    }

    async fn create_grant(
        &self,
        workspace_id: &str,
        environment_id: &str,
        actor_id: &str,
        request: tl_core::CreateAuthorizationGrantRequest,
    ) -> Result<tl_core::AuthorizationGrant, AuthorizationStoreError> {
        self.0
            .create_grant(workspace_id, environment_id, actor_id, request)
            .await
            .map_err(authorization_store_error)
    }

    async fn get_grant(
        &self,
        workspace_id: &str,
        environment_id: &str,
        grant_id: &str,
    ) -> Result<tl_core::AuthorizationGrant, AuthorizationStoreError> {
        self.0
            .get_grant(workspace_id, environment_id, grant_id)
            .await
            .map_err(authorization_store_error)
    }

    async fn list_grants(
        &self,
        workspace_id: &str,
        environment_id: Option<&str>,
    ) -> Result<Vec<tl_core::AuthorizationGrant>, AuthorizationStoreError> {
        self.0
            .list_grants(workspace_id, environment_id)
            .await
            .map_err(authorization_store_error)
    }

    async fn revoke_grant(
        &self,
        workspace_id: &str,
        environment_id: &str,
        grant_id: &str,
        actor_id: &str,
    ) -> Result<tl_core::AuthorizationGrant, AuthorizationStoreError> {
        self.0
            .revoke_grant(workspace_id, environment_id, grant_id, actor_id)
            .await
            .map_err(authorization_store_error)
    }

    async fn claim_lease(
        &self,
        workspace_id: &str,
        environment_id: &str,
        intent_id: &str,
        grant_id: Option<&str>,
        attempt_id: &str,
        fingerprint: &str,
    ) -> Result<tl_core::AuthorizationLease, AuthorizationStoreError> {
        self.0
            .claim_lease(
                workspace_id,
                environment_id,
                intent_id,
                grant_id,
                attempt_id,
                fingerprint,
            )
            .await
            .map_err(authorization_store_error)
    }

    async fn get_lease_by_attempt(
        &self,
        workspace_id: &str,
        environment_id: &str,
        intent_id: &str,
        attempt_id: &str,
    ) -> Result<Option<tl_core::AuthorizationLease>, AuthorizationStoreError> {
        self.0
            .get_lease_by_attempt(workspace_id, environment_id, intent_id, attempt_id)
            .await
            .map_err(authorization_store_error)
    }

    async fn complete_lease(
        &self,
        workspace_id: &str,
        environment_id: &str,
        lease_id: &str,
        request: tl_core::CompleteAuthorizationLeaseRequest,
    ) -> Result<tl_core::AuthorizationLease, AuthorizationStoreError> {
        self.0
            .complete_lease(workspace_id, environment_id, lease_id, request)
            .await
            .map_err(authorization_store_error)
    }

    async fn get_lease_principal(
        &self,
        workspace_id: &str,
        environment_id: &str,
        lease_id: &str,
    ) -> Result<String, AuthorizationStoreError> {
        self.0
            .get_lease_principal(workspace_id, environment_id, lease_id)
            .await
            .map_err(authorization_store_error)
    }

    async fn write_receipt(
        &self,
        workspace_id: &str,
        environment_id: &str,
        receipt: tl_core::AuthorizationReceipt,
    ) -> Result<(), AuthorizationStoreError> {
        self.0
            .write_receipt(workspace_id, environment_id, receipt)
            .await
            .map_err(authorization_store_error)
    }

    async fn get_receipt(
        &self,
        workspace_id: &str,
        environment_id: &str,
        receipt_id: &str,
    ) -> Result<tl_core::AuthorizationReceipt, AuthorizationStoreError> {
        self.0
            .get_receipt(workspace_id, environment_id, receipt_id)
            .await
            .map_err(authorization_store_error)
    }

    async fn get_receipt_principal(
        &self,
        workspace_id: &str,
        environment_id: &str,
        receipt_id: &str,
    ) -> Result<String, AuthorizationStoreError> {
        self.0
            .get_receipt_principal(workspace_id, environment_id, receipt_id)
            .await
            .map_err(authorization_store_error)
    }

    async fn list_receipts(
        &self,
        workspace_id: &str,
        environment_id: Option<&str>,
    ) -> Result<Vec<tl_core::AuthorizationReceipt>, AuthorizationStoreError> {
        self.0
            .list_receipts(workspace_id, environment_id)
            .await
            .map_err(authorization_store_error)
    }
}

fn authorization_store_error(error: StorageError) -> AuthorizationStoreError {
    match error {
        StorageError::NotFound => AuthorizationStoreError::NotFound,
        StorageError::Conflict => AuthorizationStoreError::Conflict("state conflict".into()),
        StorageError::Internal(message) => AuthorizationStoreError::Internal(message),
    }
}
