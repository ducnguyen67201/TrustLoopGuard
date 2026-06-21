use std::collections::HashMap;
use std::sync::RwLock;

use async_trait::async_trait;

pub const KNOWLEDGE_GROUNDING_FLAG: &str = "knowledge_grounding";

#[derive(Debug, Clone, thiserror::Error)]
pub enum FeatureFlagStoreError {
    #[error("internal: {0}")]
    Internal(String),
}

#[async_trait]
pub trait FeatureFlagStore: Send + Sync {
    async fn is_enabled(&self, key: &str) -> Result<bool, FeatureFlagStoreError>;
}

#[derive(Default)]
pub struct MemoryFeatureFlagStore {
    flags: RwLock<HashMap<String, bool>>,
}

impl MemoryFeatureFlagStore {
    pub fn new() -> Self {
        Self::default()
    }

    #[cfg(test)]
    pub fn with_flag(key: impl Into<String>, enabled: bool) -> Self {
        let store = Self::new();
        store.set_enabled(key, enabled);
        store
    }

    #[cfg(test)]
    pub fn set_enabled(&self, key: impl Into<String>, enabled: bool) {
        self.flags
            .write()
            .expect("feature flag lock")
            .insert(key.into(), enabled);
    }
}

#[async_trait]
impl FeatureFlagStore for MemoryFeatureFlagStore {
    async fn is_enabled(&self, key: &str) -> Result<bool, FeatureFlagStoreError> {
        Ok(*self
            .flags
            .read()
            .map_err(|_| FeatureFlagStoreError::Internal("feature flag lock poisoned".into()))?
            .get(key)
            .unwrap_or(&false))
    }
}

#[cfg(test)]
mod tests {
    use super::{FeatureFlagStore, MemoryFeatureFlagStore};

    #[tokio::test]
    async fn memory_flags_default_to_disabled() {
        let store = MemoryFeatureFlagStore::new();

        assert!(!store.is_enabled("missing").await.unwrap());
    }

    #[tokio::test]
    async fn memory_flags_return_configured_value() {
        let store = MemoryFeatureFlagStore::with_flag("knowledge_grounding", true);

        assert!(store.is_enabled("knowledge_grounding").await.unwrap());
        assert!(!store.is_enabled("other").await.unwrap());
    }
}
