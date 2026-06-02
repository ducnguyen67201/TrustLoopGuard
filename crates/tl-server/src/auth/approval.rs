use axum::{extract::Request, response::Response};

use crate::auth_user::UserStoreError;

use super::{
    response::{forbidden, internal_error},
    AuthConfig,
};

pub(super) fn forwarded_user_id(req: &Request) -> Option<uuid::Uuid> {
    req.headers()
        .get("x-tlg-user-id")
        .and_then(|value| value.to_str().ok())
        .and_then(|value| uuid::Uuid::parse_str(value.trim()).ok())
}

pub(super) async fn require_approved_user(
    cfg: &AuthConfig,
    user_id: uuid::Uuid,
) -> Result<(), Response> {
    if !cfg.hosted_user_approval_required {
        return Ok(());
    }

    let Some(store) = cfg.user_store.as_ref() else {
        tracing::error!(
            user_id = %user_id,
            "hosted approval gate enabled without a user store"
        );
        return Err(forbidden(
            "user approval is required for this hosted deployment",
        ));
    };

    match store.is_approved(user_id).await {
        Ok(true) => Ok(()),
        Ok(false) | Err(UserStoreError::NotFound) => {
            Err(forbidden("user is not approved for this hosted deployment"))
        }
        Err(e) => {
            tracing::error!(
                user_id = %user_id,
                error = %e,
                "user approval lookup failed"
            );
            Err(internal_error("user approval lookup failed"))
        }
    }
}
