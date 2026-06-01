use axum::{http::HeaderMap, Extension};
use uuid::Uuid;

use crate::jwt::UserContext;

pub(super) const X_USER_HEADER: &str = "x-tlg-user-id";
pub(super) const X_USER_EMAIL_HEADER: &str = "x-tlg-user-email";

pub(super) fn request_user_id(
    headers: &HeaderMap,
    user: Option<Extension<UserContext>>,
) -> Option<Uuid> {
    user.map(|Extension(ctx)| ctx.user_id).or_else(|| {
        headers
            .get(X_USER_HEADER)
            .and_then(|value| value.to_str().ok())
            .and_then(|value| Uuid::parse_str(value.trim()).ok())
    })
}
