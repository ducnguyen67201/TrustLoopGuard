use axum::{
    extract::{Extension, Path, State},
    http::HeaderMap,
    response::{IntoResponse, Response},
    Json,
};
use tl_core::{
    AgentEvaluationPolicyAssignmentListResponse, EvaluationJobStatus, EvaluationResultListResponse,
    PutAgentEvaluationPolicyAssignmentsRequest, PutAgentEvaluationProfileRequest,
    ReevaluateRunRequest, ReevaluateRunResponse,
};

use super::{evaluation_error_response, EvaluationState};
use crate::{
    auth::{InternalServiceContext, WorkspaceKeyContext},
    jwt::UserContext,
};

#[utoipa::path(
    get,
    path = "/v1/agents/{agent_id}/evaluation-profile",
    tag = "evaluations",
    params(("agent_id" = String, Path, description = "Registered agent id")),
    responses(
        (status = 200, description = "Agent evaluation profile", body = tl_core::AgentEvaluationProfile),
        (status = 404, description = "Profile not configured", body = tl_core::ApiError),
    ),
)]
pub async fn get_agent_evaluation_profile(
    State(state): State<EvaluationState>,
    headers: HeaderMap,
    Path(agent_id): Path<String>,
) -> Response {
    let (workspace_id, environment_id) = match context(&state, &headers).await {
        Ok(context) => context,
        Err(response) => return response,
    };
    match state
        .store
        .get_profile(&workspace_id, &environment_id, &agent_id)
        .await
    {
        Ok(Some(profile)) => Json(profile).into_response(),
        Ok(None) => evaluation_error_response(super::EvaluationStoreError::NotFound),
        Err(error) => evaluation_error_response(error),
    }
}

#[utoipa::path(
    put,
    path = "/v1/agents/{agent_id}/evaluation-profile",
    tag = "evaluations",
    params(("agent_id" = String, Path, description = "Registered agent id")),
    request_body = PutAgentEvaluationProfileRequest,
    responses(
        (status = 200, description = "Evaluation profile replaced", body = tl_core::AgentEvaluationProfile),
        (status = 409, description = "Profile version conflict", body = tl_core::ApiError),
    ),
)]
pub async fn put_agent_evaluation_profile(
    State(state): State<EvaluationState>,
    user: Option<Extension<UserContext>>,
    internal: Option<Extension<InternalServiceContext>>,
    runtime_key: Option<Extension<WorkspaceKeyContext>>,
    headers: HeaderMap,
    Path(agent_id): Path<String>,
    Json(input): Json<PutAgentEvaluationProfileRequest>,
) -> Response {
    let (workspace_id, environment_id) = match admin_context(
        &state,
        &headers,
        user,
        internal,
        runtime_key,
        "modify agent evaluation profiles",
    )
    .await
    {
        Ok(context) => context,
        Err(response) => return response,
    };
    match state
        .store
        .put_profile(&workspace_id, &environment_id, &agent_id, input)
        .await
    {
        Ok(profile) => Json(profile).into_response(),
        Err(error) => evaluation_error_response(error),
    }
}

#[utoipa::path(
    get,
    path = "/v1/agents/{agent_id}/evaluation-policy-assignments",
    tag = "evaluations",
    params(("agent_id" = String, Path, description = "Registered agent id")),
    responses(
        (status = 200, description = "Current evaluation policy assignments", body = AgentEvaluationPolicyAssignmentListResponse),
    ),
)]
pub async fn list_agent_evaluation_assignments(
    State(state): State<EvaluationState>,
    headers: HeaderMap,
    Path(agent_id): Path<String>,
) -> Response {
    let (workspace_id, environment_id) = match context(&state, &headers).await {
        Ok(context) => context,
        Err(response) => return response,
    };
    match state
        .store
        .list_assignments(&workspace_id, &environment_id, &agent_id)
        .await
    {
        Ok(assignments) => Json(AgentEvaluationPolicyAssignmentListResponse {
            agent_id,
            environment_id,
            assignments,
        })
        .into_response(),
        Err(error) => evaluation_error_response(error),
    }
}

#[utoipa::path(
    put,
    path = "/v1/agents/{agent_id}/evaluation-policy-assignments",
    tag = "evaluations",
    params(("agent_id" = String, Path, description = "Registered agent id")),
    request_body = PutAgentEvaluationPolicyAssignmentsRequest,
    responses(
        (status = 200, description = "Evaluation policy assignments replaced", body = AgentEvaluationPolicyAssignmentListResponse),
    ),
)]
pub async fn put_agent_evaluation_assignments(
    State(state): State<EvaluationState>,
    user: Option<Extension<UserContext>>,
    internal: Option<Extension<InternalServiceContext>>,
    runtime_key: Option<Extension<WorkspaceKeyContext>>,
    headers: HeaderMap,
    Path(agent_id): Path<String>,
    Json(input): Json<PutAgentEvaluationPolicyAssignmentsRequest>,
) -> Response {
    let (workspace_id, environment_id) = match admin_context(
        &state,
        &headers,
        user,
        internal,
        runtime_key,
        "modify agent evaluation policy assignments",
    )
    .await
    {
        Ok(context) => context,
        Err(response) => return response,
    };
    match state
        .store
        .replace_assignments(&workspace_id, &environment_id, &agent_id, input.assignments)
        .await
    {
        Ok(assignments) => Json(AgentEvaluationPolicyAssignmentListResponse {
            agent_id,
            environment_id,
            assignments,
        })
        .into_response(),
        Err(error) => evaluation_error_response(error),
    }
}

#[utoipa::path(
    get,
    path = "/v1/runs/{run_id}/evaluations",
    tag = "evaluations",
    params(("run_id" = String, Path, description = "Run id")),
    responses(
        (status = 200, description = "Participant evaluation results and findings", body = EvaluationResultListResponse),
    ),
)]
pub async fn list_run_evaluations(
    State(state): State<EvaluationState>,
    headers: HeaderMap,
    Path(run_id): Path<String>,
) -> Response {
    let (workspace_id, environment_id) = match context(&state, &headers).await {
        Ok(context) => context,
        Err(response) => return response,
    };
    match state
        .store
        .list_results(&workspace_id, &environment_id, &run_id)
        .await
    {
        Ok(results) => match state
            .store
            .list_jobs(&workspace_id, &environment_id, &run_id)
            .await
        {
            Ok(jobs) => Json(EvaluationResultListResponse { jobs, results }).into_response(),
            Err(error) => evaluation_error_response(error),
        },
        Err(error) => evaluation_error_response(error),
    }
}

#[utoipa::path(
    post,
    path = "/v1/runs/{run_id}/evaluations",
    tag = "evaluations",
    params(("run_id" = String, Path, description = "Terminal Run id")),
    request_body = ReevaluateRunRequest,
    responses(
        (status = 202, description = "New capture snapshot requested", body = ReevaluateRunResponse),
        (status = 409, description = "Run is not eligible for re-evaluation", body = tl_core::ApiError),
    ),
)]
pub async fn reevaluate_run(
    State(state): State<EvaluationState>,
    user: Option<Extension<UserContext>>,
    internal: Option<Extension<InternalServiceContext>>,
    runtime_key: Option<Extension<WorkspaceKeyContext>>,
    headers: HeaderMap,
    Path(run_id): Path<String>,
    Json(input): Json<ReevaluateRunRequest>,
) -> Response {
    let (workspace_id, environment_id) = match admin_context(
        &state,
        &headers,
        user,
        internal,
        runtime_key,
        "re-evaluate runs",
    )
    .await
    {
        Ok(context) => context,
        Err(response) => return response,
    };
    match state
        .store
        .request_reevaluation(&workspace_id, &environment_id, &run_id, input.agent_ids)
        .await
    {
        Ok(()) => (
            axum::http::StatusCode::ACCEPTED,
            Json(ReevaluateRunResponse {
                run_id,
                status: EvaluationJobStatus::WaitingCapture,
            }),
        )
            .into_response(),
        Err(error) => evaluation_error_response(error),
    }
}

async fn context(
    state: &EvaluationState,
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

async fn admin_context(
    state: &EvaluationState,
    headers: &HeaderMap,
    user: Option<Extension<UserContext>>,
    internal: Option<Extension<InternalServiceContext>>,
    runtime_key: Option<Extension<WorkspaceKeyContext>>,
    action: &str,
) -> Result<(String, String), Response> {
    let (workspace_id, _) = crate::dashboard_admin::authorize_workspace_admin(
        &state.team_store,
        headers,
        user,
        internal,
        runtime_key,
        action,
    )
    .await?;
    let environment_id = crate::environments::resolve_environment_id(
        headers,
        state.environment_store.as_ref(),
        &workspace_id,
    )
    .await
    .map_err(crate::environments::environment_error_response)?;
    Ok((workspace_id, environment_id))
}
