use tl_core::{HumanReviewOutcome, RunEventKind, RunKind, RunStatus};

use crate::StorageError;

pub(super) fn kind_text(kind: RunKind) -> &'static str {
    match kind {
        RunKind::ChatSession => "chat_session",
        RunKind::LiveCall => "live_call",
        RunKind::Workflow => "workflow",
        RunKind::Job => "job",
        RunKind::Other => "other",
    }
}

pub(super) fn parse_kind(value: &str) -> Result<RunKind, StorageError> {
    match value {
        "chat_session" => Ok(RunKind::ChatSession),
        "live_call" => Ok(RunKind::LiveCall),
        "workflow" => Ok(RunKind::Workflow),
        "job" => Ok(RunKind::Job),
        "other" => Ok(RunKind::Other),
        other => Err(StorageError::Internal(format!("unknown run kind: {other}"))),
    }
}

pub(super) fn status_text(status: RunStatus) -> &'static str {
    match status {
        RunStatus::Warming => "warming",
        RunStatus::Running => "running",
        RunStatus::Completed => "completed",
        RunStatus::Failed => "failed",
        RunStatus::Canceled => "canceled",
    }
}

pub(super) fn parse_status(value: &str) -> Result<RunStatus, StorageError> {
    match value {
        "warming" => Ok(RunStatus::Warming),
        "running" => Ok(RunStatus::Running),
        "completed" => Ok(RunStatus::Completed),
        "failed" => Ok(RunStatus::Failed),
        "canceled" => Ok(RunStatus::Canceled),
        other => Err(StorageError::Internal(format!(
            "unknown run status: {other}"
        ))),
    }
}

pub(super) fn event_kind_text(kind: RunEventKind) -> &'static str {
    match kind {
        RunEventKind::UserTurn => "user_turn",
        RunEventKind::AssistantTurn => "assistant_turn",
        RunEventKind::ToolCall => "tool_call",
        RunEventKind::WorkflowStep => "workflow_step",
        RunEventKind::Interruption => "interruption",
        RunEventKind::Retry => "retry",
        RunEventKind::SystemEvent => "system_event",
        RunEventKind::Other => "other",
    }
}

pub(super) fn parse_event_kind(value: &str) -> Result<RunEventKind, StorageError> {
    match value {
        "user_turn" => Ok(RunEventKind::UserTurn),
        "assistant_turn" => Ok(RunEventKind::AssistantTurn),
        "tool_call" => Ok(RunEventKind::ToolCall),
        "workflow_step" => Ok(RunEventKind::WorkflowStep),
        "interruption" => Ok(RunEventKind::Interruption),
        "retry" => Ok(RunEventKind::Retry),
        "system_event" => Ok(RunEventKind::SystemEvent),
        "other" => Ok(RunEventKind::Other),
        other => Err(StorageError::Internal(format!(
            "unknown run event kind: {other}"
        ))),
    }
}

pub(super) fn parse_review_outcome(value: &str) -> Result<HumanReviewOutcome, StorageError> {
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
