//! Policy authoring endpoints.

use std::sync::Arc;

use async_trait::async_trait;
use tl_core::{EntityVersionDetail, EntityVersionListResponse, PolicyDocument, PolicySummary};
use tl_llm::LlmClient;
use tl_policy::{FamilyPolicy, Policy};

use crate::environments::EnvironmentStore;
use crate::team::TeamStore;

mod ai_edit;
pub(crate) mod authoring;
mod context;
mod draft;
pub(crate) mod draft_handler;
pub(crate) mod guardrails;
mod mapping;
mod memory_store;
mod response;
#[cfg(test)]
mod tests;
mod validation;
mod versions;
pub use ai_edit::ai_edit_policy;
pub use authoring::{
    batch_set_policy_enabled, delete_policy, get_policy, list_policies, set_policy_enabled,
    upsert_policy, validate_policy,
};
pub(crate) use context::workspace_id_from_headers;
pub use draft_handler::draft_policy;
pub use guardrails::{generate_guardrails, list_guardrails, GuardrailState};
pub(crate) use mapping::{any_policy_document, any_policy_summary};
use mapping::{policy_document, policy_summary};
pub use memory_store::MemoryPolicyStore;
pub(crate) use response::{api_error_response, policy_store_error_response};
#[cfg(test)]
use validation::validate_raw_policy;
pub use versions::{get_policy_version, list_policy_versions};

#[derive(Debug, thiserror::Error)]
pub enum PolicyStoreError {
    #[error("not found")]
    NotFound,
    #[error("internal: {0}")]
    Internal(String),
}

#[async_trait]
pub trait PolicyStore: Send + Sync {
    async fn upsert(
        &self,
        workspace_id: &str,
        environment_id: &str,
        policy: &Policy,
        source_yaml: &str,
    ) -> Result<PolicyDocument, PolicyStoreError>;
    async fn get(
        &self,
        workspace_id: &str,
        environment_id: &str,
        policy_id: &str,
    ) -> Result<PolicyDocument, PolicyStoreError>;
    async fn list(
        &self,
        workspace_id: &str,
        environment_id: &str,
    ) -> Result<Vec<PolicySummary>, PolicyStoreError>;
    async fn list_enabled(
        &self,
        workspace_id: &str,
        environment_id: &str,
    ) -> Result<Vec<Arc<Policy>>, PolicyStoreError>;

    /// Upsert a family policy (e.g. the financial family). Stored in the same
    /// registry as content policies and enabled for the selected environment.
    async fn upsert_family(
        &self,
        workspace_id: &str,
        environment_id: &str,
        policy: &FamilyPolicy,
        source_yaml: &str,
    ) -> Result<(), PolicyStoreError>;

    /// Active, enabled family policies for a workspace/environment.
    async fn list_enabled_families(
        &self,
        workspace_id: &str,
        environment_id: &str,
    ) -> Result<Vec<Arc<FamilyPolicy>>, PolicyStoreError>;

    async fn set_enabled(
        &self,
        workspace_id: &str,
        environment_id: &str,
        policy_id: &str,
        enabled: bool,
    ) -> Result<PolicyDocument, PolicyStoreError>;
    async fn batch_set_enabled(
        &self,
        workspace_id: &str,
        environment_id: &str,
        policy_ids: &[String],
        enabled: bool,
    ) -> Result<Vec<PolicySummary>, PolicyStoreError>;
    async fn delete(&self, workspace_id: &str, policy_id: &str) -> Result<(), PolicyStoreError>;

    /// Active policies owned by `agent_id`. Backs
    /// `GET /v1/agents/{id}/guardrails`. Returns an empty vec when the
    /// agent has none; existence of the agent is the caller's concern.
    async fn list_for_agent(
        &self,
        workspace_id: &str,
        environment_id: &str,
        agent_id: &str,
    ) -> Result<Vec<PolicySummary>, PolicyStoreError>;

    /// Soft-delete every active policy owned by `agent_id`. Returns the
    /// ids that were deleted. Called from the cascade-delete path when
    /// an agent is soft-deleted.
    async fn delete_for_agent(
        &self,
        workspace_id: &str,
        agent_id: &str,
    ) -> Result<Vec<String>, PolicyStoreError>;

    /// All saved versions for a policy, newest first. Returns an empty
    /// list when no versions exist yet (pre-versioning policies).
    async fn list_versions(
        &self,
        workspace_id: &str,
        policy_id: &str,
    ) -> Result<EntityVersionListResponse, PolicyStoreError>;

    /// Fetch the YAML for a specific historical version.
    async fn get_version(
        &self,
        workspace_id: &str,
        policy_id: &str,
        version: i32,
    ) -> Result<EntityVersionDetail, PolicyStoreError>;
}

#[derive(Clone)]
pub struct PolicyState {
    pub store: Arc<dyn PolicyStore>,
    pub environment_store: Arc<dyn EnvironmentStore>,
    pub team_store: Arc<dyn TeamStore>,
    /// LLM used by `POST /v1/policies/draft`. `None` when no provider
    /// key is configured — the handler returns 503 in that case.
    pub draft_llm: Option<Arc<dyn LlmClient>>,
    /// Model name passed to the LLM client for drafts. Defaults to
    /// `gpt-4o-mini`; override with `TL_POLICY_DRAFT_MODEL`.
    pub draft_model: String,
}
