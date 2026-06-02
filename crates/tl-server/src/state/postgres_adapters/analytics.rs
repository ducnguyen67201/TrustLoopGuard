use std::sync::Arc;

use anyhow::Result;
use async_trait::async_trait;
use tl_storage::AnalyticsRepo;

use crate::analytics::{AnalyticsStore, AnalyticsStoreError};

pub struct PostgresAnalyticsAdapter(pub Arc<AnalyticsRepo>);

impl PostgresAnalyticsAdapter {
    pub fn new(repo: Arc<AnalyticsRepo>) -> Arc<Self> {
        Arc::new(Self(repo))
    }
}

#[async_trait]
impl AnalyticsStore for PostgresAnalyticsAdapter {
    async fn catalog(
        &self,
        workspace_id: &str,
    ) -> Result<tl_core::AnalyticsFacetCatalogResponse, AnalyticsStoreError> {
        self.0
            .catalog(workspace_id)
            .await
            .map_err(analytics_store_error)
    }

    async fn query(
        &self,
        workspace_id: &str,
        request: tl_core::AnalyticsQueryRequest,
    ) -> Result<tl_core::AnalyticsQueryResponse, AnalyticsStoreError> {
        self.0
            .query(workspace_id, request)
            .await
            .map_err(analytics_store_error)
    }

    async fn list_views(
        &self,
        workspace_id: &str,
    ) -> Result<Vec<tl_core::AnalyticsDashboardView>, AnalyticsStoreError> {
        self.0
            .list_views(workspace_id)
            .await
            .map_err(analytics_store_error)
    }

    async fn create_view(
        &self,
        workspace_id: &str,
        request: tl_core::CreateAnalyticsDashboardViewRequest,
    ) -> Result<tl_core::AnalyticsDashboardView, AnalyticsStoreError> {
        self.0
            .create_view(workspace_id, request)
            .await
            .map_err(analytics_store_error)
    }

    async fn update_view(
        &self,
        workspace_id: &str,
        view_id: &str,
        request: tl_core::UpdateAnalyticsDashboardViewRequest,
    ) -> Result<tl_core::AnalyticsDashboardView, AnalyticsStoreError> {
        self.0
            .update_view(workspace_id, view_id, request)
            .await
            .map_err(analytics_store_error)
    }

    async fn delete_view(
        &self,
        workspace_id: &str,
        view_id: &str,
    ) -> Result<(), AnalyticsStoreError> {
        self.0
            .delete_view(workspace_id, view_id)
            .await
            .map_err(analytics_store_error)
    }
}

fn analytics_store_error(error: tl_storage::StorageError) -> AnalyticsStoreError {
    match error {
        tl_storage::StorageError::NotFound => AnalyticsStoreError::NotFound,
        tl_storage::StorageError::Conflict => {
            AnalyticsStoreError::Validation("analytics view already exists".into())
        }
        tl_storage::StorageError::Internal(message)
            if message.contains("required")
                || message.contains("must")
                || message.contains("filters") =>
        {
            AnalyticsStoreError::Validation(message)
        }
        tl_storage::StorageError::Internal(message) => AnalyticsStoreError::Internal(message),
    }
}
