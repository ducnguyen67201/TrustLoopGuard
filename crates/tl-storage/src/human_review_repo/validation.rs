use tl_core::CreateHumanReviewEventRequest;
use uuid::Uuid;

use crate::StorageError;

pub(super) fn validate_create_event(
    input: &CreateHumanReviewEventRequest,
) -> Result<(), StorageError> {
    validate_metadata(&input.metadata)?;
    for code in &input.reason_codes {
        if code.trim().is_empty() {
            return Err(StorageError::Internal(
                "reason_codes must not contain empty values".into(),
            ));
        }
    }
    Ok(())
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

pub(super) fn clean_reason_codes(values: Vec<String>) -> Vec<String> {
    values
        .into_iter()
        .filter_map(|value| non_empty_string(value.trim()))
        .collect()
}

pub(super) fn normalize_metadata(value: serde_json::Value) -> serde_json::Value {
    if value.is_null() {
        serde_json::json!({})
    } else {
        value
    }
}

pub(super) fn non_empty_string(value: &str) -> Option<String> {
    if value.is_empty() {
        None
    } else {
        Some(value.to_string())
    }
}

pub(super) fn parse_uuid(label: &str, value: &str) -> Result<Uuid, StorageError> {
    Uuid::parse_str(value)
        .map_err(|error| StorageError::Internal(format!("{label} parse: {error}")))
}
