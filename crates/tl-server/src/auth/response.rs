use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};
use tl_core::{ApiError, ApiErrorCode};

pub(super) fn unauthorized(message: &str) -> Response {
    api_error(
        StatusCode::UNAUTHORIZED,
        ApiErrorCode::Unauthorized,
        message,
    )
}

pub(super) fn forbidden(message: &str) -> Response {
    api_error(StatusCode::FORBIDDEN, ApiErrorCode::Forbidden, message)
}

pub(super) fn internal_error(message: &str) -> Response {
    api_error(
        StatusCode::INTERNAL_SERVER_ERROR,
        ApiErrorCode::Internal,
        message,
    )
}

fn api_error(status: StatusCode, code: ApiErrorCode, message: &str) -> Response {
    let body = ApiError {
        code,
        message: message.into(),
        retriable: false,
        details: serde_json::Value::Null,
    };
    (status, Json(body)).into_response()
}
