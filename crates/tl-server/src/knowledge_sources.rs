//! Knowledge-source dashboard endpoints.

use std::sync::Arc;

use async_trait::async_trait;
use tl_core::{CreateKnowledgeSourceRequest, KnowledgeSourceDocument, KnowledgeSourceFileResponse};

pub(crate) mod handlers;
mod memory_store;
mod response;
mod validation;

pub use handlers::{create_knowledge_source, get_knowledge_source_file, list_knowledge_sources};
pub use memory_store::MemoryKnowledgeStore;

pub(crate) use validation::decode_file_data;

#[derive(Debug, thiserror::Error)]
pub enum KnowledgeStoreError {
    #[error("not found")]
    NotFound,
    #[error("validation: {0}")]
    Validation(String),
    #[error("internal: {0}")]
    Internal(String),
}

#[async_trait]
pub trait KnowledgeStore: Send + Sync {
    async fn list(
        &self,
        workspace_id: &str,
    ) -> Result<Vec<KnowledgeSourceDocument>, KnowledgeStoreError>;
    async fn create(
        &self,
        workspace_id: &str,
        input: CreateKnowledgeSourceRequest,
    ) -> Result<KnowledgeSourceDocument, KnowledgeStoreError>;
    async fn get_file(
        &self,
        workspace_id: &str,
        source_id: &str,
    ) -> Result<KnowledgeSourceFileResponse, KnowledgeStoreError>;
}

#[derive(Clone)]
pub struct KnowledgeState {
    pub store: Arc<dyn KnowledgeStore>,
    pub team_store: Arc<dyn crate::team::TeamStore>,
}
