use base64::Engine as _;
use tl_core::{RedteamAttackSurface, RedteamDispatchRequest, RedteamDocumentTemplate};
use url::Url;

use crate::agents::{AgentStore, AgentStoreError};

use super::RedteamJobStoreError;

/// Valid attack profiles understood by the runner.
const PROFILES: [&str; 3] = ["fast", "full", "max"];

/// Loopback hosts the orchestrator is allowed to drive an attack against.
/// Mirrors the web edge allowlist (`apps/web/lib/arena-redteam.ts`). This is
/// the authoritative SSRF gate: the worker ultimately causes `target_url` to be
/// fetched, and a direct API caller (workspace key) bypasses the web edge, so
/// the allowlist must live here too — deny-by-default.
const ALLOWED_TARGET_HOSTS: [&str; 3] = ["127.0.0.1", "localhost", "::1"];
/// The only unregistered target retained for the local demo flow. All other
/// dispatch targets must be bound to a workspace agent profile.
const LOCAL_DEMO_TARGET: &str = "http://127.0.0.1:9102";
const MAX_DOCUMENT_TEMPLATE_BYTES: usize = 10 * 1024 * 1024;
/// Caps on the planned seeds a single dispatch may carry. Mirrors the web edge
/// (`dispatch/route.ts`); enforced here too because a direct API caller bypasses
/// the web layer.
const MAX_ATTACK_VECTORS: usize = 32;
const MAX_VECTOR_FIELD_CHARS: usize = 4000;

pub(super) fn validate_dispatch(
    input: &RedteamDispatchRequest,
) -> Result<(), RedteamJobStoreError> {
    let target = input.target_url.trim();
    if target.is_empty() {
        return Err(RedteamJobStoreError::Validation(
            "target_url is required".into(),
        ));
    }
    if !is_loopback_target(target) {
        return Err(RedteamJobStoreError::Validation(
            "target_url must be an http(s) loopback agent (127.0.0.1, localhost, or ::1)".into(),
        ));
    }
    if !PROFILES.contains(&input.profile.trim()) {
        return Err(RedteamJobStoreError::Validation(
            "profile must be one of: fast, full, max".into(),
        ));
    }
    validate_document_template(input)?;
    validate_attack_vectors(input)?;
    Ok(())
}

/// Bind a dispatch target to durable workspace-owned agent configuration.
///
/// A loopback host check alone is not an SSRF boundary because it exposes every
/// local port and path. Registered runs must exactly match the selected agent's
/// stored target. The fixed demo adapter remains available without an agent.
pub(super) async fn validate_target_binding(
    agent_store: Option<&dyn AgentStore>,
    workspace_id: &str,
    input: &RedteamDispatchRequest,
) -> Result<(), RedteamJobStoreError> {
    let Some(agent_id) = input
        .agent_id
        .as_deref()
        .map(str::trim)
        .filter(|agent_id| !agent_id.is_empty())
    else {
        return if targets_match(&input.target_url, LOCAL_DEMO_TARGET) {
            Ok(())
        } else {
            Err(RedteamJobStoreError::Validation(format!(
                "agent_id is required unless target_url is the local demo adapter {LOCAL_DEMO_TARGET}"
            )))
        };
    };

    let store = agent_store.ok_or_else(|| {
        RedteamJobStoreError::Unavailable(
            "registered agent target validation is not configured".into(),
        )
    })?;
    let agent = store
        .get(workspace_id, agent_id)
        .await
        .map_err(|error| match error {
            AgentStoreError::NotFound => RedteamJobStoreError::Validation(format!(
                "agent_id `{agent_id}` is not registered in this workspace"
            )),
            AgentStoreError::Validation(message) => RedteamJobStoreError::Validation(message),
            AgentStoreError::Internal(message) => RedteamJobStoreError::Internal(message),
        })?;
    let registered_target = agent.target_url.as_deref().ok_or_else(|| {
        RedteamJobStoreError::Validation(format!("agent `{agent_id}` has no registered target_url"))
    })?;
    if !is_loopback_target(registered_target) {
        return Err(RedteamJobStoreError::Validation(format!(
            "agent `{agent_id}` has an invalid registered target_url"
        )));
    }
    if !targets_match(&input.target_url, registered_target) {
        return Err(RedteamJobStoreError::Validation(format!(
            "target_url must exactly match the registered target for agent `{agent_id}`"
        )));
    }
    Ok(())
}

fn validate_attack_vectors(input: &RedteamDispatchRequest) -> Result<(), RedteamJobStoreError> {
    let Some(vectors) = &input.attack_vectors else {
        return Ok(());
    };
    if vectors.len() > MAX_ATTACK_VECTORS {
        return Err(RedteamJobStoreError::Validation(format!(
            "attack_vectors must not exceed {MAX_ATTACK_VECTORS} entries"
        )));
    }
    for vector in vectors {
        // All four string fields are forwarded to the runner and stored as JSONB;
        // a direct API caller bypasses the web edge, so bound every one here.
        for (label, value) in [
            ("goal", &vector.goal),
            ("technique", &vector.technique),
            ("target_operation", &vector.target_operation),
            ("injection_payload", &vector.injection_payload),
        ] {
            if value.trim().is_empty() {
                return Err(RedteamJobStoreError::Validation(format!(
                    "attack vector {label} must not be empty"
                )));
            }
            if value.len() > MAX_VECTOR_FIELD_CHARS {
                return Err(RedteamJobStoreError::Validation(format!(
                    "attack vector {label} must not exceed {MAX_VECTOR_FIELD_CHARS} characters"
                )));
            }
        }
    }
    Ok(())
}

fn validate_document_template(input: &RedteamDispatchRequest) -> Result<(), RedteamJobStoreError> {
    let Some(template) = &input.document_template else {
        return Ok(());
    };
    if input.attack_surface != RedteamAttackSurface::DocumentWorkflow {
        return Err(RedteamJobStoreError::Validation(
            "document_template is only valid for document_workflow".into(),
        ));
    }
    validate_template_fields(template)?;
    let looks_like_pdf = template
        .file_name
        .trim()
        .to_ascii_lowercase()
        .ends_with(".pdf")
        || template
            .media_type
            .trim()
            .eq_ignore_ascii_case("application/pdf");
    if !looks_like_pdf {
        return Err(RedteamJobStoreError::Validation(
            "document_template must be a PDF".into(),
        ));
    }
    let decoded = base64::engine::general_purpose::STANDARD
        .decode(template.data_base64.trim())
        .map_err(|_| RedteamJobStoreError::Validation("document_template is not base64".into()))?;
    if decoded.len() > MAX_DOCUMENT_TEMPLATE_BYTES {
        return Err(RedteamJobStoreError::Validation(
            "document_template must be 10 MB or smaller".into(),
        ));
    }
    if !decoded.starts_with(b"%PDF-") {
        return Err(RedteamJobStoreError::Validation(
            "document_template must contain PDF bytes".into(),
        ));
    }
    Ok(())
}

fn validate_template_fields(
    template: &RedteamDocumentTemplate,
) -> Result<(), RedteamJobStoreError> {
    let Some(fields) = &template.fields else {
        return Ok(());
    };
    if fields.is_empty()
        || fields
            .iter()
            .any(|(field, value)| field.trim().is_empty() || value.trim().is_empty())
    {
        return Err(RedteamJobStoreError::Validation(
            "document_template.fields must be a non-empty object".into(),
        ));
    }
    Ok(())
}

/// True only for an http(s) URL whose host is a loopback agent.
fn is_loopback_target(raw: &str) -> bool {
    let Ok(url) = Url::parse(raw) else {
        return false;
    };
    if url.scheme() != "http" && url.scheme() != "https" {
        return false;
    }
    match url.host_str() {
        Some(host) => {
            let host = host
                .trim_start_matches('[')
                .trim_end_matches(']')
                .to_ascii_lowercase();
            ALLOWED_TARGET_HOSTS.contains(&host.as_str())
        }
        None => false,
    }
}

fn targets_match(left: &str, right: &str) -> bool {
    match (Url::parse(left.trim()), Url::parse(right.trim())) {
        (Ok(left), Ok(right)) => left == right,
        _ => false,
    }
}

pub(super) fn clean_optional(value: Option<String>) -> Option<String> {
    value.and_then(|value| {
        let trimmed = value.trim();
        if trimmed.is_empty() {
            None
        } else {
            Some(trimmed.to_string())
        }
    })
}
