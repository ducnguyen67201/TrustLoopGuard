use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};
use serde_json::json;
use tl_core::{ApiError, ApiErrorCode};

use super::KnowledgeStoreError;

pub(super) fn knowledge_error_response(err: KnowledgeStoreError) -> Response {
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
    crate::log_api_error(status, code, &message);
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
