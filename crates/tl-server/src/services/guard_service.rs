use axum::{http::StatusCode, response::Response};
use tl_core::{ApiErrorCode, CheckRequest, Decision};
use tl_engine::RawInput;

use crate::{app::error::api_error_response, escalation, redaction, AppState};

pub(crate) async fn execute_check_request(
    state: &AppState,
    workspace_id: &str,
    environment_id: &str,
    mut req: CheckRequest,
    check_start: std::time::Instant,
) -> Result<Decision, Response> {
    req.workspace_id = Some(workspace_id.to_string());
    let workspace_settings = match state.settings_store.get(workspace_id).await {
        Ok(settings) => settings,
        Err(e) => {
            return Err(api_error_response(
                StatusCode::INTERNAL_SERVER_ERROR,
                ApiErrorCode::Internal,
                format!("workspace settings resolution failed: {e}"),
            ));
        }
    };
    if redaction::requires_redaction_rejection(workspace_settings.data_handling_mode, &req) {
        return Err(api_error_response(
            StatusCode::BAD_REQUEST,
            ApiErrorCode::Invalid,
            "workspace requires redacted check content".into(),
        ));
    }
    if let Some(run_id) = req.run_id.as_deref() {
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
            Err(e) => return Err(run_store_api_error_response(e)),
        }
    }
    if let Some(run_event_id) = req.run_event_id.as_deref() {
        if uuid::Uuid::parse_str(run_event_id).is_err() {
            return Err(api_error_response(
                StatusCode::BAD_REQUEST,
                ApiErrorCode::Invalid,
                "run_event_id must be a UUID".into(),
            ));
        }
    }
    if req.run_event_id.is_some() && req.run_id.is_none() {
        return Err(api_error_response(
            StatusCode::BAD_REQUEST,
            ApiErrorCode::Invalid,
            "run_id is required when run_event_id is provided".into(),
        ));
    }
    if req.run_event.is_some() && req.run_event_id.is_some() {
        return Err(api_error_response(
            StatusCode::BAD_REQUEST,
            ApiErrorCode::Invalid,
            "provide either run_event or run_event_id, not both".into(),
        ));
    }
    if req.run_event.is_some() && req.run_id.is_none() {
        return Err(api_error_response(
            StatusCode::BAD_REQUEST,
            ApiErrorCode::Invalid,
            "run_id is required when run_event is provided".into(),
        ));
    }
    if let Some(run_event) = req.run_event.as_ref() {
        if let Err(error) = crate::runs::validate_create_run_event(run_event) {
            return Err(run_store_api_error_response(error));
        }
    }
    if redaction::should_apply_server_redaction(&req) {
        redaction::apply_server_redaction(&mut req);
    }
    if let (Some(run_id), Some(run_event)) = (req.run_id.clone(), req.run_event.take()) {
        match state
            .run_store
            .create_event(workspace_id, environment_id, &run_id, run_event)
            .await
        {
            Ok(event) => {
                req.run_event_id = Some(event.id);
            }
            Err(error) => return Err(run_store_api_error_response(error)),
        }
    }
    let runtime_policies = match state
        .policy_store
        .list_enabled(workspace_id, environment_id)
        .await
    {
        Ok(policies) => policies,
        Err(e) => {
            return Err(api_error_response(
                StatusCode::INTERNAL_SERVER_ERROR,
                ApiErrorCode::Internal,
                format!("runtime policy resolution failed: {e}"),
            ));
        }
    };
    let policies: Vec<_> = if runtime_policies.is_empty() {
        tracing::warn!(
            workspace_id,
            "runtime policy store returned no enabled policies; falling back to boot-loaded policy bundle"
        );
        state.engine.policies().to_vec()
    } else {
        runtime_policies
            .iter()
            .map(|policy| policy.as_ref().clone())
            .collect()
    };

    let mut decision = state
        .engine
        .check_async_with_policies(&req, &state.handler_ctx, &policies)
        .await;

    decision.latency_ms = check_start.elapsed().as_millis() as u64;
    attach_checked_text_excerpts(&mut decision, &req);

    // Observe-only event pipeline: normalizes the raw input into a
    // GuardEvent for trace evidence. All stages are no-ops, so the
    // decision passes through unchanged and no I/O joins the hot path.
    let raw = RawInput::LegacyCheck(req.clone());
    let (event, decision) = state
        .event_pipeline
        .process(&raw, workspace_id, environment_id, decision)
        .await;
    #[cfg(not(feature = "postgres"))]
    let _ = event;

    if let Some(run_id) = req.run_id.as_deref() {
        let verdict_str = match decision.verdict {
            tl_core::Verdict::Allow => "allow",
            tl_core::Verdict::Rewrite => "rewrite",
            tl_core::Verdict::Block => "block",
            tl_core::Verdict::Escalate => "escalate",
        };
        if let Err(e) = state
            .run_store
            .record_check(
                workspace_id,
                environment_id,
                run_id,
                verdict_str,
                decision.latency_ms as i32,
            )
            .await
        {
            tracing::warn!(run_id, error = %e, "could not update run stats");
        }
    }

    #[cfg(feature = "postgres")]
    if let Some(tx) = state.trace_tx.as_ref() {
        let trace = tl_storage::TraceWrite {
            decision: decision.clone(),
            event: Some(event),
            workspace_id: workspace_id.to_string(),
            environment_id: environment_id.to_string(),
            run_id: req.run_id.clone(),
            run_event_id: req.run_event_id.clone(),
            domain: req
                .domain
                .clone()
                .unwrap_or_else(|| "customer_support".to_string()),
        };
        if let Err(e) = tx.try_send(trace) {
            tracing::warn!(error = %e, "trace channel full or closed; dropped");
        }
    }

    if decision.verdict == tl_core::Verdict::Escalate {
        if let Some(tx) = state.escalation_tx.as_ref() {
            let payload = escalation::EscalationPayload {
                trace_id: decision.trace_id.clone(),
                agent_id: req.agent_id.clone(),
                domain: req
                    .domain
                    .clone()
                    .unwrap_or_else(|| "customer_support".to_string()),
                decision: decision.clone(),
            };
            if let Err(e) = tx.try_send(payload) {
                tracing::warn!(error = %e, "escalation channel full or closed; dropped");
            }
        }
    }

    Ok(decision)
}

fn attach_checked_text_excerpts(decision: &mut Decision, req: &CheckRequest) {
    if !is_gateway_full_body_check(&req.context) {
        return;
    }

    decision.checked_input_excerpt = checked_text_excerpt(&req.input);
    decision.checked_output_excerpt = checked_text_excerpt(&req.proposed_output);
}

fn is_gateway_full_body_check(context: &serde_json::Value) -> bool {
    let Some(object) = context.as_object() else {
        return false;
    };

    object
        .get("integration_mode")
        .and_then(serde_json::Value::as_str)
        == Some("gateway")
        && object
            .get("retention_mode")
            .and_then(serde_json::Value::as_str)
            == Some("full_body")
}

fn checked_text_excerpt(value: &str) -> Option<String> {
    const LIMIT: usize = 240;

    let collapsed = value.split_whitespace().collect::<Vec<_>>().join(" ");
    if collapsed.is_empty() {
        return None;
    }

    if collapsed.chars().count() <= LIMIT {
        return Some(collapsed);
    }

    let excerpt = collapsed.chars().take(LIMIT).collect::<String>();
    Some(format!("{excerpt}..."))
}

fn run_store_api_error_response(error: crate::runs::RunStoreError) -> Response {
    let (status, code) = match error {
        crate::runs::RunStoreError::NotFound => (StatusCode::NOT_FOUND, ApiErrorCode::NotFound),
        crate::runs::RunStoreError::Validation(_) => {
            (StatusCode::BAD_REQUEST, ApiErrorCode::Invalid)
        }
        crate::runs::RunStoreError::Internal(_) => {
            (StatusCode::INTERNAL_SERVER_ERROR, ApiErrorCode::Internal)
        }
    };
    api_error_response(status, code, error.to_string())
}
