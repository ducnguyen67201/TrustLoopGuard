use serde_json::json;
use tl_core::CreateHumanReviewEventRequest;

use super::HumanReviewStoreError;

pub(super) fn validate_create_event(
    input: &CreateHumanReviewEventRequest,
) -> Result<(), HumanReviewStoreError> {
    if !(input.metadata.is_null() || input.metadata.is_object()) {
        return Err(HumanReviewStoreError::Validation(
            "metadata must be a JSON object".into(),
        ));
    }
    if input
        .reason_codes
        .iter()
        .any(|value| value.trim().is_empty())
    {
        return Err(HumanReviewStoreError::Validation(
            "reason_codes must not contain empty values".into(),
        ));
    }
    Ok(())
}

pub(super) fn clean_string(value: &str) -> Option<String> {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        None
    } else {
        Some(trimmed.to_string())
    }
}

pub(super) fn normalize_metadata(value: serde_json::Value) -> serde_json::Value {
    if value.is_null() {
        json!({})
    } else {
        value
    }
}
