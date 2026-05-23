//! Custom analytics dashboard endpoints.

use std::collections::HashMap;
use std::sync::Arc;

use async_trait::async_trait;
use axum::{
    extract::{Path, State},
    http::{HeaderMap, StatusCode},
    response::{IntoResponse, Response},
    Json,
};
use serde_json::json;
use tl_core::{
    AnalyticsCatalogDimension, AnalyticsCatalogMetric, AnalyticsChartType, AnalyticsDashboardView,
    AnalyticsDashboardViewConfig, AnalyticsDashboardViewListResponse, AnalyticsDimension,
    AnalyticsFacetCatalogResponse, AnalyticsMetric, AnalyticsQueryRequest, AnalyticsQueryResponse,
    AnalyticsWidgetLayout, ApiError, ApiErrorCode, CreateAnalyticsDashboardViewRequest,
    UpdateAnalyticsDashboardViewRequest,
};
use tokio::sync::RwLock;

#[derive(Debug, thiserror::Error)]
pub enum AnalyticsStoreError {
    #[error("not found")]
    NotFound,
    #[error("validation: {0}")]
    Validation(String),
    #[error("internal: {0}")]
    Internal(String),
}

#[async_trait]
pub trait AnalyticsStore: Send + Sync {
    async fn catalog(
        &self,
        workspace_id: &str,
    ) -> Result<AnalyticsFacetCatalogResponse, AnalyticsStoreError>;

    async fn query(
        &self,
        workspace_id: &str,
        request: AnalyticsQueryRequest,
    ) -> Result<AnalyticsQueryResponse, AnalyticsStoreError>;

    async fn list_views(
        &self,
        workspace_id: &str,
    ) -> Result<Vec<AnalyticsDashboardView>, AnalyticsStoreError>;

    async fn create_view(
        &self,
        workspace_id: &str,
        request: CreateAnalyticsDashboardViewRequest,
    ) -> Result<AnalyticsDashboardView, AnalyticsStoreError>;

    async fn update_view(
        &self,
        workspace_id: &str,
        view_id: &str,
        request: UpdateAnalyticsDashboardViewRequest,
    ) -> Result<AnalyticsDashboardView, AnalyticsStoreError>;

    async fn delete_view(
        &self,
        workspace_id: &str,
        view_id: &str,
    ) -> Result<(), AnalyticsStoreError>;
}

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

#[derive(Clone)]
pub struct AnalyticsState {
    pub store: Arc<dyn AnalyticsStore>,
}

#[utoipa::path(
    get,
    path = "/v1/analytics/catalog",
    tag = "analytics",
    responses(
        (status = 200, description = "Analytics metric and facet catalog", body = AnalyticsFacetCatalogResponse),
        (status = 401, description = "Missing or invalid API key", body = ApiError),
    ),
)]
pub async fn catalog(State(state): State<AnalyticsState>, headers: HeaderMap) -> Response {
    let workspace_id = crate::policies::workspace_id_from_headers(&headers);
    match state.store.catalog(&workspace_id).await {
        Ok(response) => Json(response).into_response(),
        Err(error) => analytics_error_response(error),
    }
}

#[utoipa::path(
    post,
    path = "/v1/analytics/query",
    tag = "analytics",
    request_body = AnalyticsQueryRequest,
    responses(
        (status = 200, description = "Analytics query result", body = AnalyticsQueryResponse),
        (status = 400, description = "Malformed or invalid request", body = ApiError),
        (status = 401, description = "Missing or invalid API key", body = ApiError),
    ),
)]
pub async fn query(
    State(state): State<AnalyticsState>,
    headers: HeaderMap,
    Json(request): Json<AnalyticsQueryRequest>,
) -> Response {
    let workspace_id = crate::policies::workspace_id_from_headers(&headers);
    match state.store.query(&workspace_id, request).await {
        Ok(response) => Json(response).into_response(),
        Err(error) => analytics_error_response(error),
    }
}

#[utoipa::path(
    get,
    path = "/v1/analytics/views",
    tag = "analytics",
    responses(
        (status = 200, description = "Saved analytics dashboard views", body = AnalyticsDashboardViewListResponse),
        (status = 401, description = "Missing or invalid API key", body = ApiError),
    ),
)]
pub async fn list_views(State(state): State<AnalyticsState>, headers: HeaderMap) -> Response {
    let workspace_id = crate::policies::workspace_id_from_headers(&headers);
    match state.store.list_views(&workspace_id).await {
        Ok(views) => Json(AnalyticsDashboardViewListResponse { views }).into_response(),
        Err(error) => analytics_error_response(error),
    }
}

#[utoipa::path(
    post,
    path = "/v1/analytics/views",
    tag = "analytics",
    request_body = CreateAnalyticsDashboardViewRequest,
    responses(
        (status = 201, description = "Saved analytics dashboard view created", body = AnalyticsDashboardView),
        (status = 400, description = "Malformed or invalid request", body = ApiError),
        (status = 401, description = "Missing or invalid API key", body = ApiError),
    ),
)]
pub async fn create_view(
    State(state): State<AnalyticsState>,
    headers: HeaderMap,
    Json(request): Json<CreateAnalyticsDashboardViewRequest>,
) -> Response {
    let workspace_id = crate::policies::workspace_id_from_headers(&headers);
    match state.store.create_view(&workspace_id, request).await {
        Ok(view) => (StatusCode::CREATED, Json(view)).into_response(),
        Err(error) => analytics_error_response(error),
    }
}

#[utoipa::path(
    patch,
    path = "/v1/analytics/views/{id}",
    tag = "analytics",
    params(("id" = String, Path, description = "Saved dashboard view id")),
    request_body = UpdateAnalyticsDashboardViewRequest,
    responses(
        (status = 200, description = "Saved analytics dashboard view updated", body = AnalyticsDashboardView),
        (status = 400, description = "Malformed or invalid request", body = ApiError),
        (status = 401, description = "Missing or invalid API key", body = ApiError),
        (status = 404, description = "Saved view not found", body = ApiError),
    ),
)]
pub async fn update_view(
    State(state): State<AnalyticsState>,
    headers: HeaderMap,
    Path(id): Path<String>,
    Json(request): Json<UpdateAnalyticsDashboardViewRequest>,
) -> Response {
    let workspace_id = crate::policies::workspace_id_from_headers(&headers);
    match state.store.update_view(&workspace_id, &id, request).await {
        Ok(view) => Json(view).into_response(),
        Err(error) => analytics_error_response(error),
    }
}

#[utoipa::path(
    delete,
    path = "/v1/analytics/views/{id}",
    tag = "analytics",
    params(("id" = String, Path, description = "Saved dashboard view id")),
    responses(
        (status = 204, description = "Saved analytics dashboard view deleted"),
        (status = 401, description = "Missing or invalid API key", body = ApiError),
        (status = 404, description = "Saved view not found", body = ApiError),
    ),
)]
pub async fn delete_view(
    State(state): State<AnalyticsState>,
    headers: HeaderMap,
    Path(id): Path<String>,
) -> Response {
    let workspace_id = crate::policies::workspace_id_from_headers(&headers);
    match state.store.delete_view(&workspace_id, &id).await {
        Ok(()) => StatusCode::NO_CONTENT.into_response(),
        Err(error) => analytics_error_response(error),
    }
}

pub fn analytics_error_response(error: AnalyticsStoreError) -> Response {
    let (status, code) = match error {
        AnalyticsStoreError::NotFound => (StatusCode::NOT_FOUND, ApiErrorCode::NotFound),
        AnalyticsStoreError::Validation(_) => (StatusCode::BAD_REQUEST, ApiErrorCode::Invalid),
        AnalyticsStoreError::Internal(_) => {
            (StatusCode::INTERNAL_SERVER_ERROR, ApiErrorCode::Internal)
        }
    };
    crate::log_api_error(status, code, &error.to_string());
    let body = ApiError {
        code,
        message: error.to_string(),
        retriable: matches!(code, ApiErrorCode::Internal | ApiErrorCode::Unavailable),
        details: json!(null),
    };
    (status, Json(body)).into_response()
}

fn validate_view_request(
    name: &str,
    config: &AnalyticsDashboardViewConfig,
) -> Result<(), AnalyticsStoreError> {
    validate_name(name)?;
    validate_config(config)
}

fn validate_name(name: &str) -> Result<(), AnalyticsStoreError> {
    if name.trim().is_empty() {
        Err(AnalyticsStoreError::Validation(
            "analytics view name is required".into(),
        ))
    } else {
        Ok(())
    }
}

fn validate_config(config: &AnalyticsDashboardViewConfig) -> Result<(), AnalyticsStoreError> {
    if config.widgets.is_empty() {
        return Err(AnalyticsStoreError::Validation(
            "analytics view must include at least one widget".into(),
        ));
    }
    for widget in &config.widgets {
        validate_layout(&widget.layout)?;
    }
    Ok(())
}

fn validate_layout(layout: &AnalyticsWidgetLayout) -> Result<(), AnalyticsStoreError> {
    if layout.w == 0 || layout.w > 12 || layout.h == 0 || layout.h > 4 {
        return Err(AnalyticsStoreError::Validation(
            "analytics widget layout must use width 1-12 and height 1-4".into(),
        ));
    }
    if layout.x >= 12 || layout.x + layout.w > 12 {
        return Err(AnalyticsStoreError::Validation(
            "analytics widget layout must fit within the 12-column grid".into(),
        ));
    }
    Ok(())
}

fn empty_catalog() -> AnalyticsFacetCatalogResponse {
    AnalyticsFacetCatalogResponse {
        metrics: vec![
            AnalyticsCatalogMetric {
                metric: AnalyticsMetric::TraceCount,
                label: "Traces".into(),
                default_chart_type: AnalyticsChartType::Bar,
            },
            AnalyticsCatalogMetric {
                metric: AnalyticsMetric::InterventionRate,
                label: "Intervention rate".into(),
                default_chart_type: AnalyticsChartType::Line,
            },
            AnalyticsCatalogMetric {
                metric: AnalyticsMetric::P95LatencyMs,
                label: "p95 latency".into(),
                default_chart_type: AnalyticsChartType::Line,
            },
        ],
        dimensions: vec![
            AnalyticsCatalogDimension {
                dimension: AnalyticsDimension::AgentId,
                label: "Agent".into(),
            },
            AnalyticsCatalogDimension {
                dimension: AnalyticsDimension::Decision,
                label: "Verdict".into(),
            },
        ],
        chart_types: vec![
            AnalyticsChartType::BigNumber,
            AnalyticsChartType::Bar,
            AnalyticsChartType::Line,
            AnalyticsChartType::Donut,
        ],
        facets: vec![],
    }
}

fn default_views() -> Vec<AnalyticsDashboardView> {
    let now = chrono::Utc::now().to_rfc3339();
    vec![AnalyticsDashboardView {
        id: "default".into(),
        name: "Default analytics".into(),
        is_default: true,
        config: AnalyticsDashboardViewConfig {
            filters: vec![],
            widgets: vec![
                tl_core::AnalyticsDashboardWidget {
                    id: "trace-volume".into(),
                    title: "Trace volume".into(),
                    metric: AnalyticsMetric::TraceCount,
                    chart_type: AnalyticsChartType::Bar,
                    group_by: Some(AnalyticsDimension::Decision),
                    layout: AnalyticsWidgetLayout {
                        x: 0,
                        y: 0,
                        w: 6,
                        h: 1,
                    },
                },
                tl_core::AnalyticsDashboardWidget {
                    id: "intervention-rate".into(),
                    title: "Intervention rate".into(),
                    metric: AnalyticsMetric::InterventionRate,
                    chart_type: AnalyticsChartType::BigNumber,
                    group_by: None,
                    layout: AnalyticsWidgetLayout {
                        x: 6,
                        y: 0,
                        w: 3,
                        h: 1,
                    },
                },
                tl_core::AnalyticsDashboardWidget {
                    id: "p95-latency".into(),
                    title: "p95 latency".into(),
                    metric: AnalyticsMetric::P95LatencyMs,
                    chart_type: AnalyticsChartType::BigNumber,
                    group_by: None,
                    layout: AnalyticsWidgetLayout {
                        x: 9,
                        y: 0,
                        w: 3,
                        h: 1,
                    },
                },
            ],
        },
        created_at: now.clone(),
        updated_at: now,
    }]
}
