//! Financial authorization endpoints.

use std::sync::Arc;

use async_trait::async_trait;
use tl_core::{CreateFinancialActionRequest, FinancialActionRecord, FinancialActionStatus};

mod handlers;
mod memory_store;
mod response;
mod validation;

pub use handlers::{
    __path_approve_action, __path_create_action, __path_deny_action, __path_execute_action,
    __path_get_action, approve_action, create_action, deny_action, execute_action, get_action,
};
pub use memory_store::MemoryFinancialStore;

#[derive(Debug, thiserror::Error)]
pub enum FinancialStoreError {
    #[error("not found")]
    NotFound,
    #[error("conflict")]
    Conflict,
    #[error("validation: {0}")]
    Validation(String),
    #[error("internal: {0}")]
    Internal(String),
}

#[async_trait]
pub trait FinancialStore: Send + Sync {
    async fn create_action(
        &self,
        workspace_id: &str,
        input: CreateFinancialActionRequest,
    ) -> Result<FinancialActionRecord, FinancialStoreError>;

    async fn get_action(
        &self,
        workspace_id: &str,
        action_id: &str,
    ) -> Result<FinancialActionRecord, FinancialStoreError>;

    async fn transition_action(
        &self,
        workspace_id: &str,
        action_id: &str,
        status: FinancialActionStatus,
        event_type: &str,
    ) -> Result<FinancialActionRecord, FinancialStoreError>;
}

#[derive(Clone)]
pub struct FinancialState {
    pub store: Arc<dyn FinancialStore>,
}
