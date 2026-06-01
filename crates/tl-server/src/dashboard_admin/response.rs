use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};
use tl_core::{ApiError, ApiErrorCode};

pub(super) fn api_error_response(
    status: StatusCode,
    code: ApiErrorCode,
    message: String,
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
        details: serde_json::Value::Null,
    };
    (status, Json(body)).into_response()
}
