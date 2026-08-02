use axum::{
    extract::State,
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};
#[allow(unused_imports)]
use tl_core::ApiError;
use tl_core::{ApiErrorCode, AuthResponse, OAuthIdentityRequest};

use super::response::api_error;
use super::{normalize_oauth_provider, AuthUserState};

/// `POST /v1/identity/oauth-session` — map a provider-authenticated
/// Google/GitHub account to a local Featherlane AI app user.
///
/// This endpoint is internal-only and accepts only the internal
/// `TL_API_KEY` bearer lane. User-session JWTs and workspace runtime keys
/// (`tl_live_...`) are rejected with `401`.
#[utoipa::path(
    post,
    path = "/v1/identity/oauth-session",
    tag = "auth",
    request_body = OAuthIdentityRequest,
    responses(
        (status = 200, description = "OAuth identity linked", body = AuthResponse),
        (status = 400, description = "Validation failed", body = ApiError),
        (status = 401, description = "Missing or invalid internal bearer token (`TL_API_KEY` only)", body = ApiError),
        (status = 500, description = "Internal error", body = ApiError),
    ),
)]
pub async fn oauth_session(
    State(state): State<AuthUserState>,
    Json(req): Json<OAuthIdentityRequest>,
) -> Response {
    let provider = match normalize_oauth_provider(&req.provider) {
        Ok(provider) => provider,
        Err(e) => {
            return api_error(
                StatusCode::BAD_REQUEST,
                ApiErrorCode::Invalid,
                e.to_string(),
            );
        }
    };
    let provider_subject = req.provider_subject.trim();
    if provider_subject.is_empty() {
        return api_error(
            StatusCode::BAD_REQUEST,
            ApiErrorCode::Invalid,
            "provider_subject is required".into(),
        );
    }
    let email = req.email.trim();
    if email.is_empty() {
        return api_error(
            StatusCode::BAD_REQUEST,
            ApiErrorCode::Invalid,
            "email is required".into(),
        );
    }

    let record = match state
        .store
        .ensure_oauth_identity(&provider, provider_subject, email)
        .await
    {
        Ok(record) => record,
        Err(e) => {
            return api_error(
                StatusCode::INTERNAL_SERVER_ERROR,
                ApiErrorCode::Internal,
                e.to_string(),
            );
        }
    };
    let jwt = state.mint_jwt(record.id, &record.username);
    tracing::info!(
        user_id = %record.id,
        username = %record.username,
        provider = %provider,
        "oauth identity linked"
    );
    Json(AuthResponse {
        user_id: record.id.to_string(),
        username: record.username,
        jwt,
    })
    .into_response()
}
