use std::sync::Arc;

use axum::{extract::State, Json};
#[allow(unused_imports)]
use tl_core::{ApiError, LlmRouteStatus, LlmRuntimeStatusResponse};
use tl_llm::{JudgeKind, LlmRouter};

#[derive(Clone)]
pub struct RuntimeStatusState {
    pub llm: Arc<LlmRouter>,
}

#[utoipa::path(
    get,
    path = "/v1/runtime/llm-status",
    tag = "runtime",
    responses(
        (status = 200, description = "Configured LLM judge routes", body = LlmRuntimeStatusResponse),
        (status = 401, description = "Missing or invalid API key", body = ApiError),
    ),
)]
pub async fn llm_status(State(state): State<RuntimeStatusState>) -> Json<LlmRuntimeStatusResponse> {
    let route = |kind: JudgeKind| LlmRouteStatus {
        judge: kind.as_str().to_string(),
        configured: state.llm.has_route(kind),
    };

    Json(LlmRuntimeStatusResponse {
        semantic_policy: route(JudgeKind::SemanticPolicy),
        harden_draft: route(JudgeKind::HardenDraft),
        trajectory_diagnostic: route(JudgeKind::TrajectoryDiagnostic),
        routes: JudgeKind::all().iter().copied().map(route).collect(),
    })
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use axum::extract::State;
    use tl_llm::{ProviderTarget, ResolvedRoute, TokenBudget};

    use super::*;

    fn router_with_routes(kinds: &[JudgeKind]) -> Arc<LlmRouter> {
        let routes = kinds
            .iter()
            .copied()
            .map(|kind| {
                (
                    kind,
                    ResolvedRoute {
                        primary: ProviderTarget {
                            provider: "test".into(),
                            model: kind.as_str().into(),
                            deadline_ms: 1_000,
                        },
                        fallback: None,
                    },
                )
            })
            .collect();
        Arc::new(LlmRouter::new(
            HashMap::new(),
            routes,
            Arc::new(TokenBudget::new(0)),
        ))
    }

    #[tokio::test]
    async fn llm_status_reports_configured_named_routes() {
        let Json(body) = llm_status(State(RuntimeStatusState {
            llm: router_with_routes(&[JudgeKind::SemanticPolicy, JudgeKind::HardenDraft]),
        }))
        .await;

        assert!(body.semantic_policy.configured);
        assert!(body.harden_draft.configured);
        assert!(!body.trajectory_diagnostic.configured);
        assert_eq!(body.routes.len(), JudgeKind::all().len());
        assert!(body
            .routes
            .iter()
            .any(|route| route.judge == "harden_draft" && route.configured));
    }
}
