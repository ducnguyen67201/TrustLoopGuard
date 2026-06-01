use chrono::DateTime;
use tl_core::CreateRunEventRequest;
use uuid::Uuid;

use crate::StorageError;

pub(super) fn parse_run_id(id: &str) -> Result<Uuid, StorageError> {
    Uuid::parse_str(id).map_err(|error| StorageError::Internal(format!("run_id parse: {error}")))
}

pub(super) fn normalize_metadata(value: serde_json::Value) -> serde_json::Value {
    if value.is_null() {
        serde_json::json!({})
    } else {
        value
    }
}

pub(super) fn validate_create_run_event(input: &CreateRunEventRequest) -> Result<(), StorageError> {
    if input.sequence.is_some_and(|sequence| sequence < 1) {
        return Err(StorageError::Internal(
            "sequence must be greater than 0".into(),
        ));
    }
    if let Some(occurred_at) = input.occurred_at.as_ref() {
        DateTime::parse_from_rfc3339(occurred_at)
            .map_err(|_| StorageError::Internal("occurred_at must be RFC 3339".into()))?;
    }
    validate_metadata(&input.metadata)
}

fn validate_metadata(value: &serde_json::Value) -> Result<(), StorageError> {
    if value.is_null() || value.is_object() {
        Ok(())
    } else {
        Err(StorageError::Internal(
            "metadata must be a JSON object".into(),
        ))
    }
}

pub(super) fn non_empty_string(value: &str) -> Option<String> {
    if value.is_empty() {
        None
    } else {
        Some(value.to_string())
    }
}
