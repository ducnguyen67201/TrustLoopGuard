//! Versioned exact-subject fingerprinting for executable event intents.

use sha2::{Digest, Sha256};
use tl_core::GuardEvent;

pub const FINGERPRINT_VERSION: i32 = 1;

pub fn action_fingerprint(
    event: &GuardEvent,
    workspace_id: &str,
    environment_id: &str,
) -> Result<String, serde_json::Error> {
    let value = serde_json::json!({
        "version": FINGERPRINT_VERSION,
        "workspace_id": workspace_id,
        "environment_id": environment_id,
        "principal_id": event.principal.agent_id,
        "session_id": event.principal.session_id,
        "run_id": event.principal.run_id,
        "run_event_id": event.principal.run_event_id,
        "event_kind": event.kind,
        "invocation_id": event.action.invocation_id,
        "operation": event.action.operation,
        "tool_identity": event.action.tool_identity,
        "side_effect": event.action.side_effect,
        "parameters": event.action.parameters,
    });
    let encoded = serde_json::to_vec(&value)?;
    let digest = Sha256::digest(encoded);
    Ok(format!(
        "sha256:v{FINGERPRINT_VERSION}:{}",
        digest
            .iter()
            .map(|byte| format!("{byte:02x}"))
            .collect::<String>()
    ))
}
