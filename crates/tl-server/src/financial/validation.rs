use tl_core::{CreateFinancialActionRequest, FinancialActionStatus};

use super::FinancialStoreError;

pub(super) fn validate_create_action(
    input: &CreateFinancialActionRequest,
) -> Result<(), FinancialStoreError> {
    clean_required("idempotency_key", &input.idempotency_key)?;
    clean_required("principal_id", &input.action.principal_id)?;
    clean_required("currency", &input.action.amount.currency)?;
    if input.action.amount.amount_minor <= 0 {
        return Err(FinancialStoreError::Validation(
            "amount.amount_minor must be positive".into(),
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

pub(super) fn is_valid_transition(from: FinancialActionStatus, to: FinancialActionStatus) -> bool {
    use FinancialActionStatus::*;
    matches!(
        (from, to),
        (Proposed, Authorized | Held | Denied | Failed | Expired)
            | (Held, Authorized | Executed | Denied | Failed | Expired)
            | (Authorized, Executed | Denied | Failed | Expired)
            | (Executed, Reversed)
    )
}
