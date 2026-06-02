use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};
use serde_json::json;
use tl_core::{ApiError, ApiErrorCode};

use super::HumanReviewStoreError;

pub(super) fn review_error_response(error: HumanReviewStoreError) -> Response {
    let (status, code) = match error {
        HumanReviewStoreError::NotFound => (StatusCode::NOT_FOUND, ApiErrorCode::NotFound),
        HumanReviewStoreError::Validation(_) => (StatusCode::BAD_REQUEST, ApiErrorCode::Invalid),
        HumanReviewStoreError::Internal(_) => {
            (StatusCode::INTERNAL_SERVER_ERROR, ApiErrorCode::Internal)
        }
    };
    crate::log_api_error(status, code, &error.to_string());
    let retriable = matches!(code, ApiErrorCode::Internal | ApiErrorCode::Unavailable);
    let body = ApiError {
        code,
        message: error.to_string(),
        retriable,
        details: json!(null),
    };
    (status, Json(body)).into_response()
}
