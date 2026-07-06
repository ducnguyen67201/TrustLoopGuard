from __future__ import annotations

import json

import httpx
import respx

from trustloopguard import (
    Client,
    CounterpartyRef,
    CreateFinancialActionRequest,
    FinancialAction,
    FinancialActionListResponse,
    FinancialActionKind,
    FinancialActionRecord,
    FinancialActionStatus,
    FinancialRail,
    MoneyAmount,
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
