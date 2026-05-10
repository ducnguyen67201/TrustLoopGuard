"""Smoke tests for the TrustLoopGuard Python SDK. No live network calls."""

from __future__ import annotations

import httpx
import respx

from trustloopguard import (
    Channel,
    CheckRequest,
    Client,
    Decision,
    Verdict,
)


@respx.mock
def test_check_allow_round_trip() -> None:
    respx.post("https://api.example.test/v1/check").mock(
        return_value=httpx.Response(
            200,
            json={
                "trace_id": "trace-1",
                "verdict": "allow",
                "reason": "no policies triggered",
                "triggered_policies": [],
                "safe_output": None,
                "latency_ms": 1,
            },
        )
    )
    with Client("https://api.example.test", api_key="test") as client:
        decision: Decision = client.check(
            CheckRequest(
                agent_id="agent-a",
                channel=Channel.chat,
                input="hi",
                proposed_output="hello",
            )
        )
    assert decision.verdict is Verdict.allow
    assert decision.trace_id == "trace-1"


@respx.mock
def test_bearer_header_is_sent() -> None:
    route = respx.post("https://api.example.test/v1/check").mock(
        return_value=httpx.Response(
            200,
            json={
                "trace_id": "t",
                "verdict": "allow",
                "reason": "",
                "triggered_policies": [],
                "safe_output": None,
                "latency_ms": 0,
            },
        )
    )
    with Client("https://api.example.test", api_key="sk-abc") as client:
        client.check(
            CheckRequest(
                agent_id="agent-a",
                channel=Channel.voice,
                input="",
                proposed_output="",
            )
        )
    assert route.calls.last.request.headers["authorization"] == "Bearer sk-abc"
