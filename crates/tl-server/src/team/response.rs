use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};
use serde_json::json;
use tl_core::{ApiError, ApiErrorCode};

use super::TeamStoreError;

pub(super) fn internal_error(error: TeamStoreError) -> Response {
    api_error(
        StatusCode::INTERNAL_SERVER_ERROR,
        ApiErrorCode::Internal,
        error.to_string(),
    )
}

pub(super) fn api_error(status: StatusCode, code: ApiErrorCode, message: String) -> Response {
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
