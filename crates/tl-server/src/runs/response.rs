use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};
use serde_json::json;
use tl_core::{ApiError, ApiErrorCode};

use super::RunStoreError;

pub(super) fn run_error_response(error: RunStoreError) -> Response {
    let (status, code) = match error {
        RunStoreError::NotFound => (StatusCode::NOT_FOUND, ApiErrorCode::NotFound),
        RunStoreError::Validation(_) => (StatusCode::BAD_REQUEST, ApiErrorCode::Invalid),
        RunStoreError::Internal(_) => (StatusCode::INTERNAL_SERVER_ERROR, ApiErrorCode::Internal),
    };
    crate::log_api_error(status, code, &error.to_string());
    let body = ApiError {
        code,
        message: error.to_string(),
        retriable: matches!(code, ApiErrorCode::RateLimited | ApiErrorCode::Unavailable),
        details: json!(null),
    };
    (status, Json(body)).into_response()
}
