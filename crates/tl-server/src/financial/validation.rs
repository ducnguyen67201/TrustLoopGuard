use tl_core::{CreateFinancialActionRequest, FinancialExecutionStatus};

use super::FinancialStoreError;

pub(super) fn validate_create_action(
    input: &CreateFinancialActionRequest,
) -> Result<(), FinancialStoreError> {
    clean_required("idempotency_key", &input.idempotency_key)?;
    clean_operation(&input.action.operation)?;
    clean_required("principal_id", &input.action.principal_id)?;
    clean_required("currency", &input.action.amount.currency)?;
    if input.action.amount.amount_minor <= 0 {
        return Err(FinancialStoreError::Validation(
            "amount.amount_minor must be positive".into(),
        ));
    }
    Ok(())
}

fn clean_operation(operation: &str) -> Result<(), FinancialStoreError> {
    let trimmed = operation.trim();
    if trimmed.is_empty() {
        return Err(FinancialStoreError::Validation(
            "operation must not be empty".into(),
        ));
    }
    if trimmed.len() > 128
        || !trimmed
            .chars()
            .all(|ch| ch.is_ascii_lowercase() || ch.is_ascii_digit() || ch == '_' || ch == '-')
    {
        return Err(FinancialStoreError::Validation(
            "operation must be lowercase ASCII, digits, '_' or '-'".into(),
        ));
    }
    Ok(())
}

pub(super) fn clean_required(name: &str, value: &str) -> Result<String, FinancialStoreError> {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        return Err(FinancialStoreError::Validation(format!(
            "{name} must not be empty"
        )));
    }
    Ok(trimmed.to_string())
}

pub(super) fn is_valid_execution_transition(
    from: FinancialExecutionStatus,
    to: FinancialExecutionStatus,
) -> bool {
    use FinancialExecutionStatus::*;
    matches!(
        (from, to),
        (NotStarted, Executing | Canceled)
            | (Executing, Succeeded | Failed | Canceled)
            | (Failed, Executing | Canceled)
            | (Succeeded, Reversed)
    )
}
