//! Knowledge-source dashboard endpoints.

use std::sync::Arc;

use async_trait::async_trait;
use axum::{
    extract::{Path, State},
    http::{HeaderMap, StatusCode},
    response::{IntoResponse, Response},
    Json,
};
use base64::{engine::general_purpose::STANDARD, Engine as _};
use serde_json::json;
use tl_core::{
    ApiError, ApiErrorCode, CreateKnowledgeSourceRequest, DashboardKnowledgeSourceKind,
    KnowledgeSourceDocument, KnowledgeSourceFileResponse, KnowledgeSourceListResponse,
    KnowledgeSourceStatus,
};
use tokio::sync::RwLock;

const MAX_KNOWLEDGE_FILE_BYTES: usize = 10 * 1024 * 1024;

#[derive(Debug, thiserror::Error)]
pub enum KnowledgeStoreError {
    #[error("not found")]
    NotFound,
    #[error("validation: {0}")]
    Validation(String),
    #[error("internal: {0}")]
    Internal(String),
}

#[async_trait]
pub trait KnowledgeStore: Send + Sync {
    async fn list(
        &self,
        workspace_id: &str,
    ) -> Result<Vec<KnowledgeSourceDocument>, KnowledgeStoreError>;
    async fn create(
        &self,
        workspace_id: &str,
        input: CreateKnowledgeSourceRequest,
    ) -> Result<KnowledgeSourceDocument, KnowledgeStoreError>;
    async fn get_file(
        &self,
        workspace_id: &str,
        source_id: &str,
    ) -> Result<KnowledgeSourceFileResponse, KnowledgeStoreError>;
}

#[derive(Debug, Default)]
pub struct MemoryKnowledgeStore {
    sources: RwLock<Vec<KnowledgeSourceDocument>>,
    files: RwLock<std::collections::HashMap<String, KnowledgeSourceFileResponse>>,
}

impl MemoryKnowledgeStore {
    pub fn new() -> Self {
        Self::default()
    }
}

#[async_trait]
impl KnowledgeStore for MemoryKnowledgeStore {
    async fn list(
        &self,
        _workspace_id: &str,
    ) -> Result<Vec<KnowledgeSourceDocument>, KnowledgeStoreError> {
        Ok(self.sources.read().await.clone())
    }

    async fn create(
        &self,
        _workspace_id: &str,
        input: CreateKnowledgeSourceRequest,
    ) -> Result<KnowledgeSourceDocument, KnowledgeStoreError> {
        validate_create_request(&input)?;
        let now = chrono::Utc::now().to_rfc3339();
        let id = uuid::Uuid::new_v4().to_string();
        let mut metadata = serde_json::Map::new();
        if let Some(notes) = input
            .notes
            .as_ref()
            .filter(|value| !value.trim().is_empty())
        {
            metadata.insert("notes".into(), serde_json::Value::String(notes.clone()));
        }
        if let Some(file) = input.file {
            let bytes = decode_file_data(&file.data_base64)?;
            metadata.insert(
                "file".into(),
                json!({
                    "fileName": file.file_name,
                    "mediaType": file.media_type,
                    "byteSize": bytes.len(),
                }),
            );
            self.files.write().await.insert(
                id.clone(),
                KnowledgeSourceFileResponse {
                    file_name: file.file_name,
                    media_type: file.media_type,
                    byte_size: bytes.len() as i32,
                    data_base64: STANDARD.encode(bytes),
                },
            );
        }

        let source = KnowledgeSourceDocument {
            id,
            title: input.title,
            kind: input.kind,
            location: input.location,
            status: KnowledgeSourceStatus::Ready,
            metadata: serde_json::Value::Object(metadata),
            created_at: now.clone(),
            updated_at: now.clone(),
            last_indexed_at: Some(now),
        };
        self.sources.write().await.push(source.clone());
        Ok(source)
    }

    async fn get_file(
        &self,
        _workspace_id: &str,
        source_id: &str,
    ) -> Result<KnowledgeSourceFileResponse, KnowledgeStoreError> {
        self.files
            .read()
            .await
            .get(source_id)
            .cloned()
            .ok_or(KnowledgeStoreError::NotFound)
    }
}

#[derive(Clone)]
pub struct KnowledgeState {
    pub store: Arc<dyn KnowledgeStore>,
}

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
    let workspace_id = crate::policies::workspace_id_from_headers(&headers);
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
    ),
)]
pub async fn create_knowledge_source(
    State(state): State<KnowledgeState>,
    headers: HeaderMap,
    Json(input): Json<CreateKnowledgeSourceRequest>,
) -> Response {
    if let Err(e) = validate_create_request(&input) {
        return knowledge_error_response(e);
    }
    let workspace_id = crate::policies::workspace_id_from_headers(&headers);
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
    let workspace_id = crate::policies::workspace_id_from_headers(&headers);
    match state.store.get_file(&workspace_id, &id).await {
        Ok(file) => Json(file).into_response(),
        Err(e) => knowledge_error_response(e),
    }
}

fn validate_create_request(
    input: &CreateKnowledgeSourceRequest,
) -> Result<(), KnowledgeStoreError> {
    if input.title.trim().is_empty() {
        return Err(KnowledgeStoreError::Validation("title is required".into()));
    }
    match input.kind {
        DashboardKnowledgeSourceKind::File => {
            let file = input
                .file
                .as_ref()
                .ok_or_else(|| KnowledgeStoreError::Validation("file is required".into()))?;
            if file.file_name.trim().is_empty() {
                return Err(KnowledgeStoreError::Validation(
                    "file_name is required".into(),
                ));
            }
            let bytes = decode_file_data(&file.data_base64)?;
            if bytes.len() > MAX_KNOWLEDGE_FILE_BYTES {
                return Err(KnowledgeStoreError::Validation(
                    "file must be 10 MB or smaller".into(),
                ));
            }
        }
        DashboardKnowledgeSourceKind::Url | DashboardKnowledgeSourceKind::Note => {
            if input.file.is_some() {
                return Err(KnowledgeStoreError::Validation(
                    "file is only valid for file knowledge sources".into(),
                ));
            }
        }
    }
    Ok(())
}

pub(crate) fn decode_file_data(data_base64: &str) -> Result<Vec<u8>, KnowledgeStoreError> {
    STANDARD
        .decode(data_base64)
        .map_err(|e| KnowledgeStoreError::Validation(format!("file data is not base64: {e}")))
}

fn knowledge_error_response(err: KnowledgeStoreError) -> Response {
    match err {
        KnowledgeStoreError::NotFound => api_error_response(
            StatusCode::NOT_FOUND,
            ApiErrorCode::NotFound,
            "knowledge source not found".into(),
        ),
        KnowledgeStoreError::Validation(e) => {
            api_error_response(StatusCode::BAD_REQUEST, ApiErrorCode::Invalid, e)
        }
        KnowledgeStoreError::Internal(e) => {
            api_error_response(StatusCode::INTERNAL_SERVER_ERROR, ApiErrorCode::Internal, e)
        }
    }
}

fn api_error_response(status: StatusCode, code: ApiErrorCode, message: String) -> Response {
    let retriable = matches!(
        code,
        ApiErrorCode::RateLimited | ApiErrorCode::Internal | ApiErrorCode::Unavailable
    );
    let body = ApiError {
        code,
        message,
        retriable,
        details: json!(null),
    };
    (status, Json(body)).into_response()
}
