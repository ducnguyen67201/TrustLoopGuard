use axum::{
    extract::State,
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};
#[allow(unused_imports)]
use tl_core::ApiError;
use tl_core::{ApiErrorCode, AuthRequest, AuthResponse, ChangePasswordRequest};

use super::{
    password::{hash_password, verify_password},
    response::{api_error, invalid_credentials, password_auth_disabled},
    validation::{validate_password_hex, validate_username},
    AuthUserState, UserStoreError,
};

/// `POST /v1/auth/signup` — create a new account.
#[utoipa::path(
    post,
    path = "/v1/auth/signup",
    tag = "auth",
    request_body = AuthRequest,
    responses(
        (status = 201, description = "Account created", body = AuthResponse),
        (status = 400, description = "Validation failed", body = ApiError),
        (status = 404, description = "Password auth disabled", body = ApiError),
        (status = 409, description = "Username already exists", body = ApiError),
        (status = 500, description = "Internal error", body = ApiError),
    ),
)]
pub async fn signup(State(state): State<AuthUserState>, Json(req): Json<AuthRequest>) -> Response {
    if !state.password_auth_enabled {
        return password_auth_disabled();
    }

    if let Err(msg) = validate_username(&req.username) {
        return api_error(StatusCode::BAD_REQUEST, ApiErrorCode::Invalid, msg);
    }
    if let Err(msg) = validate_password_hex(&req.password) {
        return api_error(StatusCode::BAD_REQUEST, ApiErrorCode::Invalid, msg);
    }

    let hash = match hash_password(&req.password) {
        Ok(h) => h,
        Err(e) => {
            return api_error(
                StatusCode::INTERNAL_SERVER_ERROR,
                ApiErrorCode::Internal,
                e.to_string(),
            )
        }
    };

    let record = match state.store.create(req.username.trim(), &hash).await {
        Ok(record) => record,
        Err(UserStoreError::Conflict) => {
            return api_error(
                StatusCode::CONFLICT,
                ApiErrorCode::Unprocessable,
                "username already exists".into(),
            )
        }
        Err(e) => {
            return api_error(
                StatusCode::INTERNAL_SERVER_ERROR,
                ApiErrorCode::Internal,
                e.to_string(),
            )
        }
    };

    // Any pending invite for this email is auto-bound on the user's
    // first call to /v1/team/my-workspaces (see
    // TeamStore::accept_pending_invites_for_email).
    let jwt = state.mint_jwt(record.id, &record.username);
    tracing::info!(
        user_id = %record.id,
        username = %record.username,
        "auth signup succeeded"
    );

    (
        StatusCode::CREATED,
        Json(AuthResponse {
            user_id: record.id.to_string(),
            username: record.username,
            jwt,
        }),
    )
        .into_response()
}

/// `POST /v1/auth/login` — verify credentials.
#[utoipa::path(
    post,
    path = "/v1/auth/login",
    tag = "auth",
    request_body = AuthRequest,
    responses(
        (status = 200, description = "Credentials accepted", body = AuthResponse),
        (status = 400, description = "Validation failed", body = ApiError),
        (status = 401, description = "Invalid credentials", body = ApiError),
        (status = 404, description = "Password auth disabled", body = ApiError),
        (status = 500, description = "Internal error", body = ApiError),
    ),
)]
pub async fn login(State(state): State<AuthUserState>, Json(req): Json<AuthRequest>) -> Response {
    if !state.password_auth_enabled {
        return password_auth_disabled();
    }

    if let Err(msg) = validate_username(&req.username) {
        return api_error(StatusCode::BAD_REQUEST, ApiErrorCode::Invalid, msg);
    }
    if let Err(msg) = validate_password_hex(&req.password) {
        return api_error(StatusCode::BAD_REQUEST, ApiErrorCode::Invalid, msg);
    }

    let record = match state.store.find_by_username(req.username.trim()).await {
        Ok(r) => r,
        // Same response shape for NotFound and bad password so the
        // endpoint doesn't reveal which usernames exist.
        Err(UserStoreError::NotFound) => return invalid_credentials(),
        Err(e) => {
            return api_error(
                StatusCode::INTERNAL_SERVER_ERROR,
                ApiErrorCode::Internal,
                e.to_string(),
            )
        }
    };

    match verify_password(&req.password, &record.password_hash) {
        Ok(true) => {
            let jwt = state.mint_jwt(record.id, &record.username);
            tracing::info!(
                user_id = %record.id,
                username = %record.username,
                "auth login succeeded"
            );
            Json(AuthResponse {
                user_id: record.id.to_string(),
                username: record.username,
                jwt,
            })
            .into_response()
        }
        Ok(false) => invalid_credentials(),
        Err(e) => api_error(
            StatusCode::INTERNAL_SERVER_ERROR,
            ApiErrorCode::Internal,
            e.to_string(),
        ),
    }
}

/// `POST /v1/auth/password` — change an existing user's password.
///
/// The caller must demonstrate knowledge of the current password by
/// including it in the request. tl-server does not issue per-user
/// session tokens (see `docs/concept/authorization.md`); the
/// current-password check is what proves account ownership here.
#[utoipa::path(
    post,
    path = "/v1/auth/password",
    tag = "auth",
    request_body = ChangePasswordRequest,
    responses(
        (status = 200, description = "Password updated", body = AuthResponse),
        (status = 400, description = "Validation failed", body = ApiError),
        (status = 401, description = "Current password did not match", body = ApiError),
        (status = 404, description = "Password auth disabled", body = ApiError),
        (status = 500, description = "Internal error", body = ApiError),
    ),
)]
pub async fn change_password(
    State(state): State<AuthUserState>,
    Json(req): Json<ChangePasswordRequest>,
) -> Response {
    if !state.password_auth_enabled {
        return password_auth_disabled();
    }

    if let Err(msg) = validate_username(&req.username) {
        return api_error(StatusCode::BAD_REQUEST, ApiErrorCode::Invalid, msg);
    }
    if let Err(msg) = validate_password_hex(&req.current_password) {
        return api_error(StatusCode::BAD_REQUEST, ApiErrorCode::Invalid, msg);
    }
    if let Err(msg) = validate_password_hex(&req.new_password) {
        return api_error(StatusCode::BAD_REQUEST, ApiErrorCode::Invalid, msg);
    }
    if req.current_password == req.new_password {
        return api_error(
            StatusCode::BAD_REQUEST,
            ApiErrorCode::Invalid,
            "new password must differ from current password".into(),
        );
    }

    let record = match state.store.find_by_username(req.username.trim()).await {
        Ok(r) => r,
        Err(UserStoreError::NotFound) => return invalid_credentials(),
        Err(e) => {
            return api_error(
                StatusCode::INTERNAL_SERVER_ERROR,
                ApiErrorCode::Internal,
                e.to_string(),
            )
        }
    };

    match verify_password(&req.current_password, &record.password_hash) {
        Ok(true) => {}
        Ok(false) => return invalid_credentials(),
        Err(e) => {
            return api_error(
                StatusCode::INTERNAL_SERVER_ERROR,
                ApiErrorCode::Internal,
                e.to_string(),
            )
        }
    }

    let new_hash = match hash_password(&req.new_password) {
        Ok(h) => h,
        Err(e) => {
            return api_error(
                StatusCode::INTERNAL_SERVER_ERROR,
                ApiErrorCode::Internal,
                e.to_string(),
            )
        }
    };

    match state.store.update_password(record.id, &new_hash).await {
        Ok(()) => {
            let jwt = state.mint_jwt(record.id, &record.username);
            Json(AuthResponse {
                user_id: record.id.to_string(),
                username: record.username,
                jwt,
            })
            .into_response()
        }
        Err(UserStoreError::NotFound) => invalid_credentials(),
        Err(e) => api_error(
            StatusCode::INTERNAL_SERVER_ERROR,
            ApiErrorCode::Internal,
            e.to_string(),
        ),
    }
}
