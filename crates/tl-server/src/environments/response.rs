use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};
use serde_json::json;
use tl_core::{ApiError, ApiErrorCode};

use super::EnvironmentStoreError;

pub(crate) fn environment_error_response(error: EnvironmentStoreError) -> Response {
    let (status, code) = match error {
        EnvironmentStoreError::NotFound => (StatusCode::NOT_FOUND, ApiErrorCode::NotFound),
        EnvironmentStoreError::Validation(_) => (StatusCode::BAD_REQUEST, ApiErrorCode::Invalid),
        EnvironmentStoreError::Internal(_) => {
            (StatusCode::INTERNAL_SERVER_ERROR, ApiErrorCode::Internal)
        }
    };
    crate::log_api_error(status, code, &error.to_string());
    let body = ApiError {
        code,
        message: error.to_string(),
        retriable: false,
        details: json!(null),
    };
    (status, Json(body)).into_response()
}
