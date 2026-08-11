use std::sync::Arc;

use axum::{
    extract::DefaultBodyLimit,
    middleware::{from_fn, from_fn_with_state},
    routing::post,
    Router,
};

use crate::{auth, AppState, AuthConfig};

use super::{middleware::log_http_response, route_groups};

/// Build the application router.
///
/// `auth` is optional so deployments without exposed endpoints (local
/// dev, integration tests) can run without setting `TL_API_KEY`. When
/// `Some`, every `/v1/*` route requires `Authorization: Bearer <key>`;
/// `/health` is always public so liveness probes don't need a secret.
///
/// The route groups live in `route_groups` so this function stays focused on
/// top-level composition, auth layering, and shared middleware.
pub fn router(
    state: AppState,
    auth: Option<Arc<AuthConfig>>,
    gateway_seal_key: [u8; 32],
) -> Router {
    let jwt_signer = state.jwt_signer.clone();
    let mcp_jwt_signer = jwt_signer.clone();
    let mcp_auth = auth.clone();
    let api_key_store = state.api_key_store.clone();
    let user_store = state.user_store.clone();

    let public = route_groups::public_routes(&state, jwt_signer.clone())
        .merge(crate::oauth::oauth_public_routes(state.clone()));
    let trusted_identity_routes = route_groups::auth_identity_routes(&state, jwt_signer.clone())
        .merge(crate::oauth::oauth_protected_routes(state.clone()));

    let mut protected = Router::new()
        .route(
            "/v1/events",
            // A valid event tops out around ~200 KiB under the
            // event_service limits; cap the body well above that and
            // far below axum's 2 MiB default so oversized payloads are
            // rejected before deserialization.
            post(crate::api::events::submit_event).layer(DefaultBodyLimit::max(512 * 1024)),
        )
        .route(
            "/v1/policies/validate",
            post(crate::policies::validate_policy),
        )
        .with_state(state.clone())
        .merge(route_groups::agent_routes(&state))
        .merge(route_groups::evaluation_routes(&state))
        .merge(route_groups::notification_routes(&state))
        .merge(route_groups::otel_routes(&state))
        .merge(route_groups::tool_metadata_routes(&state))
        .merge(route_groups::authorization_routes(&state))
        .merge(route_groups::label_policy_routes(&state))
        .merge(route_groups::policy_routes(&state))
        .merge(route_groups::guardrail_routes(&state))
        .merge(route_groups::run_routes(&state))
        .merge(route_groups::redteam_routes(&state))
        .merge(route_groups::github_integration_routes(&state))
        .merge(route_groups::trace_routes(&state))
        .merge(route_groups::analytics_routes(&state))
        .merge(route_groups::human_review_routes(&state))
        .merge(route_groups::financial_routes(&state, gateway_seal_key))
        .merge(route_groups::budget_alert_routes(&state))
        .merge(route_groups::llm_usage_routes(&state))
        .merge(route_groups::llm_pricing_routes(&state))
        .merge(route_groups::dashboard_admin_routes(&state))
        .merge(route_groups::environment_routes(&state))
        .merge(route_groups::gateway_routes(&state, gateway_seal_key))
        .merge(route_groups::mcp_gateway_routes(&state, gateway_seal_key))
        .merge(route_groups::knowledge_routes(&state))
        .merge(route_groups::team_routes(&state));

    if let Some(cfg) = auth {
        let cfg = cfg.with_jwt(jwt_signer);
        let cfg = cfg.with_workspace_keys(Some(api_key_store));
        let cfg = cfg.with_user_approval(Some(user_store));
        protected = protected.layer(from_fn_with_state(cfg.clone(), auth::require_bearer));

        let trusted_identity_routes =
            trusted_identity_routes.layer(from_fn_with_state(cfg, auth::require_internal_bearer));
        protected = protected.merge(trusted_identity_routes);
    } else {
        protected = protected.merge(trusted_identity_routes);
    }

    let mut app = public.merge(protected);
    match (mcp_auth, mcp_jwt_signer) {
        (Some(config), Some(signer)) => {
            app = app.merge(route_groups::mcp_resource_routes(
                &state,
                config.with_jwt(Some(signer)),
                gateway_seal_key,
            ));
        }
        _ => tracing::info!(
            "hosted /mcp resource disabled because bearer auth or JWT signing is not configured"
        ),
    }
    app.layer(from_fn(log_http_response))
}
