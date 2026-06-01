use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};
use serde_json::json;
use tl_core::{ApiError, ApiErrorCode};

pub(super) fn invalid_credentials() -> Response {
    api_error(
        StatusCode::UNAUTHORIZED,
        ApiErrorCode::Unauthorized,
        "invalid username or password".into(),
    )
}

pub(super) fn password_auth_disabled() -> Response {
    api_error(
        StatusCode::NOT_FOUND,
        ApiErrorCode::NotFound,
        "username/password auth is disabled for this deployment".into(),
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
