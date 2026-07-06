from __future__ import annotations

import json

import httpx
import respx

from trustloopguard import (
    Client,
    CounterpartyRef,
    CreateFinancialActionRequest,
    CreateFinancialMandateRequest,
    FinancialAction,
    FinancialActionListResponse,
    FinancialActionKind,
    FinancialActionOutcome,
    FinancialActionOutcomeStatus,
    FinancialMandateListResponse,
    FinancialMandateStatus,
    FinancialOutcomeListResponse,
    FinancialActionRecord,
    FinancialActionStatus,
    FinancialReceipt,
    FinancialRail,
    MoneyAmount,
    RecoveryStatus,
    ReversalCapability,
)


def request() -> CreateFinancialActionRequest:
    return CreateFinancialActionRequest(
        idempotency_key="idem-refund-75",
        execute=False,
        action=FinancialAction(
            kind=FinancialActionKind.refund,
            principal_id="refund-bot",
            amount=MoneyAmount(amount_minor=7500, currency="USD"),
            counterparty=CounterpartyRef(
                id="cust_456",
                display_name="Casey Customer",
                kind="customer",
                country="US",
                metadata={},
            ),
            rail=FinancialRail.card,
            memo="refund damaged item",
            metadata={"order_id": "order_123"},
        ),
        evidence=[],
    )


def action_body(status: str = "proposed") -> dict[str, object]:
    return {
        "id": "018f3333-3333-7333-8333-333333333333",
        "workspace_id": "ws_finance",
        "status": status,
        "action": {
            **request().action.model_dump(mode="json", exclude_none=True),
            "id": "018f3333-3333-7333-8333-333333333333",
        },
        "evidence": [],
        "created_at": "2026-07-05T00:00:00Z",
        "updated_at": "2026-07-05T00:00:00Z",
    }

def mandate_request() -> CreateFinancialMandateRequest:
    return CreateFinancialMandateRequest(
        id="mandate_refund_bot",
        version=1,
        principal_id="refund-bot",
        scope={
            "action_kinds": ["refund"],
            "max_amount_minor": 10000,
            "currency": "USD",
        },
        metadata={"source": "python_sdk_test"},
        expires_at="2026-08-05T19:00:00Z",
    )


def mandate_body(status: str = "active") -> dict[str, object]:
    return {
        "id": "mandate_refund_bot",
        "workspace_id": "ws_finance",
        "version": 1,
        "status": status,
        "principal_id": "refund-bot",
        "scope": mandate_request().scope,
        "metadata": mandate_request().metadata,
        "expires_at": "2026-08-05T19:00:00Z",
        "created_at": "2026-07-05T00:00:00Z",
        "updated_at": "2026-07-05T00:00:00Z",
    }


def receipt_body() -> dict[str, object]:
    return {
        "id": "018f3333-3333-7333-8333-333333333333",
        "action_id": "018f3333-3333-7333-8333-333333333333",
        "trace_id": "018f4444-4444-7444-8444-444444444444",
        "ledger_event_ids": ["ledger_execute_1"],
        "proof": {"action_status": "executed", "provider_reference": "refund_123"},
        "created_at": "2026-07-05T00:00:00Z",
    }


def outcome() -> FinancialActionOutcome:
    return FinancialActionOutcome(
        action_id="018f3333-3333-7333-8333-333333333333",
        status=FinancialActionOutcomeStatus.succeeded,
        reversal_capability=ReversalCapability.manual_recovery,
        recovery_status=RecoveryStatus.manual_required,
        provider_status="provider_status",
        provider_reference="provider_ref_123",
        occurred_at="2026-07-05T20:00:00Z",
        metadata={"source": "python_sdk_test"},
    )


def outcome_body() -> dict[str, object]:
    return outcome().model_dump(mode="json", exclude_none=True)


@respx.mock
def test_verify_action_and_guard_payment_post_financial_action() -> None:
    route = respx.post("https://api.example.test/v1/financial/actions").mock(
        return_value=httpx.Response(201, json=action_body())
    )

    with Client("https://api.example.test", api_key="test") as client:
        action: FinancialActionRecord = client.verify_action(request())
        payment_action = client.guard_payment(request())

    assert action.status is FinancialActionStatus.proposed
    assert payment_action.id == action.id
    assert route.call_count == 2
    assert json.loads(route.calls.last.request.content)["action"]["kind"] == "refund"


@respx.mock
def test_financial_action_get_and_transitions() -> None:
    action_id = "018f3333-3333-7333-8333-333333333333"
    get = respx.get(f"https://api.example.test/v1/financial/actions/{action_id}").mock(
        return_value=httpx.Response(200, json=action_body())
    )
    approve = respx.post(
        f"https://api.example.test/v1/financial/actions/{action_id}/approve"
    ).mock(return_value=httpx.Response(200, json=action_body("authorized")))
    execute = respx.post(
        f"https://api.example.test/v1/financial/actions/{action_id}/execute"
    ).mock(return_value=httpx.Response(200, json=action_body("executed")))

    with Client("https://api.example.test", api_key="test") as client:
        assert client.get_financial_action(action_id).id == action_id
        assert client.approve_action(action_id).status is FinancialActionStatus.authorized
        assert client.execute_action(action_id).status is FinancialActionStatus.executed

    assert get.called
    assert approve.called
    assert execute.called


@respx.mock
def test_financial_actions_list() -> None:
    route = respx.get("https://api.example.test/v1/financial/actions").mock(
        return_value=httpx.Response(200, json={"actions": [action_body()]})
    )

    with Client("https://api.example.test", api_key="test") as client:
        response: FinancialActionListResponse = client.list_financial_actions()

    assert len(response.actions) == 1
    assert response.actions[0].status is FinancialActionStatus.proposed
    assert route.called


@respx.mock
def test_financial_mandates_create_list_and_revoke() -> None:
    create = respx.post("https://api.example.test/v1/financial/mandates").mock(
        return_value=httpx.Response(201, json=mandate_body())
    )
    list_route = respx.get("https://api.example.test/v1/financial/mandates").mock(
        return_value=httpx.Response(200, json={"mandates": [mandate_body()]})
    )
    revoke = respx.post(
        "https://api.example.test/v1/financial/mandates/mandate_refund_bot/revoke"
    ).mock(return_value=httpx.Response(200, json=mandate_body("revoked")))

    with Client("https://api.example.test", api_key="test") as client:
        mandate = client.create_mandate(mandate_request())
        mandates: FinancialMandateListResponse = client.list_mandates()
        revoked = client.revoke_mandate("mandate_refund_bot")

    assert mandate.status is FinancialMandateStatus.active
    assert len(mandates.mandates) == 1
    assert revoked.status is FinancialMandateStatus.revoked
    assert create.called
    assert list_route.called
    assert revoke.called


@respx.mock
def test_financial_receipt_get() -> None:
    action_id = "018f3333-3333-7333-8333-333333333333"
    route = respx.get(f"https://api.example.test/v1/financial/receipts/{action_id}").mock(
        return_value=httpx.Response(200, json=receipt_body())
    )

    with Client("https://api.example.test", api_key="test") as client:
        receipt: FinancialReceipt = client.get_receipt(action_id)

    assert receipt.id == action_id
    assert receipt.action_id == action_id
    assert receipt.proof["action_status"] == "executed"
    assert route.called


@respx.mock
def test_financial_outcomes_record_and_list() -> None:
    action_id = "018f3333-3333-7333-8333-333333333333"
    record = respx.post(
        f"https://api.example.test/v1/financial/actions/{action_id}/outcomes"
    ).mock(return_value=httpx.Response(201, json=outcome_body()))
    list_route = respx.get(
        f"https://api.example.test/v1/financial/actions/{action_id}/outcomes"
    ).mock(return_value=httpx.Response(200, json={"outcomes": [outcome_body()]}))

    with Client("https://api.example.test", api_key="test") as client:
        recorded = client.record_action_outcome(action_id, outcome())
        outcomes: FinancialOutcomeListResponse = client.list_action_outcomes(action_id)

    assert recorded.status is FinancialActionOutcomeStatus.succeeded
    assert len(outcomes.outcomes) == 1
    assert outcomes.outcomes[0].provider_reference == "provider_ref_123"
    assert record.called
    assert list_route.called
