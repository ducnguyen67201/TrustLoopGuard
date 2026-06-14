"""submit_event tests: typed round trip + error mapping, sync and async."""

from __future__ import annotations

import httpx
import pytest
import respx

from trustloopguard import (
    AsyncClient,
    Client,
    Decision,
    GuardEvent,
    Internal,
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
@pytest.mark.asyncio
async def test_async_submit_event_round_trip() -> None:
    respx.post("https://api.example.test/v1/events").mock(
        return_value=httpx.Response(200, json=default_allow_decision())
    )

    async with AsyncClient("https://api.example.test") as client:
        decision = await client.submit_event(send_email_event())

    assert decision.verdict is Verdict.allow
    assert decision.reason == DEFAULT_EVENT_ALLOW_REASON
