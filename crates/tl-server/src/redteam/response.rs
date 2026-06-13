use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};
use serde_json::json;
use tl_core::{ApiError, ApiErrorCode};

use super::RedteamJobStoreError;

pub(super) fn job_error_response(error: RedteamJobStoreError) -> Response {
    let (status, code) = match error {
        RedteamJobStoreError::NotFound => (StatusCode::NOT_FOUND, ApiErrorCode::NotFound),
        RedteamJobStoreError::Validation(_) => (StatusCode::BAD_REQUEST, ApiErrorCode::Invalid),
        RedteamJobStoreError::Unavailable(_) => {
            (StatusCode::SERVICE_UNAVAILABLE, ApiErrorCode::Unavailable)
        }
        RedteamJobStoreError::Internal(_) => {
            (StatusCode::INTERNAL_SERVER_ERROR, ApiErrorCode::Internal)
        }
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
