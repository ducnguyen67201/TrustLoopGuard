//! Custom analytics dashboard endpoints.

use std::sync::Arc;

use async_trait::async_trait;
use axum::{
    extract::{Extension, Path, State},
    http::{HeaderMap, StatusCode},
    response::{IntoResponse, Response},
    Json,
};
#[allow(unused_imports)]
use tl_core::ApiError;
use tl_core::{
    AnalyticsDashboardView, AnalyticsDashboardViewListResponse, AnalyticsFacetCatalogResponse,
    AnalyticsQueryRequest, AnalyticsQueryResponse, CreateAnalyticsDashboardViewRequest,
    UpdateAnalyticsDashboardViewRequest,
};

use crate::{
    auth::{InternalServiceContext, WorkspaceKeyContext},
    jwt::UserContext,
    team::TeamStore,
};

use crate::environments::EnvironmentStore;

mod authorization;
mod defaults;
mod memory_store;
mod query_filter;
mod response;
mod validation;

use authorization::authorize_analytics_workspace;
pub use memory_store::MemoryAnalyticsStore;
use query_filter::with_default_environment_filter;
use response::analytics_error_response;

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

#[derive(Clone)]
pub struct AnalyticsState {
    pub store: Arc<dyn AnalyticsStore>,
    pub environment_store: Arc<dyn EnvironmentStore>,
    pub team_store: Arc<dyn TeamStore>,
}

#[utoipa::path(
    get,
    path = "/v1/analytics/catalog",
    tag = "analytics",
    responses(
        (status = 200, description = "Analytics metric and facet catalog", body = AnalyticsFacetCatalogResponse),
        (status = 401, description = "Missing or invalid API key", body = ApiError),
        (status = 403, description = "Caller cannot access analytics for this workspace", body = ApiError),
    ),
)]
pub async fn catalog(
    State(state): State<AnalyticsState>,
    user: Option<Extension<UserContext>>,
    internal: Option<Extension<InternalServiceContext>>,
    runtime_key: Option<Extension<WorkspaceKeyContext>>,
    headers: HeaderMap,
) -> Response {
    let workspace_id =
        match authorize_analytics_workspace(&state, &headers, user, internal, runtime_key).await {
            Ok(workspace_id) => workspace_id,
            Err(response) => return response,
        };
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
        (status = 403, description = "Caller cannot access analytics for this workspace", body = ApiError),
    ),
)]
pub async fn query(
    State(state): State<AnalyticsState>,
    user: Option<Extension<UserContext>>,
    internal: Option<Extension<InternalServiceContext>>,
    runtime_key: Option<Extension<WorkspaceKeyContext>>,
    headers: HeaderMap,
    Json(request): Json<AnalyticsQueryRequest>,
) -> Response {
    let workspace_id =
        match authorize_analytics_workspace(&state, &headers, user, internal, runtime_key).await {
            Ok(workspace_id) => workspace_id,
            Err(response) => return response,
        };
    let environment_id = match crate::environments::resolve_environment_id(
        &headers,
        state.environment_store.as_ref(),
        &workspace_id,
    )
    .await
    {
        Ok(environment_id) => environment_id,
        Err(error) => return crate::environments::environment_error_response(error),
    };
    let request = with_default_environment_filter(request, &environment_id);
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
        (status = 403, description = "Caller cannot access analytics for this workspace", body = ApiError),
    ),
)]
pub async fn list_views(
    State(state): State<AnalyticsState>,
    user: Option<Extension<UserContext>>,
    internal: Option<Extension<InternalServiceContext>>,
    runtime_key: Option<Extension<WorkspaceKeyContext>>,
    headers: HeaderMap,
) -> Response {
    let workspace_id =
        match authorize_analytics_workspace(&state, &headers, user, internal, runtime_key).await {
            Ok(workspace_id) => workspace_id,
            Err(response) => return response,
        };
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
        (status = 403, description = "Caller cannot access analytics for this workspace", body = ApiError),
    ),
)]
pub async fn create_view(
    State(state): State<AnalyticsState>,
    user: Option<Extension<UserContext>>,
    internal: Option<Extension<InternalServiceContext>>,
    runtime_key: Option<Extension<WorkspaceKeyContext>>,
    headers: HeaderMap,
    Json(request): Json<CreateAnalyticsDashboardViewRequest>,
) -> Response {
    let workspace_id =
        match authorize_analytics_workspace(&state, &headers, user, internal, runtime_key).await {
            Ok(workspace_id) => workspace_id,
            Err(response) => return response,
        };
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
        (status = 403, description = "Caller cannot access analytics for this workspace", body = ApiError),
        (status = 404, description = "Saved view not found", body = ApiError),
    ),
)]
pub async fn update_view(
    State(state): State<AnalyticsState>,
    user: Option<Extension<UserContext>>,
    internal: Option<Extension<InternalServiceContext>>,
    runtime_key: Option<Extension<WorkspaceKeyContext>>,
    headers: HeaderMap,
    Path(id): Path<String>,
    Json(request): Json<UpdateAnalyticsDashboardViewRequest>,
) -> Response {
    let workspace_id =
        match authorize_analytics_workspace(&state, &headers, user, internal, runtime_key).await {
            Ok(workspace_id) => workspace_id,
            Err(response) => return response,
        };
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
        (status = 403, description = "Caller cannot access analytics for this workspace", body = ApiError),
        (status = 404, description = "Saved view not found", body = ApiError),
    ),
)]
pub async fn delete_view(
    State(state): State<AnalyticsState>,
    user: Option<Extension<UserContext>>,
    internal: Option<Extension<InternalServiceContext>>,
    runtime_key: Option<Extension<WorkspaceKeyContext>>,
    headers: HeaderMap,
    Path(id): Path<String>,
) -> Response {
    let workspace_id =
        match authorize_analytics_workspace(&state, &headers, user, internal, runtime_key).await {
            Ok(workspace_id) => workspace_id,
            Err(response) => return response,
        };
    match state.store.delete_view(&workspace_id, &id).await {
        Ok(()) => StatusCode::NO_CONTENT.into_response(),
        Err(error) => analytics_error_response(error),
    }
}
