"""Tests for the ``guard()`` helper. Sync + async, all branches.

Mirrors the TypeScript test suite — same scenarios, same assertions
where the language allows."""

from __future__ import annotations

from typing import Any

import httpx
import pytest
import respx

from trustloopguard import (
    AsyncClient,
    Channel,
    Client,
    Decision,
    GuardLogEvent,
    RetryConfig,
    SdkError,
    Transport,
    Verdict,
    guard,
    guard_async,
)


def _decision_payload(**overrides: Any) -> dict[str, Any]:
    base = {
        "trace_id": "t-1",
        "verdict": "allow",
        "reason": "ok",
        "triggered_policies": [],
        "safe_output": None,
        "latency_ms": 1,
        "tier_results": [],
    }
    base.update(overrides)
    return base


# -- Sync ------------------------------------------------------------------


@respx.mock
def test_guard_returns_draft_on_allow_by_default() -> None:
    respx.post("https://t.test/v1/check").mock(
        return_value=httpx.Response(200, json=_decision_payload(verdict="allow"))
    )
    with Client(base_url="https://t.test") as c:
        out = guard(
            client=c,
            agent_id="a",
            input="hi",
            draft="hello",
            on_block=lambda _: "BLOCKED",
            on_escalate=lambda _: "ESC",
        )
    assert out == "hello"


@respx.mock
def test_guard_returns_safe_output_on_rewrite() -> None:
    respx.post("https://t.test/v1/check").mock(
        return_value=httpx.Response(
            200,
            json=_decision_payload(verdict="rewrite", safe_output="please contact support"),
        )
    )
    with Client(base_url="https://t.test") as c:
        out = guard(
            client=c,
            agent_id="a",
            input="hi",
            draft="hello",
            on_block=lambda _: "BLOCKED",
            on_escalate=lambda _: "ESC",
        )
    assert out == "please contact support"


@respx.mock
def test_guard_falls_back_to_draft_on_rewrite_without_safe_output() -> None:
    respx.post("https://t.test/v1/check").mock(
        return_value=httpx.Response(
            200, json=_decision_payload(verdict="rewrite", safe_output=None)
        )
    )
    with Client(base_url="https://t.test") as c:
        out = guard(
            client=c,
            agent_id="a",
            input="hi",
            draft="hello",
            on_block=lambda _: "BLOCKED",
            on_escalate=lambda _: "ESC",
        )
    assert out == "hello"


@respx.mock
def test_guard_invokes_on_block_on_block_verdict() -> None:
    respx.post("https://t.test/v1/check").mock(
        return_value=httpx.Response(200, json=_decision_payload(verdict="block"))
    )
    seen: list[Decision] = []

    def on_block(d: Decision) -> str:
        seen.append(d)
        return "BLOCKED"

    with Client(base_url="https://t.test") as c:
        out = guard(
            client=c,
            agent_id="a",
            input="hi",
            draft="hello",
            on_block=on_block,
            on_escalate=lambda _: "ESC",
        )
    assert out == "BLOCKED"
    assert len(seen) == 1
    assert seen[0].verdict == Verdict.block


@respx.mock
def test_guard_invokes_on_escalate_on_escalate_verdict() -> None:
    respx.post("https://t.test/v1/check").mock(
        return_value=httpx.Response(200, json=_decision_payload(verdict="escalate"))
    )
    with Client(base_url="https://t.test") as c:
        out = guard(
            client=c,
            agent_id="a",
            input="hi",
            draft="hello",
            on_block=lambda _: "BLOCKED",
            on_escalate=lambda _: "ESCALATED",
        )
    assert out == "ESCALATED"


@respx.mock
def test_guard_passes_on_allow_when_supplied() -> None:
    respx.post("https://t.test/v1/check").mock(
        return_value=httpx.Response(200, json=_decision_payload(verdict="allow"))
    )
    with Client(base_url="https://t.test") as c:
        out = guard(
            client=c,
            agent_id="a",
            input="hi",
            draft="hello",
            on_allow=lambda draft, _d: f"[ok] {draft}",
            on_block=lambda _: "BLOCKED",
            on_escalate=lambda _: "ESC",
        )
    assert out == "[ok] hello"


@respx.mock
def test_guard_fails_open_on_transport_error_by_default() -> None:
    respx.post("https://t.test/v1/check").mock(
        side_effect=httpx.ConnectError("econnrefused")
    )
    with Client(
        base_url="https://t.test",
        retry=RetryConfig(max_attempts=1, base_delay_s=0, max_delay_s=0, total_budget_s=0),
    ) as c:
        out = guard(
            client=c,
            agent_id="a",
            input="hi",
            draft="ORIGINAL",
            on_block=lambda _: "BLOCKED",
            on_escalate=lambda _: "ESC",
        )
    assert out == "ORIGINAL"


@respx.mock
def test_guard_routes_errors_through_on_error_when_supplied() -> None:
    respx.post("https://t.test/v1/check").mock(
        side_effect=httpx.ConnectError("econnrefused")
    )
    seen: list[SdkError] = []

    def on_error(err: SdkError, draft: str) -> str:
        seen.append(err)
        return "FAIL_CLOSED"

    with Client(
        base_url="https://t.test",
        retry=RetryConfig(max_attempts=1, base_delay_s=0, max_delay_s=0, total_budget_s=0),
    ) as c:
        out = guard(
            client=c,
            agent_id="a",
            input="hi",
            draft="hello",
            on_block=lambda _: "BLOCKED",
            on_escalate=lambda _: "ESC",
            on_error=on_error,
        )
    assert out == "FAIL_CLOSED"
    assert isinstance(seen[0], Transport)


@respx.mock
def test_guard_emits_log_event_with_chosen_branch() -> None:
    respx.post("https://t.test/v1/check").mock(
        return_value=httpx.Response(
            200, json=_decision_payload(verdict="block", trace_id="trace-abc")
        )
    )
    events: list[GuardLogEvent] = []
    with Client(base_url="https://t.test") as c:
        guard(
            client=c,
            agent_id="a",
            input="hi",
            draft="hello",
            on_block=lambda _: "BLOCKED",
            on_escalate=lambda _: "ESC",
            log=events.append,
        )
    assert len(events) == 1
    assert events[0].trace_id == "trace-abc"
    assert events[0].verdict == "block"
    assert events[0].branch == "block"


@respx.mock
def test_guard_logs_branch_error_on_transport_failure() -> None:
    respx.post("https://t.test/v1/check").mock(
        side_effect=httpx.ConnectError("econnrefused")
    )
    events: list[GuardLogEvent] = []
    with Client(
        base_url="https://t.test",
        retry=RetryConfig(max_attempts=1, base_delay_s=0, max_delay_s=0, total_budget_s=0),
    ) as c:
        guard(
            client=c,
            agent_id="a",
            input="hi",
            draft="hello",
            on_block=lambda _: "BLOCKED",
            on_escalate=lambda _: "ESC",
            log=events.append,
        )
    assert len(events) == 1
    assert events[0].branch == "error"


@respx.mock
def test_guard_builds_correct_wire_request() -> None:
    route = respx.post("https://t.test/v1/check").mock(
        return_value=httpx.Response(200, json=_decision_payload(verdict="allow"))
    )
    with Client(base_url="https://t.test") as c:
        guard(
            client=c,
            agent_id="acme",
            input="hi",
            draft="hello",
            channel=Channel.voice,
            domain="voice_agent",
            context={"docs": ["kb-1"]},
            trace_id="caller-trace-1",
            on_block=lambda _: "BLOCKED",
            on_escalate=lambda _: "ESC",
        )
    assert route.called
    req_body = route.calls.last.request.content
    import json

    body = json.loads(req_body)
    assert body["agent_id"] == "acme"
    assert body["channel"] == "voice"
    assert body["domain"] == "voice_agent"
    assert body["proposed_output"] == "hello"
    assert body["trace_id"] == "caller-trace-1"
    assert body["context"]["docs"] == ["kb-1"]


# -- Async -----------------------------------------------------------------


@pytest.mark.asyncio
@respx.mock
async def test_guard_async_allow() -> None:
    respx.post("https://t.test/v1/check").mock(
        return_value=httpx.Response(200, json=_decision_payload(verdict="allow"))
    )

    async def on_block(_d: Decision) -> str:
        return "BLOCKED"

    async def on_escalate(_d: Decision) -> str:
        return "ESC"

    async with AsyncClient(base_url="https://t.test") as c:
        out = await guard_async(
            client=c,
            agent_id="a",
            input="hi",
            draft="hello",
            on_block=on_block,
            on_escalate=on_escalate,
        )
    assert out == "hello"


@pytest.mark.asyncio
@respx.mock
async def test_guard_async_block_runs_callback() -> None:
    respx.post("https://t.test/v1/check").mock(
        return_value=httpx.Response(200, json=_decision_payload(verdict="block"))
    )

    seen: list[Decision] = []

    async def on_block(d: Decision) -> str:
        seen.append(d)
        return "BLOCKED"

    async def on_escalate(_d: Decision) -> str:
        return "ESC"

    async with AsyncClient(base_url="https://t.test") as c:
        out = await guard_async(
            client=c,
            agent_id="a",
            input="hi",
            draft="hello",
            on_block=on_block,
            on_escalate=on_escalate,
        )
    assert out == "BLOCKED"
    assert len(seen) == 1


@pytest.mark.asyncio
@respx.mock
async def test_guard_async_fails_open_by_default() -> None:
    respx.post("https://t.test/v1/check").mock(
        side_effect=httpx.ConnectError("econnrefused")
    )

    async def on_block(_d: Decision) -> str:
        return "BLOCKED"

    async def on_escalate(_d: Decision) -> str:
        return "ESC"

    async with AsyncClient(
        base_url="https://t.test",
        retry=RetryConfig(max_attempts=1, base_delay_s=0, max_delay_s=0, total_budget_s=0),
    ) as c:
        out = await guard_async(
            client=c,
            agent_id="a",
            input="hi",
            draft="ORIGINAL",
            on_block=on_block,
            on_escalate=on_escalate,
        )
    assert out == "ORIGINAL"
