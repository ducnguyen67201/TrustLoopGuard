use std::sync::Arc;

use async_trait::async_trait;
use tl_storage::GlobalFeatureFlagRepo;

use crate::feature_flags::{FeatureFlagStore, FeatureFlagStoreError};

pub struct PostgresFeatureFlagAdapter(pub Arc<GlobalFeatureFlagRepo>);

impl PostgresFeatureFlagAdapter {
    pub fn new(repo: Arc<GlobalFeatureFlagRepo>) -> Arc<Self> {
        Arc::new(Self(repo))
    }
}

#[async_trait]
impl FeatureFlagStore for PostgresFeatureFlagAdapter {
    async fn is_enabled(&self, key: &str) -> Result<bool, FeatureFlagStoreError> {
        self.0
            .is_enabled(key, false)
            .await
            .map_err(|error| FeatureFlagStoreError::Internal(error.to_string()))
    }
}
