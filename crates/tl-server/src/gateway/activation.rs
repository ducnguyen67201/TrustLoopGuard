use axum::{
    extract::{Path, State},
    http::{HeaderMap, StatusCode, Uri},
    response::{IntoResponse, Response},
    Extension, Json,
};
use serde_json::json;
use tl_core::{
    AgentAuthority, AgentEvaluationPolicyAssignment, AgentProfile, AgentScope, AgentTone, ApiError,
    ApiErrorCode, CaptureMode, ContentCaptureMode, CreateGatewayActivationRequest,
    CreateGatewayActivationResponse, CreateNotificationRuleRequest, EvaluationJobStatus,
    EvaluationVerdict, GatewayActivationAgentInput, GatewayProductionReadiness,
    MissingEvidenceBehavior, NotificationEventKind, ProductionReadinessCheck,
    ProductionReadinessStatus, PutAgentEvaluationProfileRequest, RunCaptureStatus, RunKind,
    UpdateWorkspaceSettingsRequest,
};

use crate::runs::RunListFilter;

use super::api::{reject_runtime_key_config_access, GatewayState};
use super::errors::api_error_response;
use super::normalization::{normalize_gateway_route, normalize_provider_connection};
use super::store::ProviderConnectionPatch;

const STARTER_DENIED_ID: &str = "featherlane-starter-no-denied-decisions";
const STARTER_PROVIDER_ID: &str = "featherlane-starter-provider-reliability";
const STARTER_DENIED_YAML: &str = r#"family: evaluation
id: featherlane-starter-no-denied-decisions
description: Completed runs must not contain denied decisions.
severity: high
scope: runtime_decisions
grader:
  kind: run_metric
  metric: denied_decisions
  comparator: lte
  value: 0
on_missing_evidence: inconclusive
"#;
const STARTER_PROVIDER_YAML: &str = r#"family: evaluation
id: featherlane-starter-provider-reliability
description: Completed runs must not end with a terminal provider failure.
severity: critical
scope: trajectory
grader:
  kind: run_metric
  metric: provider_terminal_failures
  comparator: lte
  value: 0
on_missing_evidence: fail
"#;

#[utoipa::path(post, path = "/v1/gateway/activations", tag = "gateway", request_body = CreateGatewayActivationRequest, responses((status = 201, body = CreateGatewayActivationResponse)))]
pub async fn create_gateway_activation(
    State(state): State<GatewayState>,
    runtime_key: Option<Extension<crate::auth::WorkspaceKeyContext>>,
    headers: HeaderMap,
    Json(mut input): Json<CreateGatewayActivationRequest>,
) -> Response {
    if let Some(response) = reject_runtime_key_config_access(runtime_key) {
        return response;
    }
    let workspace_id = match crate::policies::workspace_id_from_headers(&headers) {
        Ok(value) => value,
        Err(response) => return response,
    };
    let environment_id = match crate::environments::resolve_environment_id(
        &headers,
        state.app.environment_store.as_ref(),
        &workspace_id,
    )
    .await
    {
        Ok(value) => value,
        Err(error) => return crate::environments::environment_error_response(error),
    };
    let alerts_deferred = input.alerts_deferred.unwrap_or(false);
    if !alerts_deferred && !valid_email(&input.alert_email) {
        return api_error_response(
            StatusCode::BAD_REQUEST,
            "alert_email must be valid unless alerts_deferred is true".into(),
        );
    }
    if !alerts_deferred && !state.app.notification_transport_configured {
        return activation_error_response(
            StatusCode::SERVICE_UNAVAILABLE,
            "notification_transport",
            &[],
            "email transport is not configured; explicitly defer alerts to continue",
            true,
        );
    }
    if !input.confirm_workspace_privacy_change {
        return api_error_response(
            StatusCode::BAD_REQUEST,
            "confirm_workspace_privacy_change is required because data handling is workspace-wide"
                .into(),
        );
    }
    let verification_session_id = match input.verification_session_id.take() {
        Some(value) => match validate_verification_session_id(&value) {
            Ok(value) => value,
            Err(message) => return api_error_response(StatusCode::BAD_REQUEST, message),
        },
        None => uuid::Uuid::now_v7().to_string(),
    };
    let mut ready_resource_ids = Vec::new();

    let slug = stable_slug(&input.route_display_name);
    if input.provider.id.is_none() {
        input.provider.id = Some(format!("gpc-production-{slug}"));
    }
    let normalized_provider =
        match normalize_provider_connection(&workspace_id, input.provider, &state.seal_key) {
            Ok(value) => value,
            Err(message) => return api_error_response(StatusCode::BAD_REQUEST, message),
        };
    if normalized_provider.kind == tl_core::GatewayProviderKind::PaymentHttp {
        return activation_error_response(
            StatusCode::BAD_REQUEST,
            "provider_connection",
            &ready_resource_ids,
            "payment_http connections cannot be used for an LLM production loop",
            false,
        );
    }
    let available_connections = match state.store.list_provider_connections(&workspace_id).await {
        Ok(value) => value,
        Err(error) => {
            tracing::warn!(error = %error, "activation provider lookup failed");
            return activation_error_response(
                StatusCode::INTERNAL_SERVER_ERROR,
                "provider_connection",
                &ready_resource_ids,
                "provider connections could not be read",
                true,
            );
        }
    };
    for fallback_id in &input.fallback_provider_connection_ids {
        let Some(fallback) = available_connections
            .iter()
            .find(|connection| connection.id == fallback_id.trim())
        else {
            return activation_error_response(
                StatusCode::BAD_REQUEST,
                "gateway_route",
                &ready_resource_ids,
                "a selected fallback provider was not found",
                false,
            );
        };
        if fallback.id == normalized_provider.id
            || fallback.kind != normalized_provider.kind
            || fallback.kind == tl_core::GatewayProviderKind::PaymentHttp
        {
            return activation_error_response(
                StatusCode::BAD_REQUEST,
                "gateway_route",
                &ready_resource_ids,
                "fallback providers must be distinct configured connections of the same protocol",
                false,
            );
        }
    }
    let provider = match available_connections
        .into_iter()
        .find(|connection| connection.id == normalized_provider.id)
    {
        Some(existing) => {
            if existing.display_name != normalized_provider.display_name
                || existing.kind != normalized_provider.kind
                || existing.base_url != normalized_provider.base_url
                || existing.default_model != normalized_provider.default_model
            {
                return activation_error_response(
                    StatusCode::CONFLICT,
                    "provider_connection",
                    &ready_resource_ids,
                    "provider id already exists with different configuration",
                    false,
                );
            }
            match state
                .store
                .update_provider_connection(
                    &workspace_id,
                    &normalized_provider.id,
                    ProviderConnectionPatch {
                        display_name: Some(normalized_provider.display_name.clone()),
                        base_url: Some(normalized_provider.base_url.clone()),
                        default_model: Some(normalized_provider.default_model.clone()),
                        encrypted_api_key: Some(normalized_provider.encrypted_api_key.clone()),
                    },
                )
                .await
            {
                Ok(value) => value,
                Err(error) => {
                    tracing::warn!(error = %error, "activation provider credential update failed");
                    return activation_error_response(
                        StatusCode::INTERNAL_SERVER_ERROR,
                        "provider_connection",
                        &ready_resource_ids,
                        "provider connection could not be reconciled",
                        true,
                    );
                }
            }
        }
        None => match state
            .store
            .create_provider_connection(normalized_provider)
            .await
        {
            Ok(value) => value,
            Err(error) => {
                tracing::warn!(error = %error, "activation provider creation failed");
                return activation_error_response(
                    StatusCode::INTERNAL_SERVER_ERROR,
                    "provider_connection",
                    &ready_resource_ids,
                    "provider connection could not be created",
                    true,
                );
            }
        },
    };
    ready_resource_ids.push(provider.id.clone());

    let agent_id = match input.agent {
        GatewayActivationAgentInput::Existing { agent_id } => match state
            .app
            .agent_store
            .get(&workspace_id, agent_id.trim())
            .await
        {
            Ok(_) => agent_id.trim().to_string(),
            Err(crate::agents::AgentStoreError::NotFound) => {
                return activation_error_response(
                    StatusCode::NOT_FOUND,
                    "agent",
                    &ready_resource_ids,
                    "selected agent was not found",
                    false,
                )
            }
            Err(error) => {
                tracing::warn!(error = %error, "activation agent lookup failed");
                return activation_error_response(
                    StatusCode::INTERNAL_SERVER_ERROR,
                    "agent",
                    &ready_resource_ids,
                    "selected agent could not be read",
                    true,
                );
            }
        },
        GatewayActivationAgentInput::New { name, purpose } => {
            if name.trim().is_empty() || purpose.trim().is_empty() {
                return api_error_response(
                    StatusCode::BAD_REQUEST,
                    "new agents require name and purpose".into(),
                );
            }
            let agent_id = format!("agent-production-{}", stable_slug(&name));
            let profile = AgentProfile {
                agent_id: agent_id.clone(),
                display_name: name.trim().to_string(),
                scope: AgentScope {
                    in_scope: vec![purpose.trim().to_string()],
                    out_of_scope: vec![],
                },
                authority: AgentAuthority::default(),
                tone: AgentTone {
                    target: "clear and safe".into(),
                    forbidden: vec![],
                },
                knowledge_sources: vec![],
                escalation_triggers: vec!["production evaluation failure".into()],
                workflow_requirements: vec![],
                system_prompt: Some(purpose.trim().to_string()),
                workflow_definition: None,
                target_url: None,
            };
            let source = match serde_yaml::to_string(&profile) {
                Ok(value) => value,
                Err(_) => {
                    return api_error_response(
                        StatusCode::INTERNAL_SERVER_ERROR,
                        "agent profile could not be serialized".into(),
                    )
                }
            };
            match state.app.agent_store.get(&workspace_id, &agent_id).await {
                Ok(existing) => {
                    let existing = serde_json::to_value(existing.as_ref()).ok();
                    let intended = serde_json::to_value(&profile).ok();
                    if existing != intended {
                        return activation_error_response(
                            StatusCode::CONFLICT,
                            "agent",
                            &ready_resource_ids,
                            "generated agent id already exists with a different profile",
                            false,
                        );
                    }
                }
                Err(crate::agents::AgentStoreError::NotFound) => {
                    if let Err(error) = state
                        .app
                        .agent_store
                        .upsert(&workspace_id, &profile, &source)
                        .await
                    {
                        tracing::warn!(error = %error, "activation agent creation failed");
                        return activation_error_response(
                            StatusCode::INTERNAL_SERVER_ERROR,
                            "agent",
                            &ready_resource_ids,
                            "agent profile could not be created",
                            true,
                        );
                    }
                }
                Err(error) => {
                    tracing::warn!(error = %error, "activation agent lookup failed");
                    return activation_error_response(
                        StatusCode::INTERNAL_SERVER_ERROR,
                        "agent",
                        &ready_resource_ids,
                        "agent profile could not be read",
                        true,
                    );
                }
            }
            agent_id
        }
    };
    ready_resource_ids.push(agent_id.clone());

    let route_id = format!("gr-production-{slug}");
    let route_input = match normalize_gateway_route(
        &workspace_id,
        tl_core::CreateGatewayRouteRequest {
            id: Some(route_id.clone()),
            display_name: input.route_display_name,
            provider_connection_id: provider.id.clone(),
            agent_id: agent_id.clone(),
            reliability_mode: input.reliability_mode,
            fallback_provider_connection_ids: input.fallback_provider_connection_ids,
        },
    ) {
        Ok(value) => value,
        Err(message) => return api_error_response(StatusCode::BAD_REQUEST, message),
    };
    let route = match state.store.list_gateway_routes(&workspace_id).await {
        Ok(routes) => match routes.into_iter().find(|route| route.id == route_id) {
            Some(existing) => {
                if existing.display_name != route_input.display_name
                    || existing.provider_connection_id != route_input.provider_connection_id
                    || existing.agent_id != route_input.agent_id
                    || existing.reliability_mode != route_input.reliability_mode
                    || existing.fallback_provider_connection_ids
                        != route_input.fallback_provider_connection_ids
                {
                    return activation_error_response(
                        StatusCode::CONFLICT,
                        "gateway_route",
                        &ready_resource_ids,
                        "gateway route id already exists with different configuration",
                        false,
                    );
                }
                existing
            }
            None => match state.store.create_gateway_route(route_input).await {
                Ok(value) => value,
                Err(error) => {
                    tracing::warn!(error = %error, "activation route creation failed");
                    return activation_error_response(
                        StatusCode::INTERNAL_SERVER_ERROR,
                        "gateway_route",
                        &ready_resource_ids,
                        "gateway route could not be created",
                        true,
                    );
                }
            },
        },
        Err(error) => {
            tracing::warn!(error = %error, "activation route lookup failed");
            return activation_error_response(
                StatusCode::INTERNAL_SERVER_ERROR,
                "gateway_route",
                &ready_resource_ids,
                "gateway routes could not be read",
                true,
            );
        }
    };
    ready_resource_ids.push(route.id.clone());

    if let Err(failure) = ensure_starter_policies(&state, &workspace_id, &environment_id).await {
        return activation_error_response(
            failure.status,
            "starter_evaluations",
            &ready_resource_ids,
            &failure.message,
            failure.retriable,
        );
    }
    let existing_profile = state
        .app
        .evaluation_store
        .get_profile(&workspace_id, &environment_id, &agent_id)
        .await
        .ok()
        .flatten();
    let evaluation_profile = if let Some(existing) = existing_profile {
        if !existing.enabled
            || existing.capture_mode != CaptureMode::Durable
            || existing.content_mode != ContentCaptureMode::MetadataOnly
            || existing.quiet_period_ms != 2_000
            || existing.max_capture_wait_ms != 30_000
            || existing.on_incomplete != MissingEvidenceBehavior::Inconclusive
        {
            return activation_error_response(
                StatusCode::CONFLICT,
                "evaluation_profile",
                &ready_resource_ids,
                "agent already has a different evaluation profile",
                false,
            );
        }
        existing
    } else {
        match state
            .app
            .evaluation_store
            .put_profile(
                &workspace_id,
                &environment_id,
                &agent_id,
                PutAgentEvaluationProfileRequest {
                    enabled: true,
                    capture_mode: CaptureMode::Durable,
                    content_mode: ContentCaptureMode::MetadataOnly,
                    quiet_period_ms: 2_000,
                    max_capture_wait_ms: 30_000,
                    on_incomplete: MissingEvidenceBehavior::Inconclusive,
                    expected_profile_version: None,
                },
            )
            .await
        {
            Ok(value) => value,
            Err(error) => {
                tracing::warn!(error = %error, "activation evaluation profile creation failed");
                return activation_error_response(
                    StatusCode::INTERNAL_SERVER_ERROR,
                    "evaluation_profile",
                    &ready_resource_ids,
                    "evaluation profile could not be created",
                    true,
                );
            }
        }
    };
    ready_resource_ids.push(format!("evaluation-profile:{agent_id}"));
    for (policy_id, critical) in [(STARTER_DENIED_ID, true), (STARTER_PROVIDER_ID, true)] {
        if let Err(error) = state
            .app
            .evaluation_store
            .ensure_assignment(
                &workspace_id,
                &environment_id,
                &agent_id,
                AgentEvaluationPolicyAssignment {
                    policy_id: policy_id.into(),
                    policy_version: None,
                    weight: 1,
                    critical,
                    enabled: true,
                },
            )
            .await
        {
            tracing::warn!(policy_id, error = %error, "activation evaluation assignment failed");
            return activation_error_response(
                StatusCode::INTERNAL_SERVER_ERROR,
                "evaluation_assignments",
                &ready_resource_ids,
                "evaluation assignment could not be reconciled",
                true,
            );
        }
    }

    let notification_rule = if alerts_deferred {
        None
    } else {
        match state
            .app
            .notification_store
            .list_rules(&workspace_id, &environment_id)
            .await
        {
            Ok(rules) => {
                if let Some(rule) = rules.into_iter().find(|rule| {
                    rule.enabled
                        && rule.agent_id.as_deref() == Some(agent_id.as_str())
                        && rule.email.eq_ignore_ascii_case(input.alert_email.trim())
                        && rule
                            .event_kinds
                            .contains(&NotificationEventKind::EvaluationFailed)
                        && rule
                            .event_kinds
                            .contains(&NotificationEventKind::EvaluationInconclusive)
                        && rule
                            .event_kinds
                            .contains(&NotificationEventKind::EvaluationError)
                        && rule
                            .event_kinds
                            .contains(&NotificationEventKind::ProviderTerminalFailure)
                }) {
                    Some(rule)
                } else {
                    match state
                        .app
                        .notification_store
                        .create_rule(
                            &workspace_id,
                            &environment_id,
                            Some(agent_id.clone()),
                            CreateNotificationRuleRequest {
                                email: input.alert_email.trim().to_string(),
                                event_kinds: vec![
                                    NotificationEventKind::EvaluationFailed,
                                    NotificationEventKind::EvaluationInconclusive,
                                    NotificationEventKind::EvaluationError,
                                    NotificationEventKind::ProviderTerminalFailure,
                                ],
                                enabled: true,
                            },
                        )
                        .await
                    {
                        Ok(value) => Some(value),
                        Err(error) => {
                            tracing::warn!(error = %error, "activation notification creation failed");
                            return activation_error_response(
                                StatusCode::INTERNAL_SERVER_ERROR,
                                "notification_rule",
                                &ready_resource_ids,
                                "email alert rule could not be created",
                                true,
                            );
                        }
                    }
                }
            }
            Err(error) => {
                tracing::warn!(error = %error, "activation notification lookup failed");
                return activation_error_response(
                    StatusCode::INTERNAL_SERVER_ERROR,
                    "notification_rule",
                    &ready_resource_ids,
                    "email alert rules could not be read",
                    true,
                );
            }
        }
    };
    if let Some(rule) = &notification_rule {
        ready_resource_ids.push(rule.id.clone());
    }
    let settings = match state
        .app
        .settings_store
        .update(
            &workspace_id,
            UpdateWorkspaceSettingsRequest {
                data_handling_mode: Some(input.data_handling_mode),
                ..Default::default()
            },
        )
        .await
    {
        Ok(value) => value,
        Err(error) => {
            tracing::warn!(error = %error, "activation privacy update failed");
            return activation_error_response(
                StatusCode::INTERNAL_SERVER_ERROR,
                "workspace_privacy",
                &ready_resource_ids,
                "workspace privacy setting could not be updated",
                true,
            );
        }
    };
    let readiness = build_readiness(
        &state,
        &workspace_id,
        &environment_id,
        &route.id,
        &agent_id,
        Some(&verification_session_id),
    )
    .await;
    (
        StatusCode::CREATED,
        Json(CreateGatewayActivationResponse {
            route,
            agent_id,
            evaluation_profile,
            notification_rule,
            alerts_deferred,
            verification_session_id,
            data_handling_mode: settings.data_handling_mode,
            readiness,
        }),
    )
        .into_response()
}

#[utoipa::path(get, path = "/v1/gateway/routes/{id}/production-readiness", tag = "gateway", params(("id" = String, Path), ("external_id" = Option<String>, Query)), responses((status = 200, body = GatewayProductionReadiness)))]
pub async fn gateway_production_readiness(
    State(state): State<GatewayState>,
    runtime_key: Option<Extension<crate::auth::WorkspaceKeyContext>>,
    headers: HeaderMap,
    Path(id): Path<String>,
    uri: Uri,
) -> Response {
    if let Some(response) = reject_runtime_key_config_access(runtime_key) {
        return response;
    }
    let workspace_id = match crate::policies::workspace_id_from_headers(&headers) {
        Ok(value) => value,
        Err(response) => return response,
    };
    let environment_id = match crate::environments::resolve_environment_id(
        &headers,
        state.app.environment_store.as_ref(),
        &workspace_id,
    )
    .await
    {
        Ok(value) => value,
        Err(error) => return crate::environments::environment_error_response(error),
    };
    let route = match state
        .store
        .list_gateway_routes(&workspace_id)
        .await
        .ok()
        .and_then(|routes| routes.into_iter().find(|route| route.id == id))
    {
        Some(value) => value,
        None => return api_error_response(StatusCode::NOT_FOUND, "gateway route not found".into()),
    };
    let requested_external_id = uri.query().and_then(|query| {
        url::form_urlencoded::parse(query.as_bytes())
            .find_map(|(key, value)| (key == "external_id").then(|| value.into_owned()))
    });
    let external_id = match requested_external_id.as_deref() {
        Some(value) => match validate_verification_session_id(value) {
            Ok(value) => Some(value),
            Err(message) => return api_error_response(StatusCode::BAD_REQUEST, message),
        },
        None => None,
    };
    Json(
        build_readiness(
            &state,
            &workspace_id,
            &environment_id,
            &id,
            &route.agent_id,
            external_id.as_deref(),
        )
        .await,
    )
    .into_response()
}

struct ActivationFailure {
    status: StatusCode,
    message: String,
    retriable: bool,
}

async fn ensure_starter_policies(
    state: &GatewayState,
    workspace_id: &str,
    environment_id: &str,
) -> Result<(), ActivationFailure> {
    let existing = state
        .app
        .policy_store
        .list(workspace_id, environment_id)
        .await
        .map_err(|error| {
            tracing::warn!(error = %error, "starter policy list failed");
            ActivationFailure {
                status: StatusCode::INTERNAL_SERVER_ERROR,
                message: "starter policies could not be read".into(),
                retriable: true,
            }
        })?;
    for (id, yaml) in [
        (STARTER_DENIED_ID, STARTER_DENIED_YAML),
        (STARTER_PROVIDER_ID, STARTER_PROVIDER_YAML),
    ] {
        if existing
            .iter()
            .any(|policy| policy.id == id && policy.family != tl_core::PolicyFamily::Evaluation)
        {
            return Err(ActivationFailure {
                status: StatusCode::CONFLICT,
                message: format!("starter policy id {id} is already used by another family"),
                retriable: false,
            });
        }
        if existing.iter().any(|policy| policy.id == id) {
            let document = state
                .app
                .policy_store
                .get(workspace_id, environment_id, id)
                .await
                .map_err(|error| {
                    tracing::warn!(policy_id = id, error = %error, "starter policy lookup failed");
                    ActivationFailure {
                        status: StatusCode::INTERNAL_SERVER_ERROR,
                        message: "starter policy could not be read".into(),
                        retriable: true,
                    }
                })?;
            let existing_value = serde_yaml::from_str::<serde_yaml::Value>(&document.source_yaml);
            let intended_value = serde_yaml::from_str::<serde_yaml::Value>(yaml);
            if existing_value.ok() != intended_value.ok() {
                return Err(ActivationFailure {
                    status: StatusCode::CONFLICT,
                    message: format!(
                        "starter policy id {id} already has different evaluation semantics"
                    ),
                    retriable: false,
                });
            }
            continue;
        }
        let policy = tl_policy::load_any_str(yaml).map_err(|error| ActivationFailure {
            status: StatusCode::INTERNAL_SERVER_ERROR,
            message: format!("starter policy parse failed: {error}"),
            retriable: false,
        })?;
        let tl_policy::AnyPolicy::Family(policy) = policy else {
            return Err(ActivationFailure {
                status: StatusCode::INTERNAL_SERVER_ERROR,
                message: "starter evaluation policy has invalid family".into(),
                retriable: false,
            });
        };
        state
            .app
            .policy_store
            .upsert_family(workspace_id, environment_id, &policy, yaml)
            .await
            .map_err(|error| {
                tracing::warn!(policy_id = id, error = %error, "starter policy creation failed");
                ActivationFailure {
                    status: StatusCode::INTERNAL_SERVER_ERROR,
                    message: "starter evaluation policy could not be created".into(),
                    retriable: true,
                }
            })?;
    }
    Ok(())
}

async fn build_readiness(
    state: &GatewayState,
    workspace_id: &str,
    environment_id: &str,
    route_id: &str,
    agent_id: &str,
    external_id: Option<&str>,
) -> GatewayProductionReadiness {
    let route_ready = state
        .store
        .resolve_gateway_route(workspace_id, route_id)
        .await
        .is_ok();
    let profile_ready = state
        .app
        .evaluation_store
        .get_profile(workspace_id, environment_id, agent_id)
        .await
        .ok()
        .flatten()
        .is_some_and(|profile| profile.enabled);
    let assignments_ready = state
        .app
        .evaluation_store
        .list_assignments(workspace_id, environment_id, agent_id)
        .await
        .is_ok_and(|assignments| {
            [STARTER_DENIED_ID, STARTER_PROVIDER_ID].iter().all(|id| {
                assignments
                    .iter()
                    .any(|assignment| assignment.enabled && assignment.policy_id == *id)
            })
        });
    let rule_ready = state
        .app
        .notification_store
        .list_rules(workspace_id, environment_id)
        .await
        .is_ok_and(|rules| {
            rules
                .iter()
                .any(|rule| rule.enabled && rule.agent_id.as_deref() == Some(agent_id))
        });
    let runtime_key_ready = state
        .app
        .api_key_store
        .list(workspace_id)
        .await
        .is_ok_and(|keys| {
            keys.iter()
                .any(|key| key.status == "active" && key.environment_id == environment_id)
        });
    let verification_run = if let Some(external_id) = external_id {
        state
            .app
            .run_store
            .list(
                workspace_id,
                environment_id,
                RunListFilter {
                    agent_id: Some(agent_id.to_string()),
                    kind: Some(RunKind::ChatSession),
                    external_id: Some(external_id.to_string()),
                    limit: 10,
                    ..RunListFilter::default()
                },
            )
            .await
            .ok()
            .and_then(|runs| {
                runs.into_iter().find(|run| {
                    run.metadata
                        .get("integration_mode")
                        .and_then(serde_json::Value::as_str)
                        == Some("gateway")
                        && run
                            .metadata
                            .get("route_id")
                            .and_then(serde_json::Value::as_str)
                            == Some(route_id)
                })
            })
    } else {
        None
    };
    let traffic_seen = verification_run.is_some();
    let run_terminal = verification_run
        .as_ref()
        .is_some_and(|run| run.status.is_terminal());
    let finalization = if let Some(run) = &verification_run {
        state
            .app
            .run_store
            .finalization(workspace_id, environment_id, &run.id)
            .await
            .ok()
            .flatten()
    } else {
        None
    };
    let run_finalized = run_terminal
        && finalization.as_ref().is_some_and(|summary| {
            matches!(
                summary.capture_status,
                RunCaptureStatus::Complete | RunCaptureStatus::Incomplete
            )
        });
    let evaluation_complete = if let Some(run) = &verification_run {
        let manifest_ready = state
            .app
            .evaluation_store
            .list_manifest(workspace_id, &run.id, Some(agent_id))
            .await
            .is_ok_and(|manifest| !manifest.is_empty());
        let results = state
            .app
            .evaluation_store
            .list_results(workspace_id, environment_id, &run.id)
            .await
            .unwrap_or_default();
        let jobs = state
            .app
            .evaluation_store
            .list_jobs(workspace_id, environment_id, &run.id)
            .await
            .unwrap_or_default();
        let has_terminal_results = !results.is_empty()
            && results
                .iter()
                .all(|result| result.result.verdict != EvaluationVerdict::NotConfigured);
        let has_terminal_jobs = !jobs.is_empty()
            && jobs.iter().all(|job| {
                matches!(
                    job.status,
                    EvaluationJobStatus::Completed
                        | EvaluationJobStatus::Failed
                        | EvaluationJobStatus::Inconclusive
                        | EvaluationJobStatus::Error
                )
            });
        manifest_ready && run_finalized && (has_terminal_results || has_terminal_jobs)
    } else {
        false
    };
    let checks = vec![
        readiness_check("route", "Provider and route", route_ready, None),
        readiness_check(
            "runtime_key",
            "Runtime key",
            runtime_key_ready,
            Some("Create a runtime key as the final recoverable step."),
        ),
        readiness_check(
            "evaluation",
            "Deterministic evaluation",
            profile_ready && assignments_ready,
            Some("Both deterministic starter evaluations must be enabled and assigned."),
        ),
        readiness_check(
            "notification_rule",
            "Email alert rule",
            rule_ready,
            Some("Create an enabled email rule or resume activation without deferring alerts."),
        ),
        readiness_check(
            "email_transport",
            "Email transport",
            state.app.notification_transport_configured,
            Some("Configure TL_NOTIFICATION_SMTP_URL and TL_NOTIFICATION_EMAIL_FROM."),
        ),
        readiness_check(
            "traffic_seen",
            "Exact test traffic seen",
            traffic_seen,
            Some("Send the generated request with the exact verification session id."),
        ),
        readiness_check(
            "run_finalized",
            "Verification Run finalized",
            run_finalized,
            Some("Send X-Featherlane-Session-End: true or wait for the session boundary."),
        ),
        readiness_check(
            "evaluation_completed",
            "Verification evaluation completed",
            evaluation_complete,
            Some("Wait for capture and deterministic evaluation; not_configured is not ready."),
        ),
    ];
    GatewayProductionReadiness {
        status: if checks.iter().all(|check| check.ready) {
            ProductionReadinessStatus::Ready
        } else {
            ProductionReadinessStatus::NeedsAttention
        },
        checks,
    }
}

fn readiness_check(
    id: &str,
    label: &str,
    ready: bool,
    detail: Option<&str>,
) -> ProductionReadinessCheck {
    ProductionReadinessCheck {
        id: id.into(),
        label: label.into(),
        ready,
        detail: (!ready).then(|| detail.map(str::to_string)).flatten(),
    }
}

fn activation_error_response(
    status: StatusCode,
    activation_step: &str,
    ready_resource_ids: &[String],
    message: &str,
    retriable: bool,
) -> Response {
    crate::log_api_error(status, ApiErrorCode::Invalid, message);
    let code = match status {
        StatusCode::NOT_FOUND => ApiErrorCode::NotFound,
        StatusCode::CONFLICT => ApiErrorCode::Conflict,
        StatusCode::UNAUTHORIZED => ApiErrorCode::Unauthorized,
        StatusCode::FORBIDDEN => ApiErrorCode::Forbidden,
        value if value.is_server_error() => ApiErrorCode::Internal,
        _ => ApiErrorCode::Invalid,
    };
    (
        status,
        Json(ApiError {
            code,
            message: message.to_string(),
            retriable,
            details: json!({
                "activation_step": activation_step,
                "ready_resource_ids": ready_resource_ids,
                "retriable": retriable,
            }),
        }),
    )
        .into_response()
}

fn validate_verification_session_id(value: &str) -> Result<String, String> {
    let value = value.trim();
    if value.is_empty() {
        return Err("verification_session_id cannot be empty".into());
    }
    if value.len() > 200 {
        return Err("verification_session_id cannot exceed 200 bytes".into());
    }
    if value.chars().any(char::is_control) {
        return Err("verification_session_id cannot contain control characters".into());
    }
    Ok(value.to_string())
}

fn valid_email(value: &str) -> bool {
    let value = value.trim();
    value.len() <= 320
        && value
            .split_once('@')
            .is_some_and(|(local, domain)| !local.is_empty() && domain.contains('.'))
}
fn stable_slug(value: &str) -> String {
    let slug = value
        .chars()
        .flat_map(char::to_lowercase)
        .map(|character| {
            if character.is_ascii_alphanumeric() {
                character
            } else {
                '-'
            }
        })
        .collect::<String>();
    let slug = slug
        .split('-')
        .filter(|part| !part.is_empty())
        .collect::<Vec<_>>()
        .join("-");
    if slug.is_empty() {
        "production-loop".into()
    } else {
        slug.chars().take(48).collect()
    }
}
