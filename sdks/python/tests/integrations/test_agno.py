from __future__ import annotations

import inspect
import json
from typing import Any, cast

import httpx
import pytest
import respx
from agno.agent import Agent
from agno.run import RunContext
from agno.run.agent import RunOutput
from agno.tools.function import Function

from featherlane_ai import (
    AsyncClient,
    Client,
    RetryConfig,
    SideEffectClass,
    Transport,
)
from featherlane_ai.integrations import (
    AdapterLogEvent,
    AdapterWarning,
    AdapterWarningCode,
    tool_schema_hash,
)
from featherlane_ai.integrations.agno import guard_agno


def _decision(
    effect: str,
    *,
    trace_id: str = "trace-1",
    transformed_value: str | None = None,
    approval: dict[str, object] | None = None,
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
    if approval is not None:
        payload["approval"] = approval
    if lease is not None:
        payload["lease"] = lease
    return payload


def _run_context() -> RunContext:
    return RunContext(
        run_id="agno-run-1",
        session_id="session-1",
        user_id="user-1",
    )


@respx.mock
def test_agno_sync_permit_executes_once_with_schema_and_framework_ids() -> None:
    route = respx.post("https://api.example.test/v1/events").mock(
        return_value=httpx.Response(200, json=_decision("permit"))
    )
    calls = 0
    logs: list[AdapterLogEvent] = []

    def confirm_order(order_id: str) -> str:
        """Confirm an order."""
        return f"confirmed:{order_id}"

    agent = Agent(name="sales-agent", id="agent-id", tools=[confirm_order])
    with Client("https://api.example.test") as client:
        assert (
            guard_agno(
                agent,
                client=client,
                tool_side_effects={
                    "confirm_order": SideEffectClass.api_mutation
                },
                log=logs.append,
            )
            is agent
        )
        hook = cast(Any, agent.tool_hooks[0])

        def function_call(**arguments: str) -> str:
            nonlocal calls
            calls += 1
            return confirm_order(**arguments)

        result = hook(
            agent=agent,
            run_context=_run_context(),
            function_name="confirm_order",
            function_call=function_call,
            arguments={"order_id": "secret-order"},
        )

    assert result == "confirmed:secret-order"
    assert calls == 1
    body = json.loads(route.calls.last.request.content)
    assert body["principal"] == {
        "workspace_id": "",
        "environment_id": "",
        "agent_id": "agent-id",
        "session_id": "session-1",
        "user_id": "user-1",
    }
    assert "run_id" not in body["principal"]
    assert body["context"] == {
        "framework": "agno",
        "framework_agent_name": "sales-agent",
        "framework_run_id": "agno-run-1",
        "framework_session_id": "session-1",
        "framework_user_id": "user-1",
    }
    expected_schema = Function.from_callable(confirm_order).parameters
    assert body["action"]["tool_identity"]["schema_hash"] == tool_schema_hash(
        expected_schema
    )
    assert body["provenance"] == {
        "order_id": ["agno.tool_arguments"]
    }
    assert logs[-1].executed
    assert "secret-order" not in repr(logs)


@pytest.mark.parametrize(
    ("effect", "expected_fragment"),
    [
        ("deny", "blocked by policy"),
        ("defer", "verified context"),
        ("transform", "revised arguments"),
    ],
)
@respx.mock
def test_agno_sync_non_execution_effects_never_call_tool(
    effect: str,
    expected_fragment: str,
) -> None:
    respx.post("https://api.example.test/v1/events").mock(
        return_value=httpx.Response(200, json=_decision(effect))
    )
    calls = 0
    agent = Agent(name="sales-agent")
    with Client("https://api.example.test") as client:
        guard_agno(agent, client=client)
        hook = cast(Any, agent.tool_hooks[0])

        def function_call(**_: object) -> str:
            nonlocal calls
            calls += 1
            return "unexpected"

        result = hook(
            agent=agent,
            run_context=_run_context(),
            function_name="confirm_order",
            function_call=function_call,
            arguments={"order_id": "order-1"},
        )

    assert calls == 0
    assert expected_fragment in result


@respx.mock
def test_agno_approval_timeout_returns_safe_result_without_execution() -> None:
    approval_id = "018f1111-1111-7111-8111-111111111111"
    respx.post("https://api.example.test/v1/events").mock(
        return_value=httpx.Response(
            200,
            json=_decision(
                "require_approval",
                approval={
                    "id": approval_id,
                    "status": "pending",
                    "envelope_hash": "sha256:v1:approval",
                    "expires_at": "2026-07-15T01:00:00Z",
                    "poll_after_ms": 1,
                },
            ),
        )
    )
    calls = 0
    agent = Agent(name="sales-agent")
    with Client("https://api.example.test") as client:
        guard_agno(agent, client=client, approval_timeout_s=0)
        hook = cast(Any, agent.tool_hooks[0])

        def function_call(**_: object) -> str:
            nonlocal calls
            calls += 1
            return "unexpected"

        result = hook(
            agent=agent,
            run_context=_run_context(),
            function_name="confirm_order",
            function_call=function_call,
            arguments={},
        )

    assert calls == 0
    assert "still requires approval" in result


@pytest.mark.asyncio
@respx.mock
async def test_agno_async_client_installs_coroutine_hooks_and_executes_once() -> None:
    respx.post("https://api.example.test/v1/events").mock(
        return_value=httpx.Response(200, json=_decision("permit"))
    )
    calls = 0

    async def lookup_inventory(sku: str) -> str:
        return f"available:{sku}"

    agent = Agent(name="sales-agent", tools=[lookup_inventory])
    async with AsyncClient("https://api.example.test") as client:
        guard_agno(
            agent,
            client=client,
            tool_side_effects={
                "lookup_inventory": SideEffectClass.read
            },
        )
        hook = cast(Any, agent.tool_hooks[0])
        output_hook = cast(Any, agent.post_hooks[-1])
        assert inspect.iscoroutinefunction(hook)
        assert inspect.iscoroutinefunction(output_hook)

        async def function_call(**arguments: str) -> str:
            nonlocal calls
            calls += 1
            return await lookup_inventory(**arguments)

        result = await hook(
            agent=agent,
            run_context=_run_context(),
            function_name="lookup_inventory",
            function_call=function_call,
            arguments={"sku": "sku-1"},
        )

    assert result == "available:sku-1"
    assert calls == 1


def test_agno_sync_client_installs_non_coroutine_hooks() -> None:
    agent = Agent(name="sales-agent")
    with Client("https://api.example.test") as client:
        guard_agno(agent, client=client)
        assert not inspect.iscoroutinefunction(agent.tool_hooks[0])
        assert not inspect.iscoroutinefunction(agent.post_hooks[-1])


@respx.mock
def test_agno_sdk_failure_before_callback_fails_closed() -> None:
    respx.post("https://api.example.test/v1/events").mock(
        return_value=httpx.Response(503, text="unavailable")
    )
    calls = 0
    agent = Agent(name="sales-agent")
    with Client(
        "https://api.example.test",
        retry=RetryConfig(max_attempts=1),
    ) as client:
        guard_agno(agent, client=client)
        hook = cast(Any, agent.tool_hooks[0])

        def function_call(**_: object) -> str:
            nonlocal calls
            calls += 1
            return "unexpected"

        result = hook(
            agent=agent,
            run_context=_run_context(),
            function_name="confirm_order",
            function_call=function_call,
            arguments={},
        )

    assert calls == 0
    assert "Safety checks are unavailable" in result


@respx.mock
def test_agno_lease_completion_failure_returns_captured_result_once() -> None:
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
    agent = Agent(name="sales-agent")
    with Client(
        "https://api.example.test",
        retry=RetryConfig(max_attempts=1),
    ) as client:
        guard_agno(agent, client=client, on_warning=warnings.append)
        hook = cast(Any, agent.tool_hooks[0])

        def function_call(**_: object) -> str:
            nonlocal calls
            calls += 1
            return "confirmed"

        result = hook(
            agent=agent,
            run_context=_run_context(),
            function_name="confirm_order",
            function_call=function_call,
            arguments={},
        )

    assert calls == 1
    assert result == "confirmed"
    assert any(
        warning.code
        is AdapterWarningCode.lease_completion_failed_after_execution
        for warning in warnings
    )


@respx.mock
def test_agno_customer_sdk_error_and_regular_error_are_preserved() -> None:
    respx.post("https://api.example.test/v1/events").mock(
        return_value=httpx.Response(200, json=_decision("permit"))
    )
    agent = Agent(name="sales-agent")
    with Client("https://api.example.test") as client:
        guard_agno(agent, client=client)
        hook = cast(Any, agent.tool_hooks[0])
        sdk_error = Transport("customer tool failed")

        def sdk_failure(**_: object) -> str:
            raise sdk_error

        with pytest.raises(Transport) as caught:
            hook(
                agent=agent,
                run_context=_run_context(),
                function_name="confirm_order",
                function_call=sdk_failure,
                arguments={},
            )
        assert caught.value is sdk_error

        regular_error = RuntimeError("customer bug")

        def regular_failure(**_: object) -> str:
            raise regular_error

        with pytest.raises(RuntimeError) as caught_regular:
            hook(
                agent=agent,
                run_context=_run_context(),
                function_name="confirm_order",
                function_call=regular_failure,
                arguments={},
            )
        assert caught_regular.value is regular_error


@respx.mock
def test_agno_output_hook_mutates_only_plain_text_and_runs_last() -> None:
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

    def existing_output_hook(run_output: RunOutput) -> None:
        run_output.content = f"{run_output.content} after existing hook"

    agent = Agent(
        name="sales-agent",
        tool_hooks=[lambda function_call, arguments: function_call(**arguments)],
        post_hooks=[existing_output_hook],
    )
    with Client("https://api.example.test") as client:
        guard_agno(agent, client=client)
        assert agent.tool_hooks[0] is not agent.tool_hooks[1]
        assert agent.post_hooks[0] is existing_output_hook
        output = RunOutput(
            run_id="agno-run-1",
            session_id="session-1",
            user_id="user-1",
            content="Unsafe draft",
        )
        existing_output_hook(output)
        result = cast(Any, agent.post_hooks[-1])(
            run_output=output,
            agent=agent,
            run_context=_run_context(),
        )

    assert result is None
    assert output.content == "Safe reply"
    assert output.run_id == "agno-run-1"
    body = json.loads(
        respx.calls.last.request.content
    )
    assert body["action"]["parameters"]["text"] == (
        "Unsafe draft after existing hook"
    )


def test_agno_structured_output_passes_through_with_warning() -> None:
    warnings: list[AdapterWarning] = []
    agent = Agent(name="sales-agent")
    with Client("https://api.example.test") as client:
        guard_agno(agent, client=client, on_warning=warnings.append)
        output = RunOutput(content={"answer": "structured"})
        cast(Any, agent.post_hooks[-1])(
            run_output=output,
            agent=agent,
            run_context=_run_context(),
        )

    assert output.content == {"answer": "structured"}
    assert warnings[-1].code is AdapterWarningCode.structured_output_unavailable


def test_agno_provider_tool_warning_and_double_install_idempotency() -> None:
    warnings: list[AdapterWarning] = []
    agent = Agent(
        name="sales-agent",
        tools=[{"type": "web_search"}],
    )
    with Client("https://api.example.test") as client:
        guard_agno(agent, client=client, on_warning=warnings.append)
        tool_hook_count = len(agent.tool_hooks or [])
        output_hook_count = len(agent.post_hooks or [])
        guard_agno(agent, client=client, on_warning=warnings.append)
        guard_agno(agent, client=client, on_warning=warnings.append)

    assert len(agent.tool_hooks or []) == tool_hook_count
    assert len(agent.post_hooks or []) == output_hook_count
    assert [warning.code for warning in warnings].count(
        AdapterWarningCode.provider_hosted_tool_unavailable
    ) == 1
    assert [warning.code for warning in warnings].count(
        AdapterWarningCode.already_guarded
    ) == 1


def test_agno_requires_identity_when_agent_has_no_id_or_name() -> None:
    agent = Agent()
    with Client("https://api.example.test") as client:
        with pytest.raises(ValueError, match="agent_id"):
            guard_agno(agent, client=client)
