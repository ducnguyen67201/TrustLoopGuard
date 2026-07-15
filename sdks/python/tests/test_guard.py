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
    AuthorizationDecision,
    GuardLogEvent,
    GuardMode,
    RegenerateFeedback,
    RetryConfig,
    SdkError,
    Transport,
    AuthorizationEffect,
    guard,
    guard_async,
)


def _decision_payload(**overrides: Any) -> dict[str, Any]:
    base = {
        "trace_id": "t-1",
        "domain": "content",
        "effect": "permit",
        "reason": "ok",
        "findings": [],
        "transformed_value": None,
        "latency_ms": 1,
    }
    base.update(overrides)
    return base


# -- Sync ------------------------------------------------------------------


@respx.mock
def test_guard_returns_draft_on_allow_by_default() -> None:
    respx.post("https://t.test/v1/events").mock(
        return_value=httpx.Response(200, json=_decision_payload(effect="permit"))
    )
    with Client(base_url="https://t.test") as c:
        out = guard(
            client=c,
            agent_id="a",
            input="hi",
            draft="hello",
            on_block=lambda _: "BLOCKED",
            on_require_approval=lambda _: "ESC",
        )
    assert out == "hello"


@respx.mock
def test_guard_returns_transformed_value_on_rewrite() -> None:
    respx.post("https://t.test/v1/events").mock(
        return_value=httpx.Response(
            200,
            json=_decision_payload(effect="transform", transformed_value="please contact support"),
        )
    )
    with Client(base_url="https://t.test") as c:
        out = guard(
            client=c,
            agent_id="a",
            input="hi",
            draft="hello",
            on_block=lambda _: "BLOCKED",
            on_require_approval=lambda _: "ESC",
        )
    assert out == "please contact support"


@respx.mock
def test_guard_falls_back_to_draft_on_rewrite_without_transformed_value() -> None:
    respx.post("https://t.test/v1/events").mock(
        return_value=httpx.Response(
            200, json=_decision_payload(effect="transform", transformed_value=None)
        )
    )
    with Client(base_url="https://t.test") as c:
        out = guard(
            client=c,
            agent_id="a",
            input="hi",
            draft="hello",
            on_block=lambda _: "BLOCKED",
            on_require_approval=lambda _: "ESC",
        )
    assert out == "hello"


@respx.mock
def test_guard_invokes_on_block_on_block_verdict() -> None:
    respx.post("https://t.test/v1/events").mock(
        return_value=httpx.Response(200, json=_decision_payload(effect="deny"))
    )
    seen: list[AuthorizationDecision] = []

    def on_block(d: AuthorizationDecision) -> str:
        seen.append(d)
        return "BLOCKED"

    with Client(base_url="https://t.test") as c:
        out = guard(
            client=c,
            agent_id="a",
            input="hi",
            draft="hello",
            on_block=on_block,
            on_require_approval=lambda _: "ESC",
        )
    assert out == "BLOCKED"
    assert len(seen) == 1
    assert seen[0].effect == AuthorizationEffect.deny


@respx.mock
def test_guard_invokes_on_require_approval_on_require_approval_verdict() -> None:
    respx.post("https://t.test/v1/events").mock(
        return_value=httpx.Response(200, json=_decision_payload(effect="require_approval"))
    )
    with Client(base_url="https://t.test") as c:
        out = guard(
            client=c,
            agent_id="a",
            input="hi",
            draft="hello",
            on_block=lambda _: "BLOCKED",
            on_require_approval=lambda _: "ESCALATED",
        )
    assert out == "ESCALATED"


@respx.mock
def test_guard_passes_on_allow_when_supplied() -> None:
    respx.post("https://t.test/v1/events").mock(
        return_value=httpx.Response(200, json=_decision_payload(effect="permit"))
    )
    with Client(base_url="https://t.test") as c:
        out = guard(
            client=c,
            agent_id="a",
            input="hi",
            draft="hello",
            on_allow=lambda draft, _d: f"[ok] {draft}",
            on_block=lambda _: "BLOCKED",
            on_require_approval=lambda _: "ESC",
        )
    assert out == "[ok] hello"


@respx.mock
def test_guard_fails_open_on_transport_error_by_default() -> None:
    respx.post("https://t.test/v1/events").mock(
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
            on_require_approval=lambda _: "ESC",
        )
    assert out == "ORIGINAL"


@respx.mock
def test_guard_routes_errors_through_on_error_when_supplied() -> None:
    respx.post("https://t.test/v1/events").mock(
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
            on_require_approval=lambda _: "ESC",
            on_error=on_error,
        )
    assert out == "FAIL_CLOSED"
    assert isinstance(seen[0], Transport)


@respx.mock
def test_guard_emits_log_event_with_chosen_branch() -> None:
    respx.post("https://t.test/v1/events").mock(
        return_value=httpx.Response(
            200, json=_decision_payload(effect="deny", trace_id="trace-abc")
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
            on_require_approval=lambda _: "ESC",
            log=events.append,
        )
    assert len(events) == 1
    assert events[0].trace_id == "trace-abc"
    assert events[0].effect == "deny"
    assert events[0].branch == "deny"


@respx.mock
def test_guard_logs_branch_error_on_transport_failure() -> None:
    respx.post("https://t.test/v1/events").mock(
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
            on_require_approval=lambda _: "ESC",
            log=events.append,
        )
    assert len(events) == 1
    assert events[0].branch == "error"


@respx.mock
def test_guard_builds_correct_wire_request() -> None:
    route = respx.post("https://t.test/v1/events").mock(
        return_value=httpx.Response(200, json=_decision_payload(effect="permit"))
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
            on_require_approval=lambda _: "ESC",
        )
    assert route.called
    req_body = route.calls.last.request.content
    import json

    body = json.loads(req_body)
    assert body["kind"] == "output.proposed"
    assert body["principal"]["agent_id"] == "acme"
    assert body["action"]["operation"] == "output"
    assert body["action"]["parameters"]["text"] == "hello"
    assert body["context"]["channel"] == "voice"
    assert body["context"]["domain"] == "voice_agent"
    assert body["context"]["docs"] == ["kb-1"]


# -- Async -----------------------------------------------------------------


@pytest.mark.asyncio
@respx.mock
async def test_guard_async_allow() -> None:
    respx.post("https://t.test/v1/events").mock(
        return_value=httpx.Response(200, json=_decision_payload(effect="permit"))
    )

    async def on_block(_d: AuthorizationDecision) -> str:
        return "BLOCKED"

    async def on_require_approval(_d: AuthorizationDecision) -> str:
        return "ESC"

    async with AsyncClient(base_url="https://t.test") as c:
        out = await guard_async(
            client=c,
            agent_id="a",
            input="hi",
            draft="hello",
            on_block=on_block,
            on_require_approval=on_require_approval,
        )
    assert out == "hello"


@pytest.mark.asyncio
@respx.mock
async def test_guard_async_block_runs_callback() -> None:
    respx.post("https://t.test/v1/events").mock(
        return_value=httpx.Response(200, json=_decision_payload(effect="deny"))
    )

    seen: list[AuthorizationDecision] = []

    async def on_block(d: AuthorizationDecision) -> str:
        seen.append(d)
        return "BLOCKED"

    async def on_require_approval(_d: AuthorizationDecision) -> str:
        return "ESC"

    async with AsyncClient(base_url="https://t.test") as c:
        out = await guard_async(
            client=c,
            agent_id="a",
            input="hi",
            draft="hello",
            on_block=on_block,
            on_require_approval=on_require_approval,
        )
    assert out == "BLOCKED"
    assert len(seen) == 1


@pytest.mark.asyncio
@respx.mock
async def test_guard_async_fails_open_by_default() -> None:
    respx.post("https://t.test/v1/events").mock(
        side_effect=httpx.ConnectError("econnrefused")
    )

    async def on_block(_d: AuthorizationDecision) -> str:
        return "BLOCKED"

    async def on_require_approval(_d: AuthorizationDecision) -> str:
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
            on_require_approval=on_require_approval,
        )
    assert out == "ORIGINAL"


# -- Simple async factory ---------------------------------------------------


@pytest.mark.asyncio
@respx.mock
async def test_guard_factory_returns_async_callable_that_allows() -> None:
    route = respx.post("https://t.test/v1/events").mock(
        return_value=httpx.Response(200, json=_decision_payload(effect="permit"))
    )

    guardrail = guard(agent_id="factory-agent", base_url="https://t.test")
    out = await guardrail(input="hello", draft="safe reply")
    await guardrail.aclose()

    assert out == "safe reply"
    assert route.called
    body = route.calls.last.request.content
    import json

    assert json.loads(body)["principal"]["agent_id"] == "factory-agent"


@pytest.mark.asyncio
@respx.mock
async def test_guard_stream_buffers_chunks_then_guards() -> None:
    route = respx.post("https://t.test/v1/events").mock(
        return_value=httpx.Response(200, json=_decision_payload(effect="permit"))
    )

    guardrail = guard(agent_id="factory-agent", base_url="https://t.test")

    async def chunks() -> Any:
        for piece in ("Our hours ", "are 9 ", "to 5."):
            yield piece

    out = await guardrail.stream(input="when open?", draft=chunks())
    await guardrail.aclose()

    assert out == "Our hours are 9 to 5."
    import json

    assert (
        json.loads(route.calls.last.request.content)["action"]["parameters"]["text"]
        == "Our hours are 9 to 5."
    )


@pytest.mark.asyncio
@respx.mock
async def test_guard_stream_blocks_buffered_output() -> None:
    respx.post("https://t.test/v1/events").mock(
        return_value=httpx.Response(200, json=_decision_payload(effect="deny"))
    )

    guardrail = guard(agent_id="factory-agent", base_url="https://t.test")
    # Sync iterables are accepted too.
    out = await guardrail.stream(input="tell me", draft=["leak ", "the secret"])
    await guardrail.aclose()

    assert out == "I can't help with that request."


@pytest.mark.asyncio
@respx.mock
async def test_guard_factory_uses_default_block_reply() -> None:
    respx.post("https://t.test/v1/events").mock(
        return_value=httpx.Response(200, json=_decision_payload(effect="deny"))
    )

    guardrail = guard(agent_id="factory-agent", base_url="https://t.test")
    out = await guardrail(input="hello", draft="unsafe reply")
    await guardrail.aclose()

    assert out == "I can't help with that request."


@pytest.mark.asyncio
@respx.mock
async def test_guard_factory_accepts_string_branch_overrides() -> None:
    respx.post("https://t.test/v1/events").mock(
        return_value=httpx.Response(200, json=_decision_payload(effect="require_approval"))
    )

    guardrail = guard(
        agent_id="factory-agent",
        base_url="https://t.test",
        on_require_approval="A human should review this.",
    )
    out = await guardrail(input="hello", draft="needs review")
    await guardrail.aclose()

    assert out == "A human should review this."


@pytest.mark.asyncio
@respx.mock
async def test_guard_factory_keeps_defer_separate_from_approval() -> None:
    respx.post("https://t.test/v1/events").mock(
        return_value=httpx.Response(200, json=_decision_payload(effect="defer"))
    )
    guardrail = guard(
        agent_id="factory-agent",
        base_url="https://t.test",
        on_require_approval="REVIEW",
        on_defer="RETRY_LATER",
    )
    out = await guardrail(input="hello", draft="missing evidence")
    await guardrail.aclose()

    assert out == "RETRY_LATER"


@pytest.mark.asyncio
@respx.mock
async def test_guard_factory_strict_mode_blocks_rewrite() -> None:
    respx.post("https://t.test/v1/events").mock(
        return_value=httpx.Response(
            200,
            json=_decision_payload(effect="transform", transformed_value="sanitized reply"),
        )
    )

    guardrail = guard(agent_id="factory-agent", base_url="https://t.test", mode=GuardMode.STRICT)
    out = await guardrail(input="hello", draft="unsafe reply")
    await guardrail.aclose()

    assert out == "I can't help with that request."


@pytest.mark.asyncio
@respx.mock
async def test_guard_factory_rewrite_mode_blocks_rewrite_without_transformed_value() -> None:
    respx.post("https://t.test/v1/events").mock(
        return_value=httpx.Response(
            200,
            json=_decision_payload(effect="transform", transformed_value=None),
        )
    )

    guardrail = guard(agent_id="factory-agent", base_url="https://t.test", mode=GuardMode.REWRITE)
    out = await guardrail(input="hello", draft="unsafe reply")
    await guardrail.aclose()

    assert out == "I can't help with that request."


@pytest.mark.asyncio
@respx.mock
async def test_guard_factory_regenerate_mode_prefers_transformed_value() -> None:
    respx.post("https://t.test/v1/events").mock(
        return_value=httpx.Response(
            200,
            json=_decision_payload(effect="transform", transformed_value="sanitized reply"),
        )
    )

    seen: list[RegenerateFeedback] = []

    async def regenerate(feedback: RegenerateFeedback) -> str:
        seen.append(feedback)
        return "regenerated reply"

    guardrail = guard(
        agent_id="factory-agent",
        base_url="https://t.test",
        mode=GuardMode.REWRITE_OR_REGENERATE,
        regenerate=regenerate,
    )
    out = await guardrail(input="hello", draft="unsafe reply")
    await guardrail.aclose()

    assert out == "sanitized reply"
    assert seen == []


@pytest.mark.asyncio
@respx.mock
async def test_guard_factory_regenerates_and_checks_again() -> None:
    responses = iter(
        [
            httpx.Response(
                200,
                json=_decision_payload(
                    effect="transform",
                    transformed_value=None,
                    reason="contains confidential data",
                    trace_id="t-1",
                ),
            ),
            httpx.Response(
                200,
                json=_decision_payload(effect="permit", trace_id="t-2"),
            ),
        ]
    )
    route = respx.post("https://t.test/v1/events").mock(side_effect=lambda _: next(responses))
    seen: list[RegenerateFeedback] = []

    async def regenerate(feedback: RegenerateFeedback) -> str:
        seen.append(feedback)
        return "safer regenerated reply"

    guardrail = guard(
        agent_id="factory-agent",
        base_url="https://t.test",
        mode=GuardMode.REWRITE_OR_REGENERATE,
        regenerate=regenerate,
    )
    out = await guardrail(input="hello", draft="unsafe reply")
    await guardrail.aclose()

    assert out == "safer regenerated reply"
    assert len(route.calls) == 2
    assert seen[0].reason == "contains confidential data"
    assert seen[0].attempt == 1
    assert seen[0].max_attempts == 1


@pytest.mark.asyncio
@respx.mock
async def test_guard_factory_caps_regeneration_attempts() -> None:
    responses = iter(
        [
            httpx.Response(
                200,
                json=_decision_payload(effect="transform", transformed_value=None, trace_id="t-1"),
            ),
            httpx.Response(
                200,
                json=_decision_payload(effect="transform", transformed_value=None, trace_id="t-2"),
            ),
        ]
    )
    route = respx.post("https://t.test/v1/events").mock(side_effect=lambda _: next(responses))
    seen: list[RegenerateFeedback] = []

    async def regenerate(feedback: RegenerateFeedback) -> str:
        seen.append(feedback)
        return "still unsafe reply"

    guardrail = guard(
        agent_id="factory-agent",
        base_url="https://t.test",
        mode=GuardMode.REWRITE_OR_REGENERATE,
        regenerate=regenerate,
        max_regenerations=1,
    )
    out = await guardrail(input="hello", draft="unsafe reply")
    await guardrail.aclose()

    assert out == "I can't help with that request."
    assert len(route.calls) == 2
    assert len(seen) == 1
