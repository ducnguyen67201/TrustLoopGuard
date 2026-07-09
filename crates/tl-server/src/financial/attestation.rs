use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine as _};
use ring::hmac;
use tl_core::CommitFinancialActionRequest;

use super::{canonical, FinancialStoreError};

const SIGNATURE_PREFIX: &str = "v1=";

pub(super) fn verify(
    secret: &[u8],
    action_id: &str,
    request: &CommitFinancialActionRequest,
) -> Result<String, FinancialStoreError> {
    if canonical::proof_digest(&request.provider_proof) != request.provider_proof_sha256 {
        return Err(invalid_attestation());
    }
    let encoded = request
        .signature
        .strip_prefix(SIGNATURE_PREFIX)
        .ok_or_else(invalid_attestation)?;
    let signature = URL_SAFE_NO_PAD
        .decode(encoded)
        .map_err(|_| invalid_attestation())?;
    let message = canonical::attestation_message(action_id, request);
    hmac::verify(
        &hmac::Key::new(hmac::HMAC_SHA256, secret),
        &message,
        &signature,
    )
    .map_err(|_| invalid_attestation())?;
    Ok(canonical::sha256_prefixed(&message))
}

#[cfg(test)]
pub(super) fn sign(
    secret: &[u8],
    action_id: &str,
    request: &CommitFinancialActionRequest,
) -> String {
    let message = canonical::attestation_message(action_id, request);
    let signature = hmac::sign(&hmac::Key::new(hmac::HMAC_SHA256, secret), &message);
    format!(
        "{SIGNATURE_PREFIX}{}",
        URL_SAFE_NO_PAD.encode(signature.as_ref())
    )
}

fn invalid_attestation() -> FinancialStoreError {
    FinancialStoreError::Validation("invalid execution attestation".into())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn request() -> CommitFinancialActionRequest {
        CommitFinancialActionRequest {
            connector_id: "connector-1".into(),
            grant_id: "grant-1".into(),
            action_hash: "sha256:action".into(),
            provider: "stripe".into(),
            provider_reference: "pi_123".into(),
            provider_status: "succeeded".into(),
            executed_at: "2026-07-09T00:00:00Z".into(),
            idempotency_key: "commit-1".into(),
            provider_proof: "provider receipt".into(),
            provider_proof_sha256: canonical::proof_digest("provider receipt"),
            signature: String::new(),
        }
    }

    #[test]
    fn valid_signature_verifies_and_tampering_fails() {
        let secret = b"01234567890123456789012345678901";
        let mut request = request();
        request.signature = sign(secret, "action-1", &request);
        assert!(verify(secret, "action-1", &request).is_ok());

        request.provider_reference = "pi_changed".into();
        assert!(verify(secret, "action-1", &request).is_err());
    }

    #[test]
    fn proof_digest_is_bound_before_signature_verification() {
        let secret = b"01234567890123456789012345678901";
        let mut request = request();
        request.signature = sign(secret, "action-1", &request);
        request.provider_proof = "changed".into();
        assert_eq!(
            verify(secret, "action-1", &request)
                .unwrap_err()
                .to_string(),
            "validation: invalid execution attestation"
        );
    }
}
