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
    let api_key_store = state.api_key_store.clone();
    let user_store = state.user_store.clone();
    let hosted_user_approval_required = state.hosted_user_approval_required;

    let public = route_groups::public_routes(&state, jwt_signer.clone());
    let auth_identity_routes = route_groups::auth_identity_routes(&state, jwt_signer.clone());

    let draft_llm = build_policy_draft_llm();
    let draft_model =
        std::env::var("TL_POLICY_DRAFT_MODEL").unwrap_or_else(|_| "gpt-4o-mini".to_string());

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
        .merge(route_groups::tool_metadata_routes(&state))
        .merge(route_groups::label_policy_routes(&state))
        .merge(route_groups::policy_routes(
            &state,
            draft_llm.clone(),
            draft_model.clone(),
        ))
        .merge(route_groups::guardrail_routes(
            &state,
            draft_llm,
            draft_model,
        ))
        .merge(route_groups::run_routes(&state))
        .merge(route_groups::bench_routes(&state))
        .merge(route_groups::redteam_routes(&state))
        .merge(route_groups::trace_routes(&state))
        .merge(route_groups::analytics_routes(&state))
        .merge(route_groups::human_review_routes(&state))
        .merge(route_groups::dashboard_admin_routes(&state))
        .merge(route_groups::environment_routes(&state))
        .merge(route_groups::gateway_routes(&state, gateway_seal_key))
        .merge(route_groups::knowledge_routes(&state))
        .merge(route_groups::team_routes(&state));

    if let Some(cfg) = auth {
        let cfg = cfg.with_jwt(jwt_signer);
        let cfg = cfg.with_workspace_keys(Some(api_key_store));
        let cfg = cfg.with_user_approval(Some(user_store), hosted_user_approval_required);
        protected = protected.layer(from_fn_with_state(cfg.clone(), auth::require_bearer));

        let auth_identity_routes =
            auth_identity_routes.layer(from_fn_with_state(cfg, auth::require_internal_bearer));
        protected = protected.merge(auth_identity_routes);
    } else {
        protected = protected.merge(auth_identity_routes);
    }

    public.merge(protected).layer(from_fn(log_http_response))
}

fn build_policy_draft_llm() -> Option<Arc<dyn tl_llm::LlmClient>> {
    match tl_llm::OpenAiClient::from_env() {
        Ok(client) => {
            tracing::info!("policy-draft LLM enabled (OpenAI)");
            Some(Arc::new(client))
        }
        Err(e) => {
            tracing::info!(
                error = %e,
                "policy-draft LLM not configured; POST /v1/policies/draft will return 503"
            );
            None
        }
    }
}
