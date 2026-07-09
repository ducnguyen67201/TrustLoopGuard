use serde_json::{Map, Value};
use sha2::{Digest, Sha256};
use tl_core::{CommitFinancialActionRequest, FinancialActionEvaluation, FinancialActionRecord};

use super::FinancialStoreError;

const AUTHORIZATION_SCHEMA: &str = "financial_action_authorization.v1";
const ATTESTATION_SCHEMA: &str = "tlg-financial-execution-attestation.v1";

pub(super) fn financial_action_hash(
    action: &FinancialActionRecord,
    evaluation: &FinancialActionEvaluation,
) -> Result<String, FinancialStoreError> {
    let environment_id = action.environment_id.as_deref().ok_or_else(|| {
        FinancialStoreError::Internal("financial action is missing environment binding".into())
    })?;
    let value = serde_json::json!({
        "schema": AUTHORIZATION_SCHEMA,
        "workspace_id": action.workspace_id,
        "environment_id": environment_id,
        "action_id": action.id,
        "action": action.action,
        "evidence": action.evidence,
        "evaluation": {
            "outcome": evaluation.outcome,
            "risks": evaluation.risks,
            "policy_ids": evaluation.policy_ids,
            "created_at": evaluation.created_at,
        }
    });
    let encoded = serde_json::to_vec(&sort_json(value)).map_err(|error| {
        FinancialStoreError::Internal(format!("canonical financial action encode: {error}"))
    })?;
    Ok(sha256_prefixed(&encoded))
}

pub(super) fn attestation_message(
    action_id: &str,
    request: &CommitFinancialActionRequest,
) -> Vec<u8> {
    let fields = [
        request.connector_id.as_str(),
        action_id,
        request.grant_id.as_str(),
        request.action_hash.as_str(),
        request.provider.as_str(),
        request.provider_reference.as_str(),
        request.provider_status.as_str(),
        request.executed_at.as_str(),
        request.idempotency_key.as_str(),
        request.provider_proof_sha256.as_str(),
    ];
    let mut message = String::from(ATTESTATION_SCHEMA);
    for field in fields {
        message.push('\n');
        message.push_str(&field.len().to_string());
        message.push(':');
        message.push_str(field);
    }
    message.into_bytes()
}

pub(super) fn proof_digest(proof: &str) -> String {
    sha256_prefixed(proof.as_bytes())
}

pub(super) fn sha256_prefixed(bytes: &[u8]) -> String {
    let digest = Sha256::digest(bytes);
    let hex = digest
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect::<String>();
    format!("sha256:{hex}")
}

pub(super) fn sort_json(value: Value) -> Value {
    match value {
        Value::Object(object) => {
            let mut entries = object.into_iter().collect::<Vec<_>>();
            entries.sort_by(|(left, _), (right, _)| left.cmp(right));
            Value::Object(
                entries
                    .into_iter()
                    .map(|(key, value)| (key, sort_json(value)))
                    .collect::<Map<_, _>>(),
            )
        }
        Value::Array(values) => Value::Array(values.into_iter().map(sort_json).collect()),
        value => value,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn recursive_sort_is_stable_and_preserves_array_order() {
        let left = sort_json(json!({"b": {"z": 1, "a": 2}, "a": [2, 1]}));
        let right = sort_json(json!({"a": [2, 1], "b": {"a": 2, "z": 1}}));
        assert_eq!(left, right);
        assert_ne!(left, sort_json(json!({"a": [1, 2], "b": {"a": 2, "z": 1}})));
    }

    #[test]
    fn attestation_message_uses_utf8_byte_lengths() {
        let request = CommitFinancialActionRequest {
            connector_id: "connector-é".into(),
            grant_id: "grant".into(),
            action_hash: "sha256:action".into(),
            provider: "provider".into(),
            provider_reference: "reference".into(),
            provider_status: "succeeded".into(),
            executed_at: "2026-07-09T00:00:00Z".into(),
            idempotency_key: "commit-1".into(),
            provider_proof: "proof".into(),
            provider_proof_sha256: "sha256:proof".into(),
            signature: String::new(),
        };
        let message = String::from_utf8(attestation_message("action", &request)).unwrap();
        assert!(message.contains("\n12:connector-é"));
    }
}
