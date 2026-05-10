use std::collections::HashMap;
use std::sync::Arc;

use async_trait::async_trait;
use tl_core::Decision;
use tokio::sync::RwLock;

use crate::{DecisionStore, StorageError};

#[derive(Debug, Default, Clone)]
pub struct MemoryStore {
    inner: Arc<RwLock<HashMap<String, Decision>>>,
}

impl MemoryStore {
    pub fn new() -> Self {
        Self::default()
    }
}

#[async_trait]
impl DecisionStore for MemoryStore {
    async fn put(&self, decision: &Decision) -> Result<(), StorageError> {
        self.inner
            .write()
            .await
            .insert(decision.trace_id.clone(), decision.clone());
        Ok(())
    }

    async fn get(&self, trace_id: &str) -> Result<Decision, StorageError> {
        self.inner
            .read()
            .await
            .get(trace_id)
            .cloned()
            .ok_or(StorageError::NotFound)
    }
}
