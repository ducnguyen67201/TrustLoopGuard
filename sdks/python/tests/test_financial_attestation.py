from trustloopguard.financial_attestation import (
    financial_execution_attestation_message,
    sign_financial_execution_attestation,
)


REQUEST = {
    "connector_id": "connector-1",
    "grant_id": "grant-1",
    "action_hash": "sha256:action",
    "provider": "stripe",
    "provider_reference": "pi_123",
    "provider_status": "succeeded",
    "executed_at": "2026-07-09T00:00:00Z",
    "idempotency_key": "commit-1",
    "provider_proof": "provider receipt",
    "provider_proof_sha256": "sha256:proof",
}


def test_financial_attestation_matches_shared_vector() -> None:
    message = financial_execution_attestation_message("action-1", REQUEST)
    assert b"\n11:connector-1\n8:action-1\n7:grant-1" in message
    assert (
        sign_financial_execution_attestation(
            "MDEyMzQ1Njc4OTAxMjM0NTY3ODkwMTIzNDU2Nzg5MDE",
            "action-1",
            REQUEST,
        )
        == "v1=FbjzlmAsFdGVBB5yKbLD6UZ6-CgtIXZCtByHv49nXpY"
    )


def test_financial_attestation_binds_provider_reference() -> None:
    secret = "MDEyMzQ1Njc4OTAxMjM0NTY3ODkwMTIzNDU2Nzg5MDE"
    original = sign_financial_execution_attestation(secret, "action-1", REQUEST)
    changed = sign_financial_execution_attestation(
        secret, "action-1", {**REQUEST, "provider_reference": "pi_changed"}
    )
    assert changed != original
