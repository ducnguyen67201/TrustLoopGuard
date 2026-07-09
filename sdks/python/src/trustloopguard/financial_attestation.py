"""Canonical signing helpers for external financial execution attestations."""

from __future__ import annotations

import base64
import hashlib
import hmac
from typing import Mapping

PREFIX = "tlg-financial-execution-attestation.v1"


def financial_execution_attestation_message(
    action_id: str, request: Mapping[str, str]
) -> bytes:
    fields = [
        request["connector_id"],
        action_id,
        request["grant_id"],
        request["action_hash"],
        request["provider"],
        request["provider_reference"],
        request["provider_status"],
        request["executed_at"],
        request["idempotency_key"],
        request["provider_proof_sha256"],
    ]
    lines = [PREFIX, *(f"{len(value.encode('utf-8'))}:{value}" for value in fields)]
    return "\n".join(lines).encode("utf-8")


def financial_provider_proof_sha256(proof: str) -> str:
    return f"sha256:{hashlib.sha256(proof.encode('utf-8')).hexdigest()}"


def sign_financial_execution_attestation(
    plaintext_secret: str, action_id: str, request: Mapping[str, str]
) -> str:
    secret = _decode_base64url(plaintext_secret)
    signature = hmac.new(
        secret,
        financial_execution_attestation_message(action_id, request),
        hashlib.sha256,
    ).digest()
    return f"v1={base64.urlsafe_b64encode(signature).decode('ascii').rstrip('=')}"


def _decode_base64url(value: str) -> bytes:
    return base64.urlsafe_b64decode(value + "=" * (-len(value) % 4))
