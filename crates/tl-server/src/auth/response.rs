use axum::{
    http::{header::WWW_AUTHENTICATE, HeaderValue, StatusCode},
    response::{IntoResponse, Response},
    Json,
};
use tl_core::{ApiError, ApiErrorCode};

pub(super) fn unauthorized(message: &str) -> Response {
    let mut response = api_error(
        StatusCode::UNAUTHORIZED,
        ApiErrorCode::Unauthorized,
        message,
    );
    // RFC 9728: advertise the protected-resource metadata so MCP clients can
    // discover the authorization server and start the OAuth flow on a 401.
    let issuer =
        std::env::var("TL_PUBLIC_URL").unwrap_or_else(|_| "http://localhost:8080".to_string());
    let challenge =
        format!("Bearer resource_metadata=\"{issuer}/.well-known/oauth-protected-resource\"");
    if let Ok(value) = HeaderValue::from_str(&challenge) {
        response.headers_mut().insert(WWW_AUTHENTICATE, value);
    }
    response
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
