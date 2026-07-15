//! Postgres projection tests for the unified financial action lifecycle.
#![cfg(feature = "postgres-it")]

use diesel::prelude::*;
use diesel_async::RunQueryDsl;
use testcontainers::runners::AsyncRunner;
use testcontainers_modules::postgres::Postgres as PostgresImage;
use tl_core::{
    CreateFinancialActionRequest, FinancialAction, FinancialActionKind, FinancialExecutionStatus,
    FinancialRail, MoneyAmount,
};
use tl_storage::{
    connect_postgres, migrate_postgres,
    schema::{organizations, workspace_environments, workspaces},
    DbPool, FinancialRepo, StorageError,
};

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

fn request(key: &str) -> CreateFinancialActionRequest {
    CreateFinancialActionRequest {
        idempotency_key: key.into(),
        execute: false,
        authorization: None,
        action: FinancialAction {
            id: None,
            kind: FinancialActionKind::Payment,
            operation: "pay".into(),
            principal_id: "agent-1".into(),
            amount: MoneyAmount {
                amount_minor: 100,
                currency: "USD".into(),
            },
            counterparty: None,
            rail: FinancialRail::Internal,
            memo: None,
            metadata: serde_json::json!({}),
        },
        evidence: Vec::new(),
    }
}

#[tokio::test]
async fn idempotency_is_scoped_by_environment() {
    let (pool, _container) = fresh_pool().await;
    let repo = FinancialRepo::new(pool);
    let first = repo
        .create_action("workspace-1", "production", request("same"))
        .await
        .unwrap();
    let duplicate = repo
        .create_action("workspace-1", "production", request("same"))
        .await
        .unwrap();
    let staging = repo
        .create_action("workspace-1", "staging", request("same"))
        .await
        .unwrap();

    assert_eq!(first.id, duplicate.id);
    assert_ne!(first.id, staging.id);
    assert_eq!(first.execution_status, FinancialExecutionStatus::NotStarted);
}

#[tokio::test]
async fn execution_transition_is_environment_scoped() {
    let (pool, _container) = fresh_pool().await;
    let repo = FinancialRepo::new(pool);
    let action = repo
        .create_action("workspace-1", "production", request("execute"))
        .await
        .unwrap();
    let executing = repo
        .transition_execution(
            "workspace-1",
            "production",
            &action.id,
            FinancialExecutionStatus::Executing,
            None,
        )
        .await
        .unwrap();
    assert_eq!(
        executing.execution_status,
        FinancialExecutionStatus::Executing
    );

    assert!(matches!(
        repo.get_action("workspace-1", "staging", &action.id).await,
        Err(StorageError::NotFound)
    ));
}
