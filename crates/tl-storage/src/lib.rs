//! Decision-log storage. Trait-first so we can swap in-memory for Postgres
//! without touching the engine or server.
//!
//! Postgres types (`PostgresStore`, `migrate`) are gated behind the
//! `postgres` feature so the default tl-storage build doesn't compile
//! sqlx. PR 15 (server) flips the feature on for production builds.

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

#[cfg(feature = "postgres")]
pub mod agent_repo;
#[cfg(feature = "postgres")]
pub mod escalations;
#[cfg(feature = "postgres")]
pub mod postgres;
#[cfg(feature = "postgres")]
pub mod writer;

#[cfg(feature = "postgres")]
pub use agent_repo::AgentRepo;
#[cfg(feature = "postgres")]
pub use escalations::{EscalationRepo, EscalationRow};
#[cfg(feature = "postgres")]
pub use postgres::{migrate as migrate_postgres, PostgresStore};
#[cfg(feature = "postgres")]
pub use writer::{spawn_writer, TraceWrite, WriterConfig};
