use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};
use serde_json::json;
use tl_core::{ApiError, ApiErrorCode};

use super::FinancialStoreError;

pub(super) fn financial_error_response(error: FinancialStoreError) -> Response {
    let (status, code) = match error {
        FinancialStoreError::NotFound => (StatusCode::NOT_FOUND, ApiErrorCode::NotFound),
        FinancialStoreError::Conflict => (StatusCode::CONFLICT, ApiErrorCode::Unprocessable),
        FinancialStoreError::Validation(_) => (StatusCode::BAD_REQUEST, ApiErrorCode::Invalid),
        FinancialStoreError::Internal(_) => {
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
