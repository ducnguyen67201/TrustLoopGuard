//! Decision-log storage. Trait-first so we can swap in-memory for Postgres
//! without touching the engine or server.

use async_trait::async_trait;
use tl_core::Decision;

#[derive(Debug, thiserror::Error)]
pub enum StorageError {
    #[error("not found")]
    NotFound,
    #[error("internal: {0}")]
    Internal(String),
}

#[async_trait]
pub trait DecisionStore: Send + Sync {
    async fn put(&self, decision: &Decision) -> Result<(), StorageError>;
    async fn get(&self, trace_id: &str) -> Result<Decision, StorageError>;
}

pub mod memory_store;
pub use memory_store::MemoryStore;
