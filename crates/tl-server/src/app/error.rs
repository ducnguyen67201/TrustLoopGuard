use axum::{http::StatusCode, response::IntoResponse, response::Response, Json};
use serde_json::json;
use tl_core::{ApiError, ApiErrorCode};

pub(crate) fn api_error_response(
    status: StatusCode,
    code: ApiErrorCode,
    message: String,
) -> Response {
    log_api_error(status, code, &message);
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

pub(crate) fn log_api_error(status: StatusCode, code: ApiErrorCode, message: &str) {
    let status = status.as_u16();
    if status >= 500 {
        tracing::error!(status, code = ?code, error = %message, "api error response");
    } else if status >= 400 {
        tracing::warn!(status, code = ?code, error = %message, "api error response");
    } else {
        tracing::info!(status, code = ?code, message = %message, "api response");
    }
}
