"""submit_event tests: typed round trip + error mapping, sync and async."""

from __future__ import annotations

import httpx
import pytest
import respx

from trustloopguard import (
    AsyncClient,
    Client,
    CreateRunEventRequest,
    Decision,
    GuardEvent,
    Internal,
    RunEventKind,
    RunKind,
    RunStatus,
    Verdict,
)
from trustloopguard.retry import RetryConfig

DEFAULT_EVENT_ALLOW_REASON = "event allowed: no enforced checker or enabled policy matched"


def send_email_event() -> GuardEvent:
    return GuardEvent.model_validate(
        {
            "kind": "tool.call.proposed",
            "principal": {
                "workspace_id": "ws_1",
                "environment_id": "production",
                "agent_id": "agent-1",
            },
            "action": {
                "operation": "send_email",
                "parameters": {"recipient": "a@b.c", "body": "hi"},
            },
            "sources": [
                {"id": "src.user", "origin": "user", "labels": {}},
                {"id": "src.web", "origin": "web", "labels": {}, "kind": "web_page"},
            ],
            "provenance": {
                "recipient": ["src.web"],
                "body": ["src.user", "src.web"],
            },
        }
    )


def default_allow_decision() -> dict:
    return {
        "trace_id": "t-1",
        "verdict": "allow",
        "reason": DEFAULT_EVENT_ALLOW_REASON,
        "triggered_policies": [],
        "safe_output": None,
        "latency_ms": 2,
    }


def run_summary() -> dict:
    return {
        "id": "018f1111-1111-7111-8111-111111111111",
        "workspace_id": "ws_1",
        "environment_id": "production",
        "environment": "production",
        "agent_id": "agent-1",
        "kind": "chat_session",
        "status": "running",
        "external_id": None,
        "metadata": {},
        "started_at": "2026-01-01T00:00:00Z",
        "ended_at": None,
        "created_at": "2026-01-01T00:00:00Z",
        "updated_at": "2026-01-01T00:00:00Z",
        "trace_count": 0,
        "blocked_count": 0,
        "rewritten_count": 0,
        "escalated_count": 0,
        "p95_latency_ms": None,
    }


def run_event_summary() -> dict:
    return {
        "id": "018f2222-2222-7222-8222-222222222222",
        "workspace_id": "ws_1",
        "run_id": "018f1111-1111-7111-8111-111111111111",
        "sequence": 1,
        "kind": "user_turn",
        "label": None,
        "input_summary": None,
        "output_summary": None,
        "metadata": {},
        "occurred_at": "2026-01-01T00:00:00Z",
        "created_at": "2026-01-01T00:00:00Z",
    }


@respx.mock
def test_submit_event_round_trip() -> None:
    route = respx.post("https://api.example.test/v1/events").mock(
        return_value=httpx.Response(200, json=default_allow_decision())
    )

    with Client("https://api.example.test", api_key="secret") as client:
        decision: Decision = client.submit_event(send_email_event())

    assert decision.verdict is Verdict.allow
    assert decision.reason == DEFAULT_EVENT_ALLOW_REASON

    request = route.calls.last.request
    assert request.headers["authorization"] == "Bearer secret"
    import json

    body = json.loads(request.content)
    assert body["action"]["operation"] == "send_email"
    assert body["provenance"]["recipient"] == ["src.web"]


@respx.mock
def test_submit_event_maps_server_error() -> None:
    respx.post("https://api.example.test/v1/events").mock(
        return_value=httpx.Response(
            500, json={"code": "internal", "message": "boom", "retriable": False}
        )
    )

    with Client(
        "https://api.example.test", retry=RetryConfig(max_attempts=1)
    ) as client:
        with pytest.raises(Internal):
            client.submit_event(send_email_event())


@respx.mock
def test_run_context_inherits_run_and_event_ids() -> None:
    respx.post("https://api.example.test/v1/runs").mock(
        return_value=httpx.Response(200, json=run_summary())
    )
    respx.patch(
        "https://api.example.test/v1/runs/018f1111-1111-7111-8111-111111111111"
    ).mock(return_value=httpx.Response(200, json={**run_summary(), "status": "completed"}))
    respx.post(
        "https://api.example.test/v1/runs/018f1111-1111-7111-8111-111111111111/events"
    ).mock(return_value=httpx.Response(200, json=run_event_summary()))
    event_route = respx.post("https://api.example.test/v1/events").mock(
        return_value=httpx.Response(200, json=default_allow_decision())
    )

    with Client("https://api.example.test") as client:
        with client.run(agent_id="agent-1", kind=RunKind.chat_session) as run:
            client.submit_event(send_email_event())
            with run.event(CreateRunEventRequest(kind=RunEventKind.user_turn)):
                client.submit_event(send_email_event())

    first = event_route.calls[0].request
    second = event_route.calls[1].request
    import json

    first_body = json.loads(first.content)
    second_body = json.loads(second.content)
    assert first_body["principal"]["run_id"] == "018f1111-1111-7111-8111-111111111111"
    assert "run_event_id" not in first_body["principal"]
    assert second_body["principal"]["run_event_id"] == "018f2222-2222-7222-8222-222222222222"


@respx.mock
def test_explicit_run_id_wins_and_tool_helper_inherits_context() -> None:
    respx.post("https://api.example.test/v1/runs").mock(
        return_value=httpx.Response(200, json=run_summary())
    )
    respx.patch(
        "https://api.example.test/v1/runs/018f1111-1111-7111-8111-111111111111"
    ).mock(return_value=httpx.Response(200, json={**run_summary(), "status": "completed"}))
    respx.post(
        "https://api.example.test/v1/runs/018f1111-1111-7111-8111-111111111111/events"
    ).mock(return_value=httpx.Response(200, json=run_event_summary()))
    event_route = respx.post("https://api.example.test/v1/events").mock(
        return_value=httpx.Response(200, json=default_allow_decision())
    )
    explicit = send_email_event()
    explicit.principal.run_id = "explicit-run"

    with Client("https://api.example.test") as client:
        with client.run(agent_id="agent-1", kind=RunKind.chat_session) as run:
            with run.event(CreateRunEventRequest(kind=RunEventKind.user_turn)):
                client.submit_event(explicit)
            client.guard_tool_call(
                agent_id="agent-1",
                operation="issue_refund",
                parameters={"orderId": "o_1"},
                side_effect="api_mutation",
                sources=[{"id": "input", "origin": "user", "labels": {}}],
                provenance={"orderId": ["input"]},
            )

    import json

    first = json.loads(event_route.calls[0].request.content)
    second = json.loads(event_route.calls[1].request.content)
    assert first["principal"]["run_id"] == "explicit-run"
    assert "run_event_id" not in first["principal"]
    assert second["kind"] == "tool.call.proposed"
    assert second["principal"]["run_id"] == "018f1111-1111-7111-8111-111111111111"


@respx.mock
@pytest.mark.asyncio
async def test_async_run_context_inherits_run_id() -> None:
    respx.post("https://api.example.test/v1/runs").mock(
        return_value=httpx.Response(200, json=run_summary())
    )
    respx.patch(
        "https://api.example.test/v1/runs/018f1111-1111-7111-8111-111111111111"
    ).mock(return_value=httpx.Response(200, json={**run_summary(), "status": "completed"}))
    event_route = respx.post("https://api.example.test/v1/events").mock(
        return_value=httpx.Response(200, json=default_allow_decision())
    )

    async with AsyncClient("https://api.example.test") as client:
        async with client.run(agent_id="agent-1", kind=RunKind.chat_session):
            await client.submit_event(send_email_event())

    import json

    body = json.loads(event_route.calls[0].request.content)
    assert body["principal"]["run_id"] == "018f1111-1111-7111-8111-111111111111"


@respx.mock
@pytest.mark.asyncio
async def test_async_submit_event_round_trip() -> None:
    respx.post("https://api.example.test/v1/events").mock(
        return_value=httpx.Response(200, json=default_allow_decision())
    )

    async with AsyncClient("https://api.example.test") as client:
        decision = await client.submit_event(send_email_event())

    assert decision.verdict is Verdict.allow
    assert decision.reason == DEFAULT_EVENT_ALLOW_REASON
