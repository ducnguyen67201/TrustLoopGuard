use axum::{
    extract::{Extension, Path, State},
    http::{HeaderMap, StatusCode},
    response::{IntoResponse, Response},
    Json,
};
#[allow(unused_imports)]
use tl_core::{
    ApiError, CreateKnowledgeSourceRequest, KnowledgeSourceDocument, KnowledgeSourceFileResponse,
    KnowledgeSourceListResponse,
};

use super::{
    response::knowledge_error_response, validation::validate_create_request, KnowledgeState,
};
use crate::{
    auth::{InternalServiceContext, WorkspaceKeyContext},
    dashboard_admin::authorize_workspace_admin,
    jwt::UserContext,
};

/// `GET /v1/knowledge-sources` - list workspace knowledge sources.
#[utoipa::path(
    get,
    path = "/v1/knowledge-sources",
    tag = "knowledge-sources",
    responses(
        (status = 200, description = "Knowledge sources", body = KnowledgeSourceListResponse),
        (status = 401, description = "Missing or invalid API key", body = ApiError),
    ),
)]
pub async fn list_knowledge_sources(
    State(state): State<KnowledgeState>,
    headers: HeaderMap,
) -> Response {
    let workspace_id = match crate::policies::workspace_id_from_headers(&headers) {
        Ok(workspace_id) => workspace_id,
        Err(response) => return response,
    };
    match state.store.list(&workspace_id).await {
        Ok(knowledge_sources) => {
            Json(KnowledgeSourceListResponse { knowledge_sources }).into_response()
        }
        Err(e) => knowledge_error_response(e),
    }
}

/// `POST /v1/knowledge-sources` - create a workspace knowledge source.
#[utoipa::path(
    post,
    path = "/v1/knowledge-sources",
    tag = "knowledge-sources",
    request_body = CreateKnowledgeSourceRequest,
    responses(
        (status = 201, description = "Knowledge source created", body = KnowledgeSourceDocument),
        (status = 400, description = "Malformed or invalid request", body = ApiError),
        (status = 401, description = "Missing or invalid API key", body = ApiError),
        (status = 403, description = "Owner or Admin role required", body = ApiError),
    ),
)]
pub async fn create_knowledge_source(
    State(state): State<KnowledgeState>,
    user: Option<Extension<UserContext>>,
    internal: Option<Extension<InternalServiceContext>>,
    runtime_key: Option<Extension<WorkspaceKeyContext>>,
    headers: HeaderMap,
    Json(input): Json<CreateKnowledgeSourceRequest>,
) -> Response {
    let (workspace_id, _) = match authorize_workspace_admin(
        &state.team_store,
        &headers,
        user,
        internal,
        runtime_key,
        "create knowledge sources",
    )
    .await
    {
        Ok(authorized) => authorized,
        Err(response) => return response,
    };
    if let Err(e) = validate_create_request(&input) {
        return knowledge_error_response(e);
    }
    match state.store.create(&workspace_id, input).await {
        Ok(source) => (StatusCode::CREATED, Json(source)).into_response(),
        Err(e) => knowledge_error_response(e),
    }
}

/// `GET /v1/knowledge-sources/:id/file` - fetch stored file content.
#[utoipa::path(
    get,
    path = "/v1/knowledge-sources/{id}/file",
    tag = "knowledge-sources",
    params(("id" = String, Path, description = "Knowledge source id")),
    responses(
        (status = 200, description = "Knowledge source file", body = KnowledgeSourceFileResponse),
        (status = 401, description = "Missing or invalid API key", body = ApiError),
        (status = 404, description = "Knowledge source file not found", body = ApiError),
    ),
)]
pub async fn get_knowledge_source_file(
    State(state): State<KnowledgeState>,
    headers: HeaderMap,
    Path(id): Path<String>,
) -> Response {
    let workspace_id = match crate::policies::workspace_id_from_headers(&headers) {
        Ok(workspace_id) => workspace_id,
        Err(response) => return response,
    };
    match state.store.get_file(&workspace_id, &id).await {
        Ok(file) => Json(file).into_response(),
        Err(e) => knowledge_error_response(e),
    }
}
