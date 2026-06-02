use tl_core::HumanReviewEvent;

use crate::{models::HumanReviewEventRecord, StorageError};

use super::text::parse_outcome;

pub(super) fn event_summary(
    record: HumanReviewEventRecord,
) -> Result<HumanReviewEvent, StorageError> {
    Ok(HumanReviewEvent {
        id: record.id.to_string(),
        workspace_id: record.workspace_id,
        trace_id: record.trace_id.to_string(),
        run_id: record.run_id.map(|id| id.to_string()),
        run_event_id: record.run_event_id.map(|id| id.to_string()),
        outcome: parse_outcome(&record.outcome)?,
        reason_codes: parse_reason_codes(record.reason_codes)?,
        note: record.note,
        reviewer_id: record.reviewer_id,
        metadata: record.metadata,
        created_at: record.created_at.to_rfc3339(),
    })
}

fn parse_reason_codes(value: serde_json::Value) -> Result<Vec<String>, StorageError> {
    match value {
        serde_json::Value::Array(values) => values
            .into_iter()
            .map(|value| match value {
                serde_json::Value::String(code) => Ok(code),
                _ => Err(StorageError::Internal(
                    "reason_codes contains a non-string value".into(),
                )),
            })
            .collect(),
        _ => Err(StorageError::Internal(
            "reason_codes must be a JSON array".into(),
        )),
    }
}
