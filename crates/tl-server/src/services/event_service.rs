//! Direct `GuardEvent` ingestion.
//!
//! Runtime `GuardEvent` submission: workspace gates, validation, the event
//! pipeline, enabled policy evaluation, and the fire-and-forget trace write.
//! No legacy tier engine runs here. The decision seeds as an allow; enforce-
//! mode checkers and enabled policies can upgrade it.

use axum::{http::StatusCode, response::Response};
use tl_core::{
    ActionGrantScope, ApiErrorCode, ApprovalRule, AuthorityRequirement, AuthorizationCapabilityId,
    AuthorizationClaim, AuthorizationDecision, AuthorizationEffect, AuthorizationFinding,
    AuthorizationGrantScope, AuthorizationSubject, Channel, CreateRunEventRequest,
    DataHandlingMode, Decision, EnforcementMode, GuardEvent, LlmUsageKind, RunEventKind,
    RunGuardrailUsage, Severity, ShellActionParameters, SideEffectClass, ToolResolution, USD,
};
use tl_engine::{evaluate_event_policies, EventPolicyEvalCtx};

use crate::{app::error::api_error_response, AppState};

/// Seed reason for `/v1/events` decisions. It survives only when no
/// enforce-mode checker fires and no enabled policy matches.
pub(crate) const DEFAULT_EVENT_ALLOW_REASON: &str =
    "event allowed: no enforced checker or enabled policy matched";

/// Default trace domain for SDK/direct events. Gateway phases use
/// trusted server-authored domains so run detail can classify input and
/// output checks without trusting arbitrary client context.
const EVENT_TRACE_DOMAIN: &str = "event";

const MAX_SOURCES: usize = 64;
const MAX_PROVENANCE_PATHS: usize = 128;
const MAX_SOURCES_PER_PATH: usize = 32;
/// Event ids are opaque, but they land in indexed columns and must stay small.
pub(super) const MAX_ID_BYTES: usize = 256;
const MAX_PATH_BYTES: usize = 512;
const MAX_PARAMETERS_BYTES: usize = 65_536;
const MAX_CONTEXT_BYTES: usize = 65_536;

pub(crate) struct EventSubmissionResult {
    pub decision: Decision,
    pub authorization: AuthorizationDecision,
}

#[derive(Debug, Clone, Default)]
pub(crate) struct EventSubmissionContext {
    pub authorization_principal_id: Option<String>,
    pub additional_findings: Vec<AuthorizationFinding>,
}

pub(crate) async fn execute_event_submission(
    state: &AppState,
    workspace_id: &str,
    environment_id: &str,
    event: GuardEvent,
    start: std::time::Instant,
) -> Result<EventSubmissionResult, Response> {
    execute_event_submission_inner(
        state,
        workspace_id,
        environment_id,
        event,
        start,
        EventSubmissionContext::default(),
    )
    .await
}

pub(crate) async fn execute_event_submission_as_principal(
    state: &AppState,
    workspace_id: &str,
    environment_id: &str,
    event: GuardEvent,
    start: std::time::Instant,
    authorization_principal_id: &str,
) -> Result<EventSubmissionResult, Response> {
    execute_event_submission_inner(
        state,
        workspace_id,
        environment_id,
        event,
        start,
        EventSubmissionContext {
            authorization_principal_id: Some(authorization_principal_id.to_string()),
            additional_findings: Vec::new(),
        },
    )
    .await
}

pub(crate) async fn execute_event_submission_with_context(
    state: &AppState,
    workspace_id: &str,
    environment_id: &str,
    event: GuardEvent,
    start: std::time::Instant,
    context: EventSubmissionContext,
) -> Result<EventSubmissionResult, Response> {
    execute_event_submission_inner(state, workspace_id, environment_id, event, start, context).await
}

async fn execute_event_submission_inner(
    state: &AppState,
    workspace_id: &str,
    environment_id: &str,
    mut event: GuardEvent,
    start: std::time::Instant,
    context: EventSubmissionContext,
) -> Result<EventSubmissionResult, Response> {
    // Authorization is a replay credential, not trace evidence. Extract it
    // before validation/pipeline work and never persist it with the event.
    let authorization = event.action.authorization.take();
    if event.kind == tl_core::EventKind::ShellActionProposed {
        event.action.side_effect = Some(SideEffectClass::ShellExec);
    }
    // Validate before any storage round trip so malformed-but-
    // authenticated spam never touches the database.
    if let Err(msg) = validate_event(&event) {
        return Err(api_error_response(
            StatusCode::UNPROCESSABLE_ENTITY,
            ApiErrorCode::Unprocessable,
            msg,
        ));
    }

    let workspace_settings = match state.settings_store.get(workspace_id).await {
        Ok(settings) => settings,
        Err(e) => {
            // Log details server-side; storage internals never reach
            // API responses.
            tracing::error!(workspace_id, error = %e, "workspace settings resolution failed");
            return Err(api_error_response(
                StatusCode::INTERNAL_SERVER_ERROR,
                ApiErrorCode::Internal,
                "workspace settings resolution failed".into(),
            ));
        }
    };
    // Event redaction does not exist yet; never silently persist raw
    // payloads for a workspace that asked for redaction guarantees.
    if workspace_settings.data_handling_mode != DataHandlingMode::RawAllowed {
        return Err(api_error_response(
            StatusCode::BAD_REQUEST,
            ApiErrorCode::Invalid,
            "workspace data handling mode requires redaction; event ingestion supports \
             raw_allowed workspaces only"
                .into(),
        ));
    }

    if let Some(run_id) = event.principal.run_id.as_deref() {
        if uuid::Uuid::parse_str(run_id).is_err() {
            return Err(api_error_response(
                StatusCode::BAD_REQUEST,
                ApiErrorCode::Invalid,
                "run_id must be a UUID".into(),
            ));
        }
        match state
            .run_store
            .get(workspace_id, environment_id, run_id)
            .await
        {
            Ok(_) => {}
            Err(crate::runs::RunStoreError::NotFound) => {
                return Err(api_error_response(
                    StatusCode::NOT_FOUND,
                    ApiErrorCode::NotFound,
                    "run_id was not found in the resolved environment".into(),
                ));
            }
            Err(e) => {
                tracing::error!(workspace_id, error = %e, "run resolution failed");
                return Err(api_error_response(
                    StatusCode::INTERNAL_SERVER_ERROR,
                    ApiErrorCode::Internal,
                    "run resolution failed".into(),
                ));
            }
        }
    }
    if let Some(run_event_id) = event.principal.run_event_id.as_deref() {
        if uuid::Uuid::parse_str(run_event_id).is_err() {
            return Err(api_error_response(
                StatusCode::BAD_REQUEST,
                ApiErrorCode::Invalid,
                "run_event_id must be a UUID".into(),
            ));
        }
    }
    if event.principal.run_event_id.is_some() && event.principal.run_id.is_none() {
        return Err(api_error_response(
            StatusCode::BAD_REQUEST,
            ApiErrorCode::Invalid,
            "run_id is required when run_event_id is provided".into(),
        ));
    }
    if let (Some(run_id), Some(run_event_id)) = (
        event.principal.run_id.as_deref(),
        event.principal.run_event_id.as_deref(),
    ) {
        match state
            .run_store
            .event_belongs_to_run(workspace_id, environment_id, run_id, run_event_id)
            .await
        {
            Ok(()) => {}
            Err(crate::runs::RunStoreError::NotFound) => {
                return Err(api_error_response(
                    StatusCode::BAD_REQUEST,
                    ApiErrorCode::Invalid,
                    "run_event_id does not belong to run_id".into(),
                ));
            }
            Err(e) => {
                tracing::error!(workspace_id, error = %e, "run event ownership check failed");
                return Err(api_error_response(
                    StatusCode::INTERNAL_SERVER_ERROR,
                    ApiErrorCode::Internal,
                    "run event ownership check failed".into(),
                ));
            }
        }
    }

    if let Some(claim) = authorization.as_ref() {
        validate_authorization(claim)?;
    }

    // No tier engine: events are evaluated by the event pipeline and
    // enabled policies.
    let mut decision = Decision::allow(tl_core::new_trace_id());
    decision.reason = DEFAULT_EVENT_ALLOW_REASON.into();

    let modes =
        super::resolve_checker_modes(state, workspace_id, environment_id, &workspace_settings)
            .await?;
    let pipeline_start = std::time::Instant::now();
    let (mut event, mut decision) = state
        .event_pipeline
        .process(event, workspace_id, environment_id, modes, decision)
        .await;
    // A collector or registry entry cannot downgrade executable shell syntax.
    if event.kind == tl_core::EventKind::ShellActionProposed {
        event.action.side_effect = Some(SideEffectClass::ShellExec);
    }
    let pipeline_latency_us = pipeline_start.elapsed().as_micros() as u64;

    let enabled_policies = match state
        .policy_store
        .list_enabled(workspace_id, environment_id)
        .await
    {
        Ok(policies) => policies,
        Err(e) => {
            tracing::error!(workspace_id, environment_id, error = %e, "policy resolution failed");
            return Err(api_error_response(
                StatusCode::INTERNAL_SERVER_ERROR,
                ApiErrorCode::Internal,
                "runtime policy resolution failed".into(),
            ));
        }
    };
    let policy_outcome = evaluate_event_policies(
        &event,
        enabled_policies.iter().map(std::convert::AsRef::as_ref),
        EventPolicyEvalCtx {
            tenant: workspace_id,
            semantic_judge: Some(state.handler_ctx.llm.as_ref()),
        },
    )
    .await;
    let semantic_invocations = policy_outcome.semantic_invocations.clone();
    decision.triggered_policies.extend(policy_outcome.triggered);
    if let Some(policy_verdict) = policy_outcome.effect {
        if effect_rank(policy_verdict) > effect_rank(decision.effect) {
            decision.effect = policy_verdict;
            if let Some(reason) = policy_outcome.reason {
                decision.reason = reason;
            }
            decision.safe_output = match policy_verdict {
                AuthorizationEffect::Transform => policy_outcome.safe_output,
                _ => None,
            };
        }
    }

    let subject = authorization_subject(&event)?;
    let attempt_id = match &subject {
        AuthorizationSubject::Tool { invocation_id, .. } => Some(invocation_id.clone()),
        _ => None,
    };
    let requirement = authority_requirement(&event, &subject, decision.effect)?;
    let requirement_id = requirement
        .as_ref()
        .map(|requirement| requirement.id.clone());
    let mut findings = authorization_findings(&event, &decision, requirement_id.as_deref());
    findings.extend(context.additional_findings);
    let authorization_decision = state
        .authorization_coordinator
        .evaluate(crate::authorization::AuthorizationEvaluationRequest {
            workspace_id: workspace_id.to_string(),
            environment_id: environment_id.to_string(),
            principal_id: context
                .authorization_principal_id
                .as_deref()
                .unwrap_or(&event.principal.agent_id)
                .to_string(),
            subject,
            findings,
            requirements: requirement.into_iter().collect(),
            policy_versions: enabled_policies
                .iter()
                .map(|policy| policy.id.to_string())
                .collect(),
            claim: authorization,
            attempt_id,
            trace_id: decision.trace_id.clone(),
            run_id: event.principal.run_id.clone(),
            transformed_value: decision.safe_output.clone().map(serde_json::Value::String),
            intent_expires_at: None,
            persist_intent: !matches!(
                event.resolution.as_ref(),
                Some(ToolResolution::ResolutionFailed)
            ),
        })
        .await
        .map_err(authorization_error)?;

    let mut existing_policy_ids = decision
        .triggered_policies
        .iter()
        .map(|policy| policy.id.clone())
        .collect::<std::collections::BTreeSet<_>>();
    for finding in &authorization_decision.findings {
        let Some(policy_id) = finding.policy_id.as_ref() else {
            continue;
        };
        if existing_policy_ids.insert(policy_id.clone()) {
            decision.triggered_policies.push(tl_core::TriggeredPolicy {
                id: policy_id.clone(),
                severity: finding.severity,
                reason: finding.reason.clone(),
            });
        }
    }
    if let Some(winning) = authorization_decision.findings.iter().find(|finding| {
        finding.effect == authorization_decision.effect && finding.policy_id.is_some()
    }) {
        decision.remediation = winning.remediation.clone();
    }
    decision.effect = authorization_decision.effect;
    decision.reason = authorization_decision.reason.clone();
    decision.approval = authorization_decision.approval.clone();
    decision.applied_grant = authorization_decision.applied_grant.clone();
    decision.lease = authorization_decision.lease.clone();
    decision.authorization_receipt_id = authorization_decision.receipt_id.clone();
    decision.latency_ms = start.elapsed().as_millis() as u64;

    record_semantic_usage(
        state,
        workspace_id,
        environment_id,
        &event,
        &decision.trace_id,
        semantic_invocations,
    )
    .await;

    tracing::info!(
        workspace_id,
        environment_id,
        effect = ?decision.effect,
        flow_mode = ?modes.information_flow,
        memory_mode = ?modes.memory,
        param_mode = ?modes.parameter_auth,
        approval_mode = ?modes.approval,
        pipeline_latency_us,
        total_latency_ms = decision.latency_ms,
        "event submission completed"
    );

    if let Some(run_id) = event.principal.run_id.as_deref() {
        if let Err(e) = state
            .run_store
            .record_check(
                workspace_id,
                environment_id,
                run_id,
                effect_name(decision.effect),
                decision.latency_ms as i32,
            )
            .await
        {
            tracing::warn!(run_id, error = %e, "could not update run stats");
        }
    }

    let agent_id = event.principal.agent_id.clone();
    let trace_domain = trace_domain(&event);

    // One trace seam for every path (postgres batches, memory accumulates); a
    // failed write is logged but never fails the decision.
    let trace = crate::traces::TraceWriteRequest {
        decision: decision.clone(),
        run_id: event.principal.run_id.clone(),
        run_event_id: event.principal.run_event_id.clone(),
        session_id: event.principal.session_id.clone(),
        event: Some(event),
        workspace_id: workspace_id.to_string(),
        environment_id: environment_id.to_string(),
        domain: trace_domain.to_string(),
    };
    if let Err(e) = state.trace_store.record(trace).await {
        tracing::warn!(error = %e, "trace record failed; dropped");
    }

    // Enforce-mode checkers and enabled policies can require approval or defer event
    // decisions; route them to the shared escalation worker.
    if decision.effect == tl_core::AuthorizationEffect::Defer {
        if let Some(tx) = state.escalation_tx.as_ref() {
            let payload = crate::escalation::EscalationPayload {
                trace_id: decision.trace_id.clone(),
                agent_id,
                domain: trace_domain.to_string(),
                decision: decision.clone(),
            };
            if let Err(e) = tx.try_send(payload) {
                tracing::warn!(error = %e, "escalation channel full or closed; dropped");
            }
        }
    }

    Ok(EventSubmissionResult {
        decision,
        authorization: authorization_decision,
    })
}

#[allow(clippy::result_large_err)]
fn authorization_subject(event: &GuardEvent) -> Result<AuthorizationSubject, Response> {
    if event.kind == tl_core::EventKind::OutputProposed {
        let input = event
            .context
            .get("input")
            .and_then(serde_json::Value::as_str)
            .unwrap_or_default()
            .to_string();
        let output = event
            .action
            .parameters
            .get("text")
            .and_then(serde_json::Value::as_str)
            .unwrap_or_default()
            .to_string();
        return Ok(AuthorizationSubject::Content {
            event_kind: event.kind,
            channel: Channel::Chat,
            input,
            output,
        });
    }
    let invocation_id = event.action.invocation_id.clone().ok_or_else(|| {
        api_error_response(
            StatusCode::UNPROCESSABLE_ENTITY,
            ApiErrorCode::Unprocessable,
            "action.invocation_id is required for executable events".into(),
        )
    })?;
    let tool_identity = event.action.tool_identity.clone().ok_or_else(|| {
        api_error_response(
            StatusCode::UNPROCESSABLE_ENTITY,
            ApiErrorCode::Unprocessable,
            "action.tool_identity is required for executable events".into(),
        )
    })?;
    Ok(AuthorizationSubject::Tool {
        invocation_id,
        operation: event.action.operation.clone(),
        tool_identity,
        parameters: event.action.parameters.clone(),
        side_effect: event.action.side_effect.unwrap_or(SideEffectClass::None),
    })
}

#[allow(clippy::result_large_err)]
fn authority_requirement(
    event: &GuardEvent,
    subject: &AuthorizationSubject,
    effect: AuthorizationEffect,
) -> Result<Option<AuthorityRequirement>, Response> {
    if effect != AuthorizationEffect::RequireApproval {
        return Ok(None);
    }
    let AuthorizationSubject::Tool {
        operation,
        tool_identity,
        parameters,
        side_effect,
        ..
    } = subject
    else {
        return Ok(None);
    };
    let capability = AuthorizationCapabilityId::parse(format!(
        "tool:{}/{}",
        tool_identity.server_id.to_ascii_lowercase(),
        tool_identity.tool_name.to_ascii_lowercase()
    ))
    .map_err(|message| {
        api_error_response(
            StatusCode::UNPROCESSABLE_ENTITY,
            ApiErrorCode::Unprocessable,
            message.into(),
        )
    })?;
    let rule = controlling_approval_rule(event);
    Ok(Some(AuthorityRequirement {
        id: format!("approval:{operation}"),
        capability,
        required_scope: AuthorizationGrantScope::Action(ActionGrantScope {
            operations: vec![operation.clone()],
            side_effects: vec![*side_effect],
            server_id: Some(tool_identity.server_id.clone()),
            tool_name: Some(tool_identity.tool_name.clone()),
            schema_hash: Some(tool_identity.schema_hash.clone()),
            parameters: Some(parameters.clone()),
            allowed_destinations: Vec::new(),
            maximum_data_confidentiality: None,
            minimum_source_trust: None,
        }),
        approver_roles: rule
            .map(|rule| rule.approver_roles.clone())
            .unwrap_or_else(|| vec!["owner".into(), "admin".into()]),
        reason: rule
            .and_then(|rule| rule.reason.clone())
            .unwrap_or_else(|| "current policy requires human authorization".into()),
        reusable_allowed: true,
        max_grant_ttl_seconds: Some(86_400),
    }))
}

fn authorization_findings(
    event: &GuardEvent,
    decision: &Decision,
    requirement_id: Option<&str>,
) -> Vec<AuthorizationFinding> {
    let mut findings = event
        .checks
        .iter()
        .flat_map(|run| {
            run.findings
                .iter()
                .enumerate()
                .map(move |(index, finding)| {
                    let recommended = finding
                        .recommended_effect
                        .unwrap_or(AuthorizationEffect::Permit);
                    let effect = if run.mode == EnforcementMode::Enforce {
                        recommended
                    } else {
                        AuthorizationEffect::Permit
                    };
                    AuthorizationFinding {
                        id: format!("checker:{}:{index}", run.checker_id),
                        source: run.checker_id.clone(),
                        effect,
                        reason: finding.reason.clone(),
                        severity: Severity::Medium,
                        policy_id: None,
                        requirement_id: (effect == AuthorizationEffect::RequireApproval)
                            .then(|| requirement_id.map(str::to_string))
                            .flatten(),
                        remediation: decision.remediation.clone(),
                        evidence: serde_json::json!({
                            "rule": finding.rule,
                            "recommended_effect": recommended,
                            "mode": run.mode,
                            "source_chain": finding.source_chain,
                            "risk_source": finding.risk_source,
                            "risk_code": finding.risk_code,
                            "harm_class": finding.harm_class,
                        }),
                    }
                })
        })
        .collect::<Vec<_>>();
    findings.extend(decision.triggered_policies.iter().map(|policy| {
        AuthorizationFinding {
            id: format!("policy:{}", policy.id),
            source: "policy".into(),
            effect: decision.effect,
            reason: policy.reason.clone(),
            severity: policy.severity,
            policy_id: Some(policy.id.clone()),
            requirement_id: (decision.effect == AuthorizationEffect::RequireApproval)
                .then(|| requirement_id.map(str::to_string))
                .flatten(),
            remediation: decision.remediation.clone(),
            evidence: serde_json::Value::Null,
        }
    }));
    if findings.is_empty() && decision.effect != AuthorizationEffect::Permit {
        findings.push(AuthorizationFinding {
            id: format!("event:{}", decision.trace_id),
            source: "event_pipeline".into(),
            effect: decision.effect,
            reason: decision.reason.clone(),
            severity: Severity::Medium,
            policy_id: None,
            requirement_id: (decision.effect == AuthorizationEffect::RequireApproval)
                .then(|| requirement_id.map(str::to_string))
                .flatten(),
            remediation: decision.remediation.clone(),
            evidence: serde_json::Value::Null,
        });
    }
    findings
}

fn authorization_error(error: crate::authorization::AuthorizationError) -> Response {
    use crate::authorization::{AuthorizationError, AuthorizationStoreError};
    match error {
        AuthorizationError::Store(AuthorizationStoreError::NotFound) => api_error_response(
            StatusCode::NOT_FOUND,
            ApiErrorCode::NotFound,
            "authorization resource was not found".into(),
        ),
        AuthorizationError::Store(AuthorizationStoreError::Conflict(message))
        | AuthorizationError::Conflict(message) => {
            api_error_response(StatusCode::CONFLICT, ApiErrorCode::Invalid, message)
        }
        AuthorizationError::Store(AuthorizationStoreError::Invalid(message))
        | AuthorizationError::Invalid(message) => api_error_response(
            StatusCode::UNPROCESSABLE_ENTITY,
            ApiErrorCode::Unprocessable,
            message,
        ),
        AuthorizationError::Adapter(error) => api_error_response(
            StatusCode::UNPROCESSABLE_ENTITY,
            ApiErrorCode::Unprocessable,
            error.to_string(),
        ),
        AuthorizationError::Store(AuthorizationStoreError::Internal(message))
        | AuthorizationError::Policy(message) => {
            tracing::error!(error = %message, "authorization coordination failed");
            api_error_response(
                StatusCode::INTERNAL_SERVER_ERROR,
                ApiErrorCode::Internal,
                "authorization coordination failed".into(),
            )
        }
    }
}

fn trace_domain(event: &GuardEvent) -> &'static str {
    let integration_mode = event
        .context
        .get("integration_mode")
        .and_then(serde_json::Value::as_str);
    let phase = event
        .context
        .get("gateway_phase")
        .and_then(serde_json::Value::as_str);
    match (integration_mode, phase) {
        (Some("gateway"), Some("gateway_input_check"))
        | (Some("hosted_mcp"), Some("mcp_preflight")) => "gateway_input",
        (Some("gateway"), Some("gateway_output_check"))
        | (Some("hosted_mcp"), Some("mcp_result_disclosure")) => "gateway_output",
        _ => EVENT_TRACE_DOMAIN,
    }
}

async fn record_semantic_usage(
    state: &AppState,
    workspace_id: &str,
    environment_id: &str,
    event: &GuardEvent,
    trace_id: &str,
    invocations: Vec<tl_llm::LlmCallAudit>,
) {
    if invocations.is_empty() {
        return;
    }
    let phase = event
        .context
        .get("gateway_phase")
        .and_then(serde_json::Value::as_str)
        .unwrap_or("event_policy");
    let gateway_request_id = event
        .context
        .get("gateway_request_id")
        .and_then(serde_json::Value::as_str);

    for (index, invocation) in invocations.into_iter().enumerate() {
        let price = match invocation.model.as_deref() {
            Some(model) => {
                crate::llm_pricing::model_price(
                    state.llm_pricing_store.as_ref(),
                    workspace_id,
                    model,
                )
                .await
            }
            None => None,
        };
        let cost_nanos =
            match (
                price,
                invocation.prompt_tokens,
                invocation.completion_tokens,
            ) {
                (Some(price), Some(prompt), Some(completion)) => Some(
                    crate::llm_pricing::cost_nanos(price, i64::from(prompt), i64::from(completion)),
                ),
                _ => None,
            };
        let evidence = RunGuardrailUsage {
            phase: phase.to_string(),
            judge: invocation.judge.clone(),
            provider: invocation.provider.clone(),
            model: invocation.model.clone(),
            status: invocation.status.clone(),
            prompt_tokens: invocation.prompt_tokens.map(i64::from),
            completion_tokens: invocation.completion_tokens.map(i64::from),
            estimated_cost_usd_nanos: cost_nanos.map(|cost| cost.to_string()),
            fallback_used: invocation.fallback_used,
            latency_ms: invocation.latency_ms,
            error_code: invocation.error_code.clone(),
        };

        if let (Some(model), Some(prompt_tokens), Some(completion_tokens)) = (
            invocation.model,
            invocation.prompt_tokens,
            invocation.completion_tokens,
        ) {
            let recorded_cost_nanos = cost_nanos.unwrap_or(0);
            let request_id = format!("guardrail:{trace_id}:{index}");
            if let Err(error) = state
                .llm_usage_store
                .insert_event(
                    workspace_id,
                    crate::llm_usage::RecordLlmUsageEvent {
                        principal_id: "trustloopguard:guardrail".to_string(),
                        api_key_id: "trustloopguard".to_string(),
                        kind: LlmUsageKind::Guardrail,
                        model,
                        prompt_tokens: i64::from(prompt_tokens),
                        completion_tokens: i64::from(completion_tokens),
                        cost_minor: recorded_cost_nanos / crate::llm_pricing::NANOS_PER_MINOR,
                        cost_nanos: recorded_cost_nanos,
                        currency: USD.to_string(),
                        request_id,
                        metadata: serde_json::json!({
                            "trace_id": trace_id,
                            "run_id": event.principal.run_id,
                            "run_event_id": event.principal.run_event_id,
                            "gateway_request_id": gateway_request_id,
                            "phase": phase,
                            "judge": evidence.judge,
                            "provider": evidence.provider,
                            "fallback_used": evidence.fallback_used,
                            "latency_ms": evidence.latency_ms,
                            "priced": cost_nanos.is_some(),
                        }),
                    },
                )
                .await
            {
                tracing::warn!(workspace_id, trace_id, error = %error, "guardrail usage record failed");
            }
        }

        let Some(run_id) = event.principal.run_id.as_deref() else {
            continue;
        };
        let run_event = CreateRunEventRequest {
            kind: RunEventKind::SystemEvent,
            sequence: None,
            label: Some("Guardrail LLM usage".to_string()),
            input_summary: None,
            output_summary: None,
            metadata: serde_json::json!({
                "integration_mode": event.context.get("integration_mode"),
                "gateway_request_id": gateway_request_id,
                "evidence_kind": "guardrail_usage",
                "guardrail_usage": evidence,
            }),
            occurred_at: None,
        };
        if let Err(error) = state
            .run_store
            .create_event(workspace_id, environment_id, run_id, run_event)
            .await
        {
            tracing::warn!(workspace_id, run_id, error = %error, "guardrail run evidence record failed");
        }
    }
}

fn effect_rank(effect: AuthorizationEffect) -> u8 {
    match effect {
        AuthorizationEffect::Permit => 0,
        AuthorizationEffect::Transform => 1,
        AuthorizationEffect::RequireApproval => 2,
        AuthorizationEffect::Defer => 3,
        AuthorizationEffect::Deny => 4,
    }
}

fn controlling_approval_rule(event: &GuardEvent) -> Option<&ApprovalRule> {
    let ToolResolution::Resolved { metadata } = event.resolution.as_ref()? else {
        return None;
    };
    metadata.approval.as_ref().filter(|rule| rule.required)
}

#[allow(clippy::result_large_err)]
fn validate_authorization(authorization: &AuthorizationClaim) -> Result<(), Response> {
    if uuid::Uuid::parse_str(&authorization.grant_id).is_err()
        || uuid::Uuid::parse_str(&authorization.attempt_id).is_err()
    {
        return Err(api_error_response(
            StatusCode::UNPROCESSABLE_ENTITY,
            ApiErrorCode::Unprocessable,
            "grant and attempt ids must be UUIDs".into(),
        ));
    }
    Ok(())
}

fn effect_name(effect: AuthorizationEffect) -> &'static str {
    match effect {
        AuthorizationEffect::Permit => "permit",
        AuthorizationEffect::Transform => "transform",
        AuthorizationEffect::Deny => "deny",
        AuthorizationEffect::RequireApproval => "require_approval",
        AuthorizationEffect::Defer => "defer",
    }
}

/// Bound submitted events so a single request cannot carry unbounded
/// payloads into the pipeline and trace storage. Returns the first
/// violation as a human-readable message (422 at the handler).
fn validate_event(event: &GuardEvent) -> Result<(), String> {
    let agent_id = event.principal.agent_id.trim();
    if agent_id.is_empty() {
        return Err("principal.agent_id must not be empty".into());
    }
    if agent_id.len() > MAX_ID_BYTES {
        return Err(format!(
            "principal.agent_id must be at most {MAX_ID_BYTES} bytes"
        ));
    }

    if let Some(session_id) = event.principal.session_id.as_deref() {
        if session_id.len() > MAX_ID_BYTES {
            return Err(format!(
                "principal.session_id must be at most {MAX_ID_BYTES} bytes"
            ));
        }
    }

    let operation = event.action.operation.trim();
    if operation.is_empty() {
        return Err("action.operation must not be empty".into());
    }
    if operation.len() > MAX_ID_BYTES {
        return Err(format!(
            "action.operation must be at most {MAX_ID_BYTES} bytes"
        ));
    }
    if let Some(invocation_id) = event.action.invocation_id.as_deref() {
        if invocation_id.trim().is_empty() || invocation_id.len() > MAX_ID_BYTES {
            return Err(format!(
                "action.invocation_id must be non-empty and at most {MAX_ID_BYTES} bytes"
            ));
        }
    }
    if let Some(identity) = event.action.tool_identity.as_ref() {
        for (name, value) in [
            ("server_id", identity.server_id.as_str()),
            ("tool_name", identity.tool_name.as_str()),
            ("schema_hash", identity.schema_hash.as_str()),
        ] {
            if value.trim().is_empty() || value.len() > MAX_ID_BYTES {
                return Err(format!(
                    "action.tool_identity.{name} must be non-empty and at most {MAX_ID_BYTES} bytes"
                ));
            }
        }
    }

    if event.sources.len() > MAX_SOURCES {
        return Err(format!("at most {MAX_SOURCES} sources are allowed"));
    }
    let mut seen_ids = std::collections::BTreeSet::new();
    for (index, source) in event.sources.iter().enumerate() {
        let id = source.id.trim();
        if id.is_empty() {
            return Err("source ids must not be empty".into());
        }
        if id.len() > MAX_ID_BYTES {
            return Err(format!("source ids must be at most {MAX_ID_BYTES} bytes"));
        }
        if !seen_ids.insert(id) {
            // Positional reference; caller-supplied ids are never
            // reflected back in error messages.
            return Err(format!("duplicate source id at index {index}"));
        }
        if let Some(kind) = source.kind.as_deref() {
            let kind = kind.trim();
            if kind.is_empty() {
                return Err("source kinds must not be blank when provided".into());
            }
            if kind.len() > MAX_ID_BYTES {
                return Err(format!("source kinds must be at most {MAX_ID_BYTES} bytes"));
            }
        }
    }

    if event.provenance.0.len() > MAX_PROVENANCE_PATHS {
        return Err(format!(
            "at most {MAX_PROVENANCE_PATHS} provenance paths are allowed"
        ));
    }
    let mut seen_paths = std::collections::BTreeSet::new();
    for (path, source_ids) in &event.provenance.0 {
        let path = path.trim();
        if path.is_empty() {
            return Err("provenance paths must not be empty".into());
        }
        if path.len() > MAX_PATH_BYTES {
            return Err(format!(
                "provenance paths must be at most {MAX_PATH_BYTES} bytes"
            ));
        }
        if !seen_paths.insert(path) {
            // The map keys are distinct only by surrounding whitespace.
            return Err("duplicate provenance path after trimming whitespace".into());
        }
        if source_ids.len() > MAX_SOURCES_PER_PATH {
            return Err(format!(
                "at most {MAX_SOURCES_PER_PATH} source ids are allowed per provenance path"
            ));
        }
        for id in source_ids {
            if id.len() > MAX_ID_BYTES {
                return Err(format!(
                    "provenance source ids must be at most {MAX_ID_BYTES} bytes"
                ));
            }
        }
    }

    if serialized_len(&event.action.parameters) > MAX_PARAMETERS_BYTES {
        return Err(format!(
            "action.parameters must serialize to at most {MAX_PARAMETERS_BYTES} bytes"
        ));
    }
    if event.kind == tl_core::EventKind::ShellActionProposed {
        let parameters: ShellActionParameters =
            serde_json::from_value(event.action.parameters.clone()).map_err(|_| {
                "action.parameters must be valid shell action parameters".to_string()
            })?;
        if parameters.command.trim().is_empty() {
            return Err("action.parameters.command must not be empty".into());
        }
        if parameters.command.len() > MAX_PARAMETERS_BYTES {
            return Err(format!(
                "action.parameters.command must be at most {MAX_PARAMETERS_BYTES} bytes"
            ));
        }
        for (name, value) in [
            ("cwd", parameters.cwd.as_deref()),
            ("workspace_root", parameters.workspace_root.as_deref()),
        ] {
            if value.is_some_and(|value| value.len() > 4_096) {
                return Err(format!(
                    "action.parameters.{name} must be at most 4096 bytes"
                ));
            }
        }
        if parameters.timeout_ms == Some(0) {
            return Err("action.parameters.timeout_ms must be greater than zero".into());
        }
    }
    if serialized_len(&event.context) > MAX_CONTEXT_BYTES {
        return Err(format!(
            "context must serialize to at most {MAX_CONTEXT_BYTES} bytes"
        ));
    }

    Ok(())
}

/// Fail closed: a `serde_json::Value` should always serialize, but if it
/// ever does not, treat the payload as oversized rather than empty so a
/// serialization quirk can never bypass the byte caps.
fn serialized_len(value: &serde_json::Value) -> usize {
    serde_json::to_string(value)
        .map(|s| s.len())
        .unwrap_or(usize::MAX)
}

#[cfg(test)]
mod tests {
    use super::*;
    use tl_core::{Action, EventKind, Principal, ProvenanceMap, Source};

    fn event() -> GuardEvent {
        GuardEvent {
            kind: EventKind::ToolCallProposed,
            principal: Principal {
                workspace_id: "ws_1".into(),
                environment_id: "production".into(),
                agent_id: "agent-1".into(),
                user_id: None,
                session_id: None,
                task_id: None,
                run_id: None,
                run_event_id: None,
            },
            action: Action {
                operation: "send_email".into(),
                parameters: serde_json::json!({ "recipient": "a@b.c" }),
                side_effect: None,
                invocation_id: None,
                tool_identity: None,
                authorization: None,
            },
            sources: vec![],
            provenance: ProvenanceMap::default(),
            resolution: None,
            label_resolution: None,
            checks: vec![],
            signals: vec![],
            context: serde_json::Value::Null,
        }
    }

    fn source(id: &str) -> Source {
        Source {
            id: id.into(),
            origin: tl_core::Origin::Web,
            labels: tl_core::Labels::default(),
            kind: None,
        }
    }

    #[test]
    fn accepts_minimal_event() {
        assert!(validate_event(&event()).is_ok());
    }

    #[test]
    fn rejects_empty_agent_and_operation() {
        let mut e = event();
        e.principal.agent_id = "  ".into();
        assert!(validate_event(&e).unwrap_err().contains("agent_id"));

        let mut e = event();
        e.action.operation = "".into();
        assert!(validate_event(&e).unwrap_err().contains("operation"));
    }

    #[test]
    fn rejects_too_many_sources() {
        let mut e = event();
        e.sources = (0..=MAX_SOURCES)
            .map(|i| source(&format!("s{i}")))
            .collect();
        assert!(validate_event(&e).unwrap_err().contains("sources"));
    }

    #[test]
    fn rejects_duplicate_source_ids() {
        let mut e = event();
        e.sources = vec![source("dup"), source("dup")];
        assert!(validate_event(&e).unwrap_err().contains("duplicate"));
    }

    #[test]
    fn rejects_oversized_provenance() {
        let mut e = event();
        for i in 0..=MAX_PROVENANCE_PATHS {
            e.provenance.insert(format!("p{i}"), vec!["s".into()]);
        }
        assert!(validate_event(&e).unwrap_err().contains("provenance"));

        let mut e = event();
        e.provenance.insert(
            "param",
            (0..=MAX_SOURCES_PER_PATH)
                .map(|i| format!("s{i}"))
                .collect(),
        );
        assert!(validate_event(&e)
            .unwrap_err()
            .contains("per provenance path"));
    }

    #[test]
    fn rejects_oversized_parameters() {
        let mut e = event();
        e.action.parameters = serde_json::json!({ "blob": "x".repeat(MAX_PARAMETERS_BYTES) });
        assert!(validate_event(&e).unwrap_err().contains("parameters"));
    }
}
