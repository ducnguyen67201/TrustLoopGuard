from __future__ import annotations

import httpx
import respx

from trustloopguard import (
    AuthorizationClaim,
    AuthorizationEffect,
    AuthorizationIntentStatus,
    Client,
    FinancialActionKind,
    FinancialExecutionStatus,
    FinancialRail,
    MoneyAmount,
)


def action_body() -> dict[str, object]:
    return {
        "id": "action-1",
        "workspace_id": "workspace-1",
        "environment_id": "production",
        "authorization_intent_id": "intent-1",
        "authorization_receipt_id": "receipt-1",
        "authorization_effect": "permit",
        "authorization_status": "authorized",
        "execution_status": "not_started",
        "action": {
            "id": "action-1",
            "kind": "payment",
            "operation": "pay",
            "principal_id": "agent-1",
            "amount": {"amount_minor": 100, "currency": "USD"},
            "rail": "internal",
            "metadata": {},
        },
        "evidence": [],
        "created_at": "2026-07-14T00:00:00Z",
        "updated_at": "2026-07-14T00:00:00Z",
    }


@respx.mock
def test_financial_operation_uses_common_authorization_claim() -> None:
    respx.post("https://api.test/v1/financial/actions").mock(
        return_value=httpx.Response(201, json=action_body())
    )
    client = Client("https://api.test")
    operation = client.financial_operation(
        operation="pay",
        kind=FinancialActionKind.payment,
        principal_id="agent-1",
        rail=FinancialRail.internal,
        amount=lambda amount, _facts: MoneyAmount(amount_minor=amount, currency="USD"),
        idempotency_key=lambda _amount, _facts: "idem-1",
        authorization=lambda _amount, _facts: AuthorizationClaim(
            grant_id="grant-1", attempt_id="attempt-1"
        ),
    )

    request = operation.build_request(100)
    assert request.authorization is not None
    assert not hasattr(request.action, "mandate")
    result = operation.verify(100)
    assert result.authorization_effect is AuthorizationEffect.permit
    assert result.authorization_status is AuthorizationIntentStatus.authorized
    assert result.execution_status is FinancialExecutionStatus.not_started
