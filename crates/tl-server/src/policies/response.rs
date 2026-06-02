use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};
use serde_json::json;
use tl_core::{ApiError, ApiErrorCode};

use super::PolicyStoreError;

pub(super) fn api_error_response(
    status: StatusCode,
    code: ApiErrorCode,
    message: String,
) -> Response {
    api_error_response_with_details(status, code, message, json!(null))
}

pub(crate) fn api_error_response_with_details(
    status: StatusCode,
    code: ApiErrorCode,
    message: String,
    details: serde_json::Value,
) -> Response {
    crate::log_api_error(status, code, &message);
    let retriable = matches!(
        code,
        ApiErrorCode::RateLimited | ApiErrorCode::Internal | ApiErrorCode::Unavailable
    );
    let body = ApiError {
        code,
        message,
        retriable,
        details,
    };
    (status, Json(body)).into_response()
}

pub(super) fn policy_store_error_response(err: PolicyStoreError) -> Response {
    match err {
        PolicyStoreError::NotFound => api_error_response(
            StatusCode::NOT_FOUND,
            ApiErrorCode::NotFound,
            "policy not found".into(),
        ),
        PolicyStoreError::Internal(e) => {
            api_error_response(StatusCode::INTERNAL_SERVER_ERROR, ApiErrorCode::Internal, e)
        }
    }
}
