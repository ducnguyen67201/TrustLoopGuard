use tl_core::HumanReviewOutcome;

use crate::StorageError;

pub(super) fn outcome_text(outcome: HumanReviewOutcome) -> &'static str {
    match outcome {
        HumanReviewOutcome::Accepted => "accepted",
        HumanReviewOutcome::Corrected => "corrected",
        HumanReviewOutcome::Rejected => "rejected",
        HumanReviewOutcome::FalsePositive => "false_positive",
        HumanReviewOutcome::MissedIssue => "missed_issue",
        HumanReviewOutcome::Ignored => "ignored",
    }
}

pub(super) fn parse_outcome(value: &str) -> Result<HumanReviewOutcome, StorageError> {
    match value {
        "accepted" => Ok(HumanReviewOutcome::Accepted),
        "corrected" => Ok(HumanReviewOutcome::Corrected),
        "rejected" => Ok(HumanReviewOutcome::Rejected),
        "false_positive" => Ok(HumanReviewOutcome::FalsePositive),
        "missed_issue" => Ok(HumanReviewOutcome::MissedIssue),
        "ignored" => Ok(HumanReviewOutcome::Ignored),
        other => Err(StorageError::Internal(format!(
            "unknown human review outcome: {other}"
        ))),
    }
}
