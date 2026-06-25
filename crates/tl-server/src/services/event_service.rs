//! Direct `GuardEvent` ingestion.
//!
//! Runtime `GuardEvent` submission: workspace gates, validation, the event
//! pipeline, enabled policy evaluation, and the fire-and-forget trace write.
//! No legacy tier engine runs here. The decision seeds as an allow; enforce-
//! mode checkers and enabled policies can upgrade it.

use axum::{http::StatusCode, response::Response};
use tl_core::{ApiErrorCode, DataHandlingMode, Decision, GuardEvent, Verdict};
use tl_engine::{evaluate_event_policies, EventPolicyEvalCtx};

use crate::{app::error::api_error_response, AppState};

/// Seed reason for `/v1/events` decisions. It survives only when no
/// enforce-mode checker fires and no enabled policy matches.
pub(crate) const DEFAULT_EVENT_ALLOW_REASON: &str =
    "event allowed: no enforced checker or enabled policy matched";

/// Trace `domain` for ingested events.
const EVENT_TRACE_DOMAIN: &str = "event";

const MAX_SOURCES: usize = 64;
const MAX_PROVENANCE_PATHS: usize = 128;
const MAX_SOURCES_PER_PATH: usize = 32;
/// Event ids are opaque, but they land in indexed columns and must stay small.
pub(super) const MAX_ID_BYTES: usize = 256;
const MAX_PATH_BYTES: usize = 512;
const MAX_PARAMETERS_BYTES: usize = 65_536;
const MAX_CONTEXT_BYTES: usize = 65_536;

pub(crate) async fn execute_event_submission(
    state: &AppState,
    workspace_id: &str,
    environment_id: &str,
    event: GuardEvent,
    start: std::time::Instant,
) -> Result<Decision, Response> {
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

    // No tier engine: events are evaluated by the event pipeline and
    // enabled policies.
    let mut decision = Decision::allow(tl_core::new_trace_id());
    decision.reason = DEFAULT_EVENT_ALLOW_REASON.into();

    let modes =
        super::resolve_checker_modes(state, workspace_id, environment_id, &workspace_settings)
            .await?;
    let pipeline_start = std::time::Instant::now();
    let (event, mut decision) = state
        .event_pipeline
        .process(event, workspace_id, environment_id, modes, decision)
        .await;
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
    decision.triggered_policies.extend(policy_outcome.triggered);
    if let Some(policy_verdict) = policy_outcome.verdict {
        if verdict_rank(policy_verdict) > verdict_rank(decision.verdict) {
            decision.verdict = policy_verdict;
            if let Some(reason) = policy_outcome.reason {
                decision.reason = reason;
            }
            decision.safe_output = match policy_verdict {
                Verdict::Rewrite => policy_outcome.safe_output,
                _ => None,
            };
        }
    }

    decision.latency_ms = start.elapsed().as_millis() as u64;

    tracing::info!(
        workspace_id,
        environment_id,
        verdict = ?decision.verdict,
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
                verdict_name(decision.verdict),
                decision.latency_ms as i32,
            )
            .await
        {
            tracing::warn!(run_id, error = %e, "could not update run stats");
        }
    }

    let agent_id = event.principal.agent_id.clone();

    #[cfg(feature = "postgres")]
    if let Some(tx) = state.trace_tx.as_ref() {
        let trace = tl_storage::TraceWrite {
            decision: decision.clone(),
            run_id: event.principal.run_id.clone(),
            run_event_id: event.principal.run_event_id.clone(),
            session_id: event.principal.session_id.clone(),
            event: Some(event),
            workspace_id: workspace_id.to_string(),
            environment_id: environment_id.to_string(),
            domain: EVENT_TRACE_DOMAIN.to_string(),
        };
        if let Err(e) = tx.try_send(trace) {
            tracing::warn!(error = %e, "trace channel full or closed; dropped");
        }
    }

    // Enforce-mode checkers and enabled policies can escalate event
    // decisions; route them to the shared escalation worker.
    if decision.verdict == tl_core::Verdict::Escalate {
        if let Some(tx) = state.escalation_tx.as_ref() {
            let payload = crate::escalation::EscalationPayload {
                trace_id: decision.trace_id.clone(),
                agent_id,
                domain: EVENT_TRACE_DOMAIN.to_string(),
                decision: decision.clone(),
            };
            if let Err(e) = tx.try_send(payload) {
                tracing::warn!(error = %e, "escalation channel full or closed; dropped");
            }
        }
    }

    Ok(decision)
}

fn verdict_rank(verdict: Verdict) -> u8 {
    match verdict {
        Verdict::Allow => 0,
        Verdict::Rewrite => 1,
        Verdict::Escalate => 2,
        Verdict::Block => 3,
    }
}

fn verdict_name(verdict: Verdict) -> &'static str {
    match verdict {
        Verdict::Allow => "allow",
        Verdict::Rewrite => "rewrite",
        Verdict::Block => "block",
        Verdict::Escalate => "escalate",
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
