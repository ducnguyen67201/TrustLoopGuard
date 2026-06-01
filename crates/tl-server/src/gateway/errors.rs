use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};
use serde_json::Value;
use tl_core::{ApiError, ApiErrorCode};

use super::store::GatewayStoreError;

pub(super) fn gateway_store_error_response(error: GatewayStoreError) -> Response {
    match error {
        GatewayStoreError::NotFound => {
            api_error_response(StatusCode::NOT_FOUND, "gateway resource not found".into())
        }
        GatewayStoreError::Internal(message) => {
            api_error_response(StatusCode::INTERNAL_SERVER_ERROR, message)
        }
    }
}

pub(super) fn api_error_response(status: StatusCode, message: String) -> Response {
    crate::log_api_error(status, ApiErrorCode::Invalid, &message);
    let code = if status == StatusCode::NOT_FOUND {
        ApiErrorCode::NotFound
    } else if status == StatusCode::UNAUTHORIZED {
        ApiErrorCode::Unauthorized
    } else if status == StatusCode::FORBIDDEN {
        ApiErrorCode::Forbidden
    } else if status == StatusCode::BAD_GATEWAY {
        ApiErrorCode::Unavailable
    } else if status.is_server_error() {
        ApiErrorCode::Internal
    } else {
        ApiErrorCode::Invalid
    };
    let retriable = matches!(
        code,
        ApiErrorCode::RateLimited | ApiErrorCode::Internal | ApiErrorCode::Unavailable
    );
    (
        status,
        Json(ApiError {
            code,
            message,
            retriable,
            details: Value::Null,
        }),
    )
        .into_response()
}
