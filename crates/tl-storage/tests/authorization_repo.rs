//! Postgres invariants for the unified authorization lifecycle.
#![cfg(feature = "postgres-it")]

use chrono::{Duration, Utc};
use diesel::prelude::*;
use diesel_async::RunQueryDsl;
use testcontainers::runners::AsyncRunner;
use testcontainers_modules::postgres::Postgres as PostgresImage;
use tl_core::{
    ActionGrantScope, ApprovalDecision, ApprovalEnvelope, AuthorizationCapabilityId,
    AuthorizationDomain, AuthorizationGrantScope, DecideAuthorizationApprovalRequest, GrantMode,
    GrantStatus, SideEffectClass,
};
use tl_storage::{
    connect_postgres, migrate_postgres,
    schema::{organizations, workspace_environments, workspaces},
    AuthorizationRepo, CreateAuthorizationApproval, CreateAuthorizationIntent, DbPool,
    StorageError,
};
use uuid::Uuid;

async fn fresh_pool() -> (DbPool, testcontainers::ContainerAsync<PostgresImage>) {
    let container = PostgresImage::default().start().await.unwrap();
    let host = container.get_host().await.unwrap();
    let port = container.get_host_port_ipv4(5432).await.unwrap();
    let url = format!("postgres://postgres:postgres@{host}:{port}/postgres");
    migrate_postgres(&url).await.unwrap();
    let pool = connect_postgres(&url, 8).await.unwrap();
    seed_workspace(&pool).await;
    (pool, container)
}

async fn seed_workspace(pool: &DbPool) {
    let mut conn = pool.get().await.unwrap();
    diesel::insert_into(organizations::table)
        .values((
            organizations::id.eq("org-1"),
            organizations::name.eq("Org 1"),
            organizations::slug.eq("org-1"),
        ))
        .execute(&mut conn)
        .await
        .unwrap();
    diesel::insert_into(workspaces::table)
        .values((
            workspaces::id.eq("workspace-1"),
            workspaces::organization_id.eq("org-1"),
            workspaces::name.eq("Workspace 1"),
            workspaces::slug.eq("workspace-1"),
        ))
        .execute(&mut conn)
        .await
        .unwrap();
    for (id, is_default) in [("production", true), ("staging", false)] {
        diesel::insert_into(workspace_environments::table)
            .values((
                workspace_environments::workspace_id.eq("workspace-1"),
                workspace_environments::id.eq(id),
                workspace_environments::slug.eq(id),
                workspace_environments::name.eq(id),
                workspace_environments::is_default.eq(is_default),
            ))
            .execute(&mut conn)
            .await
            .unwrap();
    }
}

fn intent(id: Uuid, fingerprint: &str) -> CreateAuthorizationIntent {
    CreateAuthorizationIntent {
        workspace_id: "workspace-1".into(),
        environment_id: "production".into(),
        id: id.to_string(),
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

fn envelope(intent_id: Uuid) -> ApprovalEnvelope {
    ApprovalEnvelope {
        schema: "authorization-envelope:v1".into(),
        intent_id: intent_id.to_string(),
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
        policy_versions: vec!["mail-policy:v1".into()],
        issued_at: Utc::now().to_rfc3339(),
        expires_at: (Utc::now() + Duration::minutes(15)).to_rfc3339(),
    }
}

#[tokio::test]
async fn intents_are_idempotent_scoped_and_immutable() {
    let (pool, _container) = fresh_pool().await;
    let repo = AuthorizationRepo::new(pool);
    let id = Uuid::now_v7();

    assert_eq!(
        repo.create_or_get_intent(intent(id, "sha256:v1:one"))
            .await
            .unwrap(),
        id.to_string()
    );
    assert_eq!(
        repo.create_or_get_intent(intent(id, "sha256:v1:one"))
            .await
            .unwrap(),
        id.to_string()
    );
    assert!(matches!(
        repo.create_or_get_intent(intent(id, "sha256:v1:changed"))
            .await,
        Err(StorageError::Conflict)
    ));
    assert!(matches!(
        repo.record_decision(
            "workspace-1",
            "staging",
            &id.to_string(),
            tl_core::AuthorizationEffect::Permit,
            tl_core::AuthorizationIntentStatus::Authorized,
            "ok",
            "trace-1",
        )
        .await,
        Err(StorageError::NotFound)
    ));
}

#[tokio::test]
async fn reviewer_signoff_mints_one_hash_bound_grant_and_lease_retry_consumes_once() {
    let (pool, _container) = fresh_pool().await;
    let repo = AuthorizationRepo::new(pool);
    let intent_id = Uuid::now_v7();
    let approval = repo
        .create_or_get_approval(CreateAuthorizationApproval {
            workspace_id: "workspace-1".into(),
            environment_id: "production".into(),
            envelope: envelope(intent_id),
            envelope_hash: "sha256:v1:reviewed".into(),
            approver_roles: vec!["admin".into()],
        })
        .await
        .unwrap();

    assert!(matches!(
        repo.decide_approval(
            "workspace-1",
            "production",
            &approval.id,
            "reviewer-1",
            DecideAuthorizationApprovalRequest {
                decision: ApprovalDecision::Approve,
                mode: GrantMode::ExactOnce,
                envelope_hash: "sha256:v1:changed".into(),
                scope: None,
                starts_at: None,
                expires_at: None,
                reason: None,
            },
        )
        .await,
        Err(StorageError::Conflict)
    ));

    let decided = repo
        .decide_approval(
            "workspace-1",
            "production",
            &approval.id,
            "reviewer-1",
            DecideAuthorizationApprovalRequest {
                decision: ApprovalDecision::Approve,
                mode: GrantMode::ExactOnce,
                envelope_hash: approval.envelope_hash,
                scope: None,
                starts_at: None,
                expires_at: None,
                reason: Some("reviewed".into()),
            },
        )
        .await
        .unwrap();
    let grant = decided.grant.unwrap();
    assert_eq!(grant.max_uses, Some(1));

    let first = repo
        .claim_lease(
            "workspace-1",
            "production",
            &intent_id.to_string(),
            Some(&grant.id),
            "attempt-1",
            "sha256:v1:exact",
        )
        .await
        .unwrap();
    let retry = repo
        .claim_lease(
            "workspace-1",
            "production",
            &intent_id.to_string(),
            Some(&grant.id),
            "attempt-1",
            "sha256:v1:exact",
        )
        .await
        .unwrap();
    assert_eq!(first.id, retry.id);
    let exhausted = repo
        .get_grant("workspace-1", "production", &grant.id)
        .await
        .unwrap();
    assert_eq!(exhausted.use_count, 1);
    assert_eq!(exhausted.status, GrantStatus::Exhausted);
    assert!(matches!(
        repo.claim_lease(
            "workspace-1",
            "production",
            &intent_id.to_string(),
            Some(&grant.id),
            "attempt-2",
            "sha256:v1:exact",
        )
        .await,
        Err(StorageError::Conflict)
    ));
}
