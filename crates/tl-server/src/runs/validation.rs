use serde_json::json;
use tl_core::{CreateRunEventRequest, CreateRunRequest, UpdateRunRequest};

use super::RunStoreError;

pub(super) fn validate_create_run(input: &CreateRunRequest) -> Result<(), RunStoreError> {
    if input.agent_id.trim().is_empty() {
        return Err(RunStoreError::Validation("agent_id is required".into()));
    }
    validate_metadata(&input.metadata)
}

pub(super) fn validate_update_run(input: &UpdateRunRequest) -> Result<(), RunStoreError> {
    if let Some(metadata) = input.metadata.as_ref() {
        validate_metadata(metadata)?;
    }
    if let Some(ended_at) = input.ended_at.as_ref() {
        chrono::DateTime::parse_from_rfc3339(ended_at)
            .map_err(|_| RunStoreError::Validation("ended_at must be RFC 3339".into()))?;
    }
    Ok(())
}

pub(crate) fn validate_create_run_event(
    input: &CreateRunEventRequest,
) -> Result<(), RunStoreError> {
    if input.sequence.is_some_and(|sequence| sequence < 1) {
        return Err(RunStoreError::Validation(
            "sequence must be greater than 0".into(),
        ));
    }
    if let Some(occurred_at) = input.occurred_at.as_ref() {
        chrono::DateTime::parse_from_rfc3339(occurred_at)
            .map_err(|_| RunStoreError::Validation("occurred_at must be RFC 3339".into()))?;
    }
    validate_metadata(&input.metadata)
}

pub(super) fn normalize_metadata(value: serde_json::Value) -> serde_json::Value {
    if value.is_null() {
        json!({})
    } else {
        value
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

fn validate_metadata(value: &serde_json::Value) -> Result<(), RunStoreError> {
    if value.is_null() || value.is_object() {
        Ok(())
    } else {
        Err(RunStoreError::Validation(
            "metadata must be a JSON object".into(),
        ))
    }
}
