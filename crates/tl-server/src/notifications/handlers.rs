use std::sync::Arc;

use axum::{
    extract::{Path, State},
    http::{HeaderMap, StatusCode},
    response::{IntoResponse, Response},
    Extension, Json,
};
use serde_json::json;
use tl_core::{
    ApiError, ApiErrorCode, CreateNotificationRuleRequest, NotificationDeliveryListResponse,
    NotificationEventKind, NotificationReadiness, NotificationRule, NotificationRuleListResponse,
    UpdateNotificationRuleRequest,
};

use super::{EnqueueNotification, NotificationStore, NotificationStoreError};
use crate::environments::EnvironmentStore;

#[derive(Clone)]
pub struct NotificationState {
    pub store: Arc<dyn NotificationStore>,
    pub environment_store: Arc<dyn EnvironmentStore>,
    pub transport_configured: bool,
}

#[utoipa::path(get, path = "/v1/notifications/readiness", tag = "notifications", responses((status = 200, body = NotificationReadiness)))]
pub async fn notification_readiness(
    State(state): State<NotificationState>,
    runtime_key: Option<Extension<crate::auth::WorkspaceKeyContext>>,
    headers: HeaderMap,
) -> Response {
    if let Some(response) = reject_runtime_key(runtime_key) {
        return response;
    }
    if let Err(response) = context(&state, &headers).await {
        return response;
    }
    Json(NotificationReadiness {
        configured: state.transport_configured,
        detail: (!state.transport_configured).then(|| {
            "Configure TL_NOTIFICATION_SMTP_URL and TL_NOTIFICATION_EMAIL_FROM".to_string()
        }),
    })
    .into_response()
}

#[utoipa::path(get, path = "/v1/notification-rules", tag = "notifications", responses((status = 200, body = NotificationRuleListResponse)))]
pub async fn list_notification_rules(
    State(state): State<NotificationState>,
    runtime_key: Option<Extension<crate::auth::WorkspaceKeyContext>>,
    headers: HeaderMap,
) -> Response {
    if let Some(response) = reject_runtime_key(runtime_key) {
        return response;
    }
    let (workspace_id, environment_id) = match context(&state, &headers).await {
        Ok(value) => value,
        Err(response) => return response,
    };
    match state.store.list_rules(&workspace_id, &environment_id).await {
        Ok(notification_rules) => {
            Json(NotificationRuleListResponse { notification_rules }).into_response()
        }
        Err(error) => error_response(error),
    }
}

#[utoipa::path(post, path = "/v1/notification-rules", tag = "notifications", request_body = CreateNotificationRuleRequest, responses((status = 201, body = NotificationRule)))]
pub async fn create_notification_rule(
    State(state): State<NotificationState>,
    runtime_key: Option<Extension<crate::auth::WorkspaceKeyContext>>,
    headers: HeaderMap,
    Json(input): Json<CreateNotificationRuleRequest>,
) -> Response {
    if let Some(response) = reject_runtime_key(runtime_key) {
        return response;
    }
    if let Err(error) = validate_rule(&input.email, &input.event_kinds) {
        return error_response(error);
    }
    let (workspace_id, environment_id) = match context(&state, &headers).await {
        Ok(value) => value,
        Err(response) => return response,
    };
    if input.enabled && !state.transport_configured {
        return transport_unavailable_response();
    }
    match state
        .store
        .create_rule(&workspace_id, &environment_id, None, input)
        .await
    {
        Ok(rule) => (StatusCode::CREATED, Json(rule)).into_response(),
        Err(error) => error_response(error),
    }
}

#[utoipa::path(patch, path = "/v1/notification-rules/{id}", tag = "notifications", params(("id" = String, Path)), request_body = UpdateNotificationRuleRequest, responses((status = 200, body = NotificationRule)))]
pub async fn patch_notification_rule(
    State(state): State<NotificationState>,
    runtime_key: Option<Extension<crate::auth::WorkspaceKeyContext>>,
    headers: HeaderMap,
    Path(id): Path<String>,
    Json(input): Json<UpdateNotificationRuleRequest>,
) -> Response {
    if let Some(response) = reject_runtime_key(runtime_key) {
        return response;
    }
    if input
        .email
        .as_deref()
        .is_some_and(|email| !valid_email(email))
        || input.event_kinds.as_ref().is_some_and(Vec::is_empty)
    {
        return error_response(NotificationStoreError::Validation(
            "email must be valid and event_kinds cannot be empty".into(),
        ));
    }
    let (workspace_id, _) = match context(&state, &headers).await {
        Ok(value) => value,
        Err(response) => return response,
    };
    if input.enabled == Some(true) && !state.transport_configured {
        return transport_unavailable_response();
    }
    match state.store.update_rule(&workspace_id, &id, input).await {
        Ok(rule) => Json(rule).into_response(),
        Err(error) => error_response(error),
    }
}

#[utoipa::path(delete, path = "/v1/notification-rules/{id}", tag = "notifications", params(("id" = String, Path)), responses((status = 204)))]
pub async fn delete_notification_rule(
    State(state): State<NotificationState>,
    runtime_key: Option<Extension<crate::auth::WorkspaceKeyContext>>,
    headers: HeaderMap,
    Path(id): Path<String>,
) -> Response {
    if let Some(response) = reject_runtime_key(runtime_key) {
        return response;
    }
    let (workspace_id, _) = match context(&state, &headers).await {
        Ok(value) => value,
        Err(response) => return response,
    };
    match state.store.delete_rule(&workspace_id, &id).await {
        Ok(()) => StatusCode::NO_CONTENT.into_response(),
        Err(error) => error_response(error),
    }
}

#[utoipa::path(get, path = "/v1/notification-deliveries", tag = "notifications", responses((status = 200, body = NotificationDeliveryListResponse)))]
pub async fn list_notification_deliveries(
    State(state): State<NotificationState>,
    runtime_key: Option<Extension<crate::auth::WorkspaceKeyContext>>,
    headers: HeaderMap,
) -> Response {
    if let Some(response) = reject_runtime_key(runtime_key) {
        return response;
    }
    let (workspace_id, environment_id) = match context(&state, &headers).await {
        Ok(value) => value,
        Err(response) => return response,
    };
    match state
        .store
        .list_deliveries(&workspace_id, &environment_id, 200)
        .await
    {
        Ok(deliveries) => Json(NotificationDeliveryListResponse { deliveries }).into_response(),
        Err(error) => error_response(error),
    }
}

#[utoipa::path(post, path = "/v1/notification-rules/{id}/test", tag = "notifications", params(("id" = String, Path)), responses((status = 202, body = NotificationDeliveryListResponse)))]
pub async fn test_notification(
    State(state): State<NotificationState>,
    runtime_key: Option<Extension<crate::auth::WorkspaceKeyContext>>,
    headers: HeaderMap,
    Path(id): Path<String>,
) -> Response {
    if let Some(response) = reject_runtime_key(runtime_key) {
        return response;
    }
    let (workspace_id, environment_id) = match context(&state, &headers).await {
        Ok(value) => value,
        Err(response) => return response,
    };
    if !state.transport_configured {
        return transport_unavailable_response();
    }
    let subject_id = uuid::Uuid::now_v7().to_string();
    match state.store.enqueue(EnqueueNotification {
        workspace_id: workspace_id.clone(), environment_id: environment_id.clone(), agent_id: None,
        rule_id: Some(id), event_kind: NotificationEventKind::Test, subject_id,
        subject_version: "v1".into(), run_id: None,
        payload: json!({"title":"Featherlane notification test","detail":"Your production-loop email transport is working."}),
    }).await {
        Ok(1) => {
            let deliveries = state.store.list_deliveries(&workspace_id, &environment_id, 1).await.unwrap_or_default();
            (StatusCode::ACCEPTED, Json(NotificationDeliveryListResponse { deliveries })).into_response()
        }
        Ok(_) => error_response(NotificationStoreError::NotFound),
        Err(error) => error_response(error),
    }
}

async fn context(
    state: &NotificationState,
    headers: &HeaderMap,
) -> Result<(String, String), Response> {
    let workspace_id = crate::policies::workspace_id_from_headers(headers)?;
    let environment_id = crate::environments::resolve_environment_id(
        headers,
        state.environment_store.as_ref(),
        &workspace_id,
    )
    .await
    .map_err(crate::environments::environment_error_response)?;
    Ok((workspace_id, environment_id))
}

fn validate_rule(
    email: &str,
    event_kinds: &[NotificationEventKind],
) -> Result<(), NotificationStoreError> {
    if !valid_email(email) {
        return Err(NotificationStoreError::Validation(
            "email must be a valid address".into(),
        ));
    }
    if event_kinds.is_empty() {
        return Err(NotificationStoreError::Validation(
            "event_kinds cannot be empty".into(),
        ));
    }
    Ok(())
}

fn valid_email(value: &str) -> bool {
    let value = value.trim();
    value.len() <= 320
        && value
            .split_once('@')
            .is_some_and(|(local, domain)| !local.is_empty() && domain.contains('.'))
}

fn reject_runtime_key(
    runtime_key: Option<Extension<crate::auth::WorkspaceKeyContext>>,
) -> Option<Response> {
    runtime_key.map(|_| {
        crate::app::error::api_error_response(
            StatusCode::FORBIDDEN,
            ApiErrorCode::Forbidden,
            "workspace runtime keys cannot manage notifications".into(),
        )
    })
}

fn transport_unavailable_response() -> Response {
    (
        StatusCode::CONFLICT,
        Json(ApiError {
            code: ApiErrorCode::Unavailable,
            message: "email transport is not configured".into(),
            retriable: true,
            details: json!({
                "required_environment": [
                    "TL_NOTIFICATION_SMTP_URL",
                    "TL_NOTIFICATION_EMAIL_FROM"
                ]
            }),
        }),
    )
        .into_response()
}

pub(crate) fn error_response(error: NotificationStoreError) -> Response {
    let (status, code, message) = match error {
        NotificationStoreError::NotFound => (
            StatusCode::NOT_FOUND,
            ApiErrorCode::NotFound,
            "notification resource not found".to_string(),
        ),
        NotificationStoreError::Conflict => (
            StatusCode::CONFLICT,
            ApiErrorCode::Conflict,
            "notification resource conflict".to_string(),
        ),
        NotificationStoreError::Validation(message) => {
            (StatusCode::BAD_REQUEST, ApiErrorCode::Invalid, message)
        }
        NotificationStoreError::Internal(message) => {
            tracing::error!(error = %message, "notification store failed");
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                ApiErrorCode::Internal,
                "notification operation failed".to_string(),
            )
        }
    };
    (
        status,
        Json(ApiError {
            code,
            message,
            retriable: code.default_retriable(),
            details: serde_json::Value::Null,
        }),
    )
        .into_response()
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use axum::{
        body::Body,
        http::{header, Request, StatusCode},
        routing::{get, post},
        Router,
    };
    use http_body_util::BodyExt;
    use serde_json::{json, Value};
    use tower::ServiceExt;

    use crate::{environments::MemoryEnvironmentStore, notifications::MemoryNotificationStore};

    use super::{create_notification_rule, notification_readiness, NotificationState};

    fn app(configured: bool) -> Router {
        let state = NotificationState {
            store: Arc::new(MemoryNotificationStore::new()),
            environment_store: Arc::new(MemoryEnvironmentStore::new()),
            transport_configured: configured,
        };
        Router::new()
            .route("/v1/notification-rules", post(create_notification_rule))
            .route("/v1/notifications/readiness", get(notification_readiness))
            .with_state(state)
    }

    async fn body(response: axum::response::Response) -> Value {
        let bytes = response.into_body().collect().await.unwrap().to_bytes();
        serde_json::from_slice(&bytes).unwrap()
    }

    fn request(method: &str, path: &str, body: Value) -> Request<Body> {
        Request::builder()
            .method(method)
            .uri(path)
            .header(header::CONTENT_TYPE, "application/json")
            .header("x-featherlane-ai-workspace-id", "workspace-1")
            .body(Body::from(body.to_string()))
            .unwrap()
    }

    #[tokio::test]
    async fn readiness_reports_missing_transport_and_enabled_rules_are_rejected() {
        let app = app(false);
        let readiness = app
            .clone()
            .oneshot(request("GET", "/v1/notifications/readiness", json!({})))
            .await
            .unwrap();
        assert_eq!(readiness.status(), StatusCode::OK);
        assert_eq!(body(readiness).await["configured"], false);

        let enabled = app
            .clone()
            .oneshot(request(
                "POST",
                "/v1/notification-rules",
                json!({
                    "email": "ops@example.com",
                    "event_kinds": ["evaluation_failed"],
                    "enabled": true
                }),
            ))
            .await
            .unwrap();
        assert_eq!(enabled.status(), StatusCode::CONFLICT);
        assert_eq!(body(enabled).await["code"], "unavailable");

        let draft = app
            .oneshot(request(
                "POST",
                "/v1/notification-rules",
                json!({
                    "email": "ops@example.com",
                    "event_kinds": ["evaluation_failed"],
                    "enabled": false
                }),
            ))
            .await
            .unwrap();
        assert_eq!(draft.status(), StatusCode::CREATED);
    }
}
