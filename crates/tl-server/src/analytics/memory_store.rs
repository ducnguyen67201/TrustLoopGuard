use std::collections::HashMap;

use async_trait::async_trait;
use tl_core::{
    AnalyticsDashboardView, AnalyticsFacetCatalogResponse, AnalyticsQueryRequest,
    AnalyticsQueryResponse, CreateAnalyticsDashboardViewRequest,
    UpdateAnalyticsDashboardViewRequest,
};
use tokio::sync::RwLock;

use super::defaults::{default_views, empty_catalog};
use super::validation::{validate_config, validate_name, validate_view_request};
use super::{AnalyticsStore, AnalyticsStoreError};

#[derive(Default)]
pub struct MemoryAnalyticsStore {
    views: RwLock<HashMap<String, Vec<AnalyticsDashboardView>>>,
}

impl MemoryAnalyticsStore {
    pub fn new() -> Self {
        Self::default()
    }
}

#[async_trait]
impl AnalyticsStore for MemoryAnalyticsStore {
    async fn catalog(
        &self,
        _workspace_id: &str,
    ) -> Result<AnalyticsFacetCatalogResponse, AnalyticsStoreError> {
        Ok(empty_catalog())
    }

    async fn query(
        &self,
        _workspace_id: &str,
        request: AnalyticsQueryRequest,
    ) -> Result<AnalyticsQueryResponse, AnalyticsStoreError> {
        Ok(AnalyticsQueryResponse {
            metric: request.metric,
            group_by: request.group_by,
            total: 0.0,
            points: vec![],
        })
    }

    async fn list_views(
        &self,
        workspace_id: &str,
    ) -> Result<Vec<AnalyticsDashboardView>, AnalyticsStoreError> {
        Ok(self
            .views
            .read()
            .await
            .get(workspace_id)
            .cloned()
            .unwrap_or_else(default_views))
    }

    async fn create_view(
        &self,
        workspace_id: &str,
        request: CreateAnalyticsDashboardViewRequest,
    ) -> Result<AnalyticsDashboardView, AnalyticsStoreError> {
        validate_view_request(&request.name, &request.config)?;
        let now = chrono::Utc::now().to_rfc3339();
        let mut view = AnalyticsDashboardView {
            id: uuid::Uuid::now_v7().to_string(),
            name: request.name.trim().to_string(),
            is_default: request.is_default,
            config: request.config,
            created_at: now.clone(),
            updated_at: now,
        };
        let mut views = self.views.write().await;
        let rows = views
            .entry(workspace_id.to_string())
            .or_insert_with(default_views);
        if view.is_default {
            for row in rows.iter_mut() {
                row.is_default = false;
            }
        }
        rows.push(view.clone());
        if !rows.iter().any(|row| row.is_default) {
            view.is_default = true;
        }
        Ok(view)
    }

    async fn update_view(
        &self,
        workspace_id: &str,
        view_id: &str,
        request: UpdateAnalyticsDashboardViewRequest,
    ) -> Result<AnalyticsDashboardView, AnalyticsStoreError> {
        if let Some(name) = request.name.as_deref() {
            validate_name(name)?;
        }
        if let Some(config) = request.config.as_ref() {
            validate_config(config)?;
        }
        let mut views = self.views.write().await;
        let rows = views
            .entry(workspace_id.to_string())
            .or_insert_with(default_views);
        if request.is_default == Some(true) {
            for row in rows.iter_mut() {
                row.is_default = false;
            }
        }
        let row = rows
            .iter_mut()
            .find(|row| row.id == view_id)
            .ok_or(AnalyticsStoreError::NotFound)?;
        if let Some(name) = request.name {
            row.name = name.trim().to_string();
        }
        if let Some(is_default) = request.is_default {
            row.is_default = is_default;
        }
        if let Some(config) = request.config {
            row.config = config;
        }
        row.updated_at = chrono::Utc::now().to_rfc3339();
        Ok(row.clone())
    }

    async fn delete_view(
        &self,
        workspace_id: &str,
        view_id: &str,
    ) -> Result<(), AnalyticsStoreError> {
        let mut views = self.views.write().await;
        let rows = views
            .entry(workspace_id.to_string())
            .or_insert_with(default_views);
        let before = rows.len();
        rows.retain(|row| row.id != view_id);
        if rows.len() == before {
            Err(AnalyticsStoreError::NotFound)
        } else {
            Ok(())
        }
    }
}
