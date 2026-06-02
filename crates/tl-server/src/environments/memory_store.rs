use std::collections::HashMap;

use async_trait::async_trait;
use tl_core::{
    CreateWorkspaceEnvironmentRequest, UpdateWorkspaceEnvironmentRequest, WorkspaceEnvironment,
    DEFAULT_ENVIRONMENT_ID,
};
use tokio::sync::RwLock;

use super::{
    validation::{clean_optional, validate_name, validate_slug},
    EnvironmentStore, EnvironmentStoreError,
};

#[derive(Debug, Default)]
pub struct MemoryEnvironmentStore {
    environments: RwLock<HashMap<(String, String), WorkspaceEnvironment>>,
}

impl MemoryEnvironmentStore {
    pub fn new() -> Self {
        Self::default()
    }
}

#[async_trait]
impl EnvironmentStore for MemoryEnvironmentStore {
    async fn list(
        &self,
        workspace_id: &str,
    ) -> Result<Vec<WorkspaceEnvironment>, EnvironmentStoreError> {
        ensure_default(&self.environments, workspace_id).await;
        let workspace = workspace_id.to_string();
        let mut rows = self
            .environments
            .read()
            .await
            .iter()
            .filter(|((ws, _), _)| ws == &workspace)
            .map(|(_, env)| env.clone())
            .collect::<Vec<_>>();
        rows.sort_by(|a, b| {
            b.is_default
                .cmp(&a.is_default)
                .then_with(|| a.slug.cmp(&b.slug))
        });
        Ok(rows)
    }

    async fn get(
        &self,
        workspace_id: &str,
        environment_id: &str,
    ) -> Result<WorkspaceEnvironment, EnvironmentStoreError> {
        ensure_default(&self.environments, workspace_id).await;
        self.environments
            .read()
            .await
            .get(&(workspace_id.to_string(), environment_id.to_string()))
            .cloned()
            .ok_or(EnvironmentStoreError::NotFound)
    }

    async fn default_environment_id(
        &self,
        workspace_id: &str,
    ) -> Result<String, EnvironmentStoreError> {
        ensure_default(&self.environments, workspace_id).await;
        self.environments
            .read()
            .await
            .iter()
            .find(|((ws, _), env)| ws == workspace_id && env.is_default)
            .map(|(_, env)| env.id.clone())
            .ok_or(EnvironmentStoreError::NotFound)
    }

    async fn create(
        &self,
        workspace_id: &str,
        input: CreateWorkspaceEnvironmentRequest,
    ) -> Result<WorkspaceEnvironment, EnvironmentStoreError> {
        validate_slug(&input.slug)?;
        validate_name(&input.name)?;
        let now = chrono::Utc::now().to_rfc3339();
        let id = if input.slug == DEFAULT_ENVIRONMENT_ID {
            DEFAULT_ENVIRONMENT_ID.to_string()
        } else {
            format!("env_{}", uuid::Uuid::now_v7())
        };
        let env = WorkspaceEnvironment {
            id: id.clone(),
            slug: input.slug,
            name: input.name.trim().to_string(),
            description: input.description.and_then(clean_optional),
            is_default: input.is_default,
            created_at: now.clone(),
            updated_at: now,
        };
        let mut guard = self.environments.write().await;
        if guard.iter().any(|((ws, _), existing)| {
            ws == workspace_id && existing.slug == env.slug && existing.id != id
        }) {
            return Err(EnvironmentStoreError::Validation(
                "environment slug already exists".into(),
            ));
        }
        if env.is_default {
            for ((ws, _), existing) in guard.iter_mut() {
                if ws == workspace_id {
                    existing.is_default = false;
                }
            }
        }
        guard.insert((workspace_id.to_string(), id), env.clone());
        Ok(env)
    }

    async fn update(
        &self,
        workspace_id: &str,
        environment_id: &str,
        input: UpdateWorkspaceEnvironmentRequest,
    ) -> Result<WorkspaceEnvironment, EnvironmentStoreError> {
        let mut guard = self.environments.write().await;
        let exists = guard.contains_key(&(workspace_id.to_string(), environment_id.to_string()));
        if !exists {
            return Err(EnvironmentStoreError::NotFound);
        }
        if input.is_default == Some(true) {
            for ((ws, _), existing) in guard.iter_mut() {
                if ws == workspace_id {
                    existing.is_default = false;
                }
            }
        }
        let env = guard
            .get_mut(&(workspace_id.to_string(), environment_id.to_string()))
            .ok_or(EnvironmentStoreError::NotFound)?;
        if let Some(slug) = input.slug {
            validate_slug(&slug)?;
            env.slug = slug;
        }
        if let Some(name) = input.name {
            validate_name(&name)?;
            env.name = name.trim().to_string();
        }
        if let Some(description) = input.description {
            env.description = clean_optional(description);
        }
        if let Some(is_default) = input.is_default {
            if !is_default && env.is_default {
                return Err(EnvironmentStoreError::Validation(
                    "workspace must have one default environment".into(),
                ));
            }
            env.is_default = is_default;
        }
        env.updated_at = chrono::Utc::now().to_rfc3339();
        Ok(env.clone())
    }

    async fn delete(
        &self,
        workspace_id: &str,
        environment_id: &str,
    ) -> Result<(), EnvironmentStoreError> {
        let mut guard = self.environments.write().await;
        if guard
            .get(&(workspace_id.to_string(), environment_id.to_string()))
            .map(|env| env.is_default)
            .unwrap_or(false)
        {
            return Err(EnvironmentStoreError::Validation(
                "default environment cannot be deleted".into(),
            ));
        }
        let removed = guard.remove(&(workspace_id.to_string(), environment_id.to_string()));
        removed.map(|_| ()).ok_or(EnvironmentStoreError::NotFound)
    }
}

async fn ensure_default(
    environments: &RwLock<HashMap<(String, String), WorkspaceEnvironment>>,
    workspace_id: &str,
) {
    let key = (workspace_id.to_string(), DEFAULT_ENVIRONMENT_ID.to_string());
    if environments.read().await.contains_key(&key) {
        return;
    }

    let now = chrono::Utc::now().to_rfc3339();
    environments.write().await.insert(
        key,
        WorkspaceEnvironment {
            id: DEFAULT_ENVIRONMENT_ID.to_string(),
            slug: DEFAULT_ENVIRONMENT_ID.to_string(),
            name: "Production".to_string(),
            description: None,
            is_default: true,
            created_at: now.clone(),
            updated_at: now,
        },
    );
}
