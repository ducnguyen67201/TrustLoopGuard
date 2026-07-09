use sha2::{Digest, Sha256};
use tl_core::{
    MoneyAmount, X402NormalizedPaymentRequirement, X402PaymentRequirement, X402SettlementProof,
};

use super::FinancialStoreError;

pub(super) fn normalize_payment_requirement(
    input: &X402PaymentRequirement,
) -> Result<X402NormalizedPaymentRequirement, FinancialStoreError> {
    if input.amount.amount_minor <= 0 {
        return Err(FinancialStoreError::Validation(
            "payment_requirement.amount.amount_minor must be positive".into(),
        ));
    }
    let currency = clean_required(
        "payment_requirement.amount.currency",
        &input.amount.currency,
    )?
    .to_uppercase();
    let pay_to = clean_required("payment_requirement.pay_to", &input.pay_to)?;
    let normalized_pay_to = normalize_pay_to(&pay_to);
    let method = input
        .method
        .as_deref()
        .map(|value| {
            clean_required("payment_requirement.method", value).map(|value| value.to_uppercase())
        })
        .transpose()?;
    let host = input
        .host
        .as_deref()
        .map(|value| {
            clean_required("payment_requirement.host", value).map(|value| value.to_lowercase())
        })
        .transpose()?;
    let resource = input
        .resource
        .as_deref()
        .map(|value| clean_required("payment_requirement.resource", value))
        .transpose()?;
    let network = input
        .network
        .as_deref()
        .map(|value| {
            clean_required("payment_requirement.network", value).map(|value| value.to_lowercase())
        })
        .transpose()?;
    let asset = input
        .asset
        .as_deref()
        .map(|value| {
            clean_required("payment_requirement.asset", value).map(|value| value.to_uppercase())
        })
        .transpose()?;
    let scheme = input
        .scheme
        .as_deref()
        .map(|value| {
            clean_required("payment_requirement.scheme", value).map(|value| value.to_lowercase())
        })
        .transpose()?;
    let facilitator = input
        .facilitator
        .as_deref()
        .map(|value| clean_required("payment_requirement.facilitator", value))
        .transpose()?;
    let amount = MoneyAmount {
        amount_minor: input.amount.amount_minor,
        currency,
    };
    let canonical = serde_json::json!({
        "amount": amount,
        "asset": asset,
        "facilitator": facilitator,
        "host": host,
        "method": method,
        "network": network,
        "pay_to": pay_to,
        "normalized_pay_to": normalized_pay_to,
        "resource": resource,
        "scheme": scheme,
    });
    let encoded = serde_json::to_vec(&canonical)
        .map_err(|e| FinancialStoreError::Internal(format!("x402 canonical encode: {e}")))?;
    let hash = Sha256::digest(encoded);
    let hash_hex = hash
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect::<String>();
    Ok(X402NormalizedPaymentRequirement {
        payment_requirement_hash: format!("sha256:{hash_hex}"),
        amount,
        pay_to,
        normalized_pay_to: Some(normalized_pay_to),
        network,
        asset,
        scheme,
        resource,
        method,
        host,
        facilitator,
        canonical,
    })
}

pub(super) fn verify_settlement_proof(
    normalized: &X402NormalizedPaymentRequirement,
    proof: &X402SettlementProof,
) -> Result<(), FinancialStoreError> {
    let has_reference = proof
        .settlement_reference
        .as_deref()
        .map(str::trim)
        .is_some_and(|value| !value.is_empty());
    if !has_reference && proof.payment_response.is_null() && proof.raw.is_null() {
        return Err(FinancialStoreError::Validation(
            "settlement proof must include a settlement reference or provider response".into(),
        ));
    }
    if let Some(hash) = proof.payment_requirement_hash.as_deref() {
        if hash != normalized.payment_requirement_hash {
            return Err(FinancialStoreError::Validation(
                "settlement proof payment requirement hash does not match authorization".into(),
            ));
        }
    }
    if let Some(amount) = &proof.amount {
        if amount.amount_minor != normalized.amount.amount_minor
            || !amount
                .currency
                .eq_ignore_ascii_case(&normalized.amount.currency)
        {
            return Err(FinancialStoreError::Validation(
                "settlement proof amount does not match authorization".into(),
            ));
        }
    }
    if let Some(pay_to) = proof.pay_to.as_deref() {
        if normalize_pay_to(pay_to) != normalize_pay_to(&normalized.pay_to) {
            return Err(FinancialStoreError::Validation(
                "settlement proof pay_to does not match authorization".into(),
            ));
        }
    }
    if let (Some(expected), Some(actual)) =
        (normalized.network.as_deref(), proof.network.as_deref())
    {
        if !expected.eq_ignore_ascii_case(actual) {
            return Err(FinancialStoreError::Validation(
                "settlement proof network does not match authorization".into(),
            ));
        }
    }
    if let (Some(expected), Some(actual)) = (normalized.asset.as_deref(), proof.asset.as_deref()) {
        if !expected.eq_ignore_ascii_case(actual) {
            return Err(FinancialStoreError::Validation(
                "settlement proof asset does not match authorization".into(),
            ));
        }
    }
    Ok(())
}

fn clean_required(name: &str, value: &str) -> Result<String, FinancialStoreError> {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        return Err(FinancialStoreError::Validation(format!(
            "{name} must not be empty"
        )));
    }
    Ok(trimmed.to_string())
}

fn normalize_pay_to(value: &str) -> String {
    value.trim().to_lowercase()
}
