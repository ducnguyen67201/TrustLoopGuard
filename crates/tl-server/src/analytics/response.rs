use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};
use serde_json::json;
use tl_core::{ApiError, ApiErrorCode};

use super::AnalyticsStoreError;

pub(super) fn analytics_error_response(error: AnalyticsStoreError) -> Response {
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
