from __future__ import annotations

import asyncio
import json
from typing import cast

import httpx
import pytest
import respx
from ag2 import Agent, Context
from ag2.events import (
    ModelMessage,
    ModelRequest,
    ModelResponse,
    TextInput,
    ToolCallEvent,
    ToolResultEvent,
)
from ag2.middleware import BaseMiddleware, Middleware

from trustloopguard import AsyncClient, RetryConfig, SideEffectClass, Transport
from trustloopguard.integrations import (
    AdapterLogEvent,
    AdapterWarning,
    AdapterWarningCode,
    tool_schema_hash,
)
from trustloopguard.integrations.ag2 import guard_ag2


def _decision(
    effect: str,
    *,
    trace_id: str = "trace-1",
    transformed_value: str | None = None,
    lease: dict[str, object] | None = None,
) -> dict[str, object]:
    payload: dict[str, object] = {
        "trace_id": trace_id,
        "domain": "tool",
        "effect": effect,
        "reason": "test decision",
        "findings": [],
        "latency_ms": 1,
    }
    if transformed_value is not None:
        payload["transformed_value"] = transformed_value
    if lease is not None:
        payload["lease"] = lease
    return payload


def _middleware(agent: Agent, event: ModelRequest) -> object:
    context = cast(Context, object())
    return agent._middleware[0](event, context)  # noqa: SLF001


def _result_text(result: ToolResultEvent) -> str:
    return cast(TextInput, result.result.parts[0]).content


@pytest.mark.asyncio
@respx.mock
async def test_ag2_permit_executes_once_with_schema_and_evidence() -> None:
    route = respx.post("https://api.example.test/v1/events").mock(
        return_value=httpx.Response(200, json=_decision("permit"))
    )
    calls = 0

    def confirm_order(order_id: str) -> str:
        return order_id

    agent = Agent("sales-agent", tools=[confirm_order])
    logs: list[AdapterLogEvent] = []
    async with AsyncClient("https://api.example.test") as client:
        assert (
            guard_ag2(
                agent,
                client=client,
                tool_side_effects={
                    "confirm_order": SideEffectClass.api_mutation
                },
                log=logs.append,
            )
            is agent
        )
        request = ModelRequest([TextInput("confirm it")])
        middleware = _middleware(agent, request)
        event = ToolCallEvent(
            name="confirm_order",
            id="call-1",
            arguments='{"order_id":"secret-order"}',
        )

        async def call_next(
            tool_event: ToolCallEvent,
            _: Context,
        ) -> ToolResultEvent:
            nonlocal calls
            calls += 1
            return ToolResultEvent.from_call(tool_event, "confirmed")

        result = await middleware.on_tool_execution(  # type: ignore[attr-defined]
            call_next, event, cast(Context, object())
        )

    assert calls == 1
    assert _result_text(cast(ToolResultEvent, result)) == "confirmed"
    body = json.loads(route.calls.last.request.content)
    assert body["action"]["invocation_id"] == "call-1"
    assert body["action"]["parameters"] == {"order_id": "secret-order"}
    assert body["action"]["tool_identity"] == {
        "server_id": "ag2",
        "tool_name": "confirm_order",
        "schema_hash": tool_schema_hash(
            agent.tools[0].schema.function.parameters
        ),
    }
    assert body["sources"][0]["origin"] == "unknown"
    assert body["provenance"] == {
        "order_id": ["ag2.tool_arguments"]
    }
    assert logs[-1].executed
    assert "secret-order" not in repr(logs)


@pytest.mark.asyncio
@pytest.mark.parametrize(
    ("effect", "expected_fragment"),
    [
        ("deny", "blocked by policy"),
        ("defer", "verified context"),
        ("transform", "revised arguments"),
    ],
)
@respx.mock
async def test_ag2_non_execution_effects_return_normal_tool_results(
    effect: str,
    expected_fragment: str,
) -> None:
    respx.post("https://api.example.test/v1/events").mock(
        return_value=httpx.Response(200, json=_decision(effect))
    )
    calls = 0
    agent = Agent("sales-agent")
    async with AsyncClient("https://api.example.test") as client:
        guard_ag2(agent, client=client)
        request = ModelRequest([TextInput("confirm it")])
        middleware = _middleware(agent, request)
        event = ToolCallEvent(
            name="confirm_order",
            id=f"call-{effect}",
            arguments='{"order_id":"order-1"}',
        )

        async def call_next(
            tool_event: ToolCallEvent,
            _: Context,
        ) -> ToolResultEvent:
            nonlocal calls
            calls += 1
            return ToolResultEvent.from_call(tool_event, "unexpected")

        result = await middleware.on_tool_execution(  # type: ignore[attr-defined]
            call_next, event, cast(Context, object())
        )

    assert calls == 0
    assert isinstance(result, ToolResultEvent)
    assert result.parent_id == event.id
    assert expected_fragment in _result_text(result)


@pytest.mark.asyncio
@respx.mock
async def test_ag2_sdk_failure_before_callback_fails_closed() -> None:
    respx.post("https://api.example.test/v1/events").mock(
        return_value=httpx.Response(503, text="unavailable")
    )
    calls = 0
    agent = Agent("sales-agent")
    async with AsyncClient(
        "https://api.example.test",
        retry=RetryConfig(max_attempts=1),
    ) as client:
        guard_ag2(agent, client=client)
        request = ModelRequest([TextInput("confirm it")])
        middleware = _middleware(agent, request)
        event = ToolCallEvent(name="confirm_order", id="call-1")

        async def call_next(
            tool_event: ToolCallEvent,
            _: Context,
        ) -> ToolResultEvent:
            nonlocal calls
            calls += 1
            return ToolResultEvent.from_call(tool_event, "unexpected")

        result = await middleware.on_tool_execution(  # type: ignore[attr-defined]
            call_next, event, cast(Context, object())
        )

    assert calls == 0
    assert "Safety checks are unavailable" in _result_text(result)


@pytest.mark.asyncio
@respx.mock
async def test_ag2_lease_completion_failure_returns_captured_result_once() -> None:
    lease_id = "018f3333-3333-7333-8333-333333333333"
    respx.post("https://api.example.test/v1/events").mock(
        return_value=httpx.Response(
            200,
            json=_decision(
                "permit",
                lease={
                    "id": lease_id,
                    "intent_id": "018f4444-4444-7444-8444-444444444444",
                    "grant_id": "018f2222-2222-7222-8222-222222222222",
                    "attempt_id": "attempt-1",
                    "fingerprint": "sha256:v1:subject",
                    "status": "claimed",
                    "claimed_at": "2026-07-15T00:00:00Z",
                    "expires_at": "2026-07-15T00:05:00Z",
                },
            ),
        )
    )
    respx.post(
        f"https://api.example.test/v1/authorization/leases/{lease_id}/complete"
    ).mock(return_value=httpx.Response(503, text="unavailable"))
    calls = 0
    warnings: list[AdapterWarning] = []
    agent = Agent("sales-agent")
    async with AsyncClient(
        "https://api.example.test",
        retry=RetryConfig(max_attempts=1),
    ) as client:
        guard_ag2(agent, client=client, on_warning=warnings.append)
        request = ModelRequest([TextInput("confirm it")])
        middleware = _middleware(agent, request)
        event = ToolCallEvent(name="confirm_order", id="call-1")

        async def call_next(
            tool_event: ToolCallEvent,
            _: Context,
        ) -> ToolResultEvent:
            nonlocal calls
            calls += 1
            return ToolResultEvent.from_call(tool_event, "confirmed")

        result = await middleware.on_tool_execution(  # type: ignore[attr-defined]
            call_next, event, cast(Context, object())
        )

    assert calls == 1
    assert _result_text(result) == "confirmed"
    assert any(
        warning.code
        is AdapterWarningCode.lease_completion_failed_after_execution
        for warning in warnings
    )


@pytest.mark.asyncio
@respx.mock
async def test_ag2_customer_sdk_error_and_regular_error_are_preserved() -> None:
    respx.post("https://api.example.test/v1/events").mock(
        return_value=httpx.Response(200, json=_decision("permit"))
    )
    agent = Agent("sales-agent")
    async with AsyncClient("https://api.example.test") as client:
        guard_ag2(agent, client=client)
        middleware = _middleware(
            agent, ModelRequest([TextInput("confirm it")])
        )
        event = ToolCallEvent(name="confirm_order", id="call-1")
        sdk_error = Transport("customer tool failed")

        async def sdk_failure(_: ToolCallEvent, __: Context) -> ToolResultEvent:
            raise sdk_error

        with pytest.raises(Transport) as caught:
            await middleware.on_tool_execution(  # type: ignore[attr-defined]
                sdk_failure, event, cast(Context, object())
            )
        assert caught.value is sdk_error

        regular_error = RuntimeError("customer bug")

        async def regular_failure(
            _: ToolCallEvent, __: Context
        ) -> ToolResultEvent:
            raise regular_error

        with pytest.raises(RuntimeError) as caught_regular:
            await middleware.on_tool_execution(  # type: ignore[attr-defined]
                regular_failure, event, cast(Context, object())
            )
        assert caught_regular.value is regular_error


@pytest.mark.asyncio
@respx.mock
async def test_ag2_output_is_guarded_after_turn_without_replacing_response() -> None:
    respx.post("https://api.example.test/v1/events").mock(
        return_value=httpx.Response(
            200,
            json=_decision(
                "transform",
                trace_id="trace-output",
                transformed_value="Safe reply",
            ),
        )
    )
    agent = Agent("sales-agent")
    async with AsyncClient("https://api.example.test") as client:
        guard_ag2(agent, client=client)
        request = ModelRequest([TextInput("unsafe request")])
        middleware = _middleware(agent, request)
        response = ModelResponse(ModelMessage("Unsafe draft"))

        async def call_next(
            _: ModelRequest,
            __: Context,
        ) -> ModelResponse:
            return response

        guarded = await middleware.on_turn(  # type: ignore[attr-defined]
            call_next, request, cast(Context, object())
        )

    assert guarded is response
    assert guarded.message is not None
    assert guarded.message.content == "Safe reply"


@pytest.mark.asyncio
@respx.mock
async def test_ag2_concurrent_calls_keep_distinct_invocation_ids() -> None:
    submitted: list[str] = []

    def response(request: httpx.Request) -> httpx.Response:
        submitted.append(
            json.loads(request.content)["action"]["invocation_id"]
        )
        return httpx.Response(200, json=_decision("permit"))

    respx.post("https://api.example.test/v1/events").mock(side_effect=response)
    agent = Agent("sales-agent")
    async with AsyncClient("https://api.example.test") as client:
        guard_ag2(agent, client=client)
        middleware = _middleware(
            agent, ModelRequest([TextInput("two calls")])
        )

        async def call_next(
            event: ToolCallEvent,
            _: Context,
        ) -> ToolResultEvent:
            return ToolResultEvent.from_call(event, event.id)

        results = await asyncio.gather(
            middleware.on_tool_execution(  # type: ignore[attr-defined]
                call_next,
                ToolCallEvent(name="lookup", id="call-a"),
                cast(Context, object()),
            ),
            middleware.on_tool_execution(  # type: ignore[attr-defined]
                call_next,
                ToolCallEvent(name="lookup", id="call-b"),
                cast(Context, object()),
            ),
        )

    assert {item.parent_id for item in results} == {"call-a", "call-b"}
    assert set(submitted) == {"call-a", "call-b"}


def test_ag2_double_install_is_idempotent_and_warns_once() -> None:
    warnings: list[AdapterWarning] = []
    agent = Agent("sales-agent")
    client = AsyncClient("https://api.example.test")
    guard_ag2(agent, client=client)
    initial_count = len(agent._middleware)  # noqa: SLF001
    guard_ag2(agent, client=client, on_warning=warnings.append)
    guard_ag2(agent, client=client, on_warning=warnings.append)

    assert len(agent._middleware) == initial_count  # noqa: SLF001
    assert [warning.code for warning in warnings] == [
        AdapterWarningCode.already_guarded
    ]


def test_ag2_guard_is_inserted_outside_existing_middleware() -> None:
    class ExistingMiddleware(BaseMiddleware):
        pass

    agent = Agent(
        "sales-agent",
        middleware=[Middleware(ExistingMiddleware)],
    )
    client = AsyncClient("https://api.example.test")

    guard_ag2(agent, client=client)

    request = ModelRequest([TextInput("confirm it")])
    outer = agent._middleware[0](request, cast(Context, object()))  # noqa: SLF001
    inner = agent._middleware[1](request, cast(Context, object()))  # noqa: SLF001
    assert type(outer).__name__ == "_TrustLoopAG2Middleware"
    assert isinstance(inner, ExistingMiddleware)
