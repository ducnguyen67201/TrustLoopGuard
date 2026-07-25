"""TrustLoopGuard middleware for the current async-first AG2 framework."""

from __future__ import annotations

import copy
import time
import weakref
from collections.abc import Callable, Mapping, Sequence
from dataclasses import dataclass
from typing import Any, TypeVar, cast

from ag2 import Agent, Context, Plugin
from ag2.events import (
    BaseEvent,
    ModelRequest,
    ModelResponse,
    TextInput,
    ToolCallEvent,
    ToolResultEvent,
)
from ag2.middleware import (
    AgentTurn,
    BaseMiddleware,
    Middleware,
    ToolExecution,
    ToolResultType,
)
from ag2.tools.final import FunctionTool, FunctionToolSchema
from ag2.tools.tool import Tool

from trustloopguard._generated.types import (
    Principal,
    SideEffectClass,
    ToolIdentity,
)
from trustloopguard.client import AsyncClient
from trustloopguard.errors import SdkError
from trustloopguard.guard import GuardLogEvent, guard_async
from trustloopguard.integrations._core import (
    AdapterLogEvent,
    AdapterWarning,
    AdapterWarningCode,
    CallbackState,
    OutputGuardMessages,
    ToolGuardMessages,
    build_argument_evidence,
    copied_context,
    elapsed_ms,
    emit_log,
    emit_warning,
    resolve_side_effect,
    safe_tool_message,
    tool_schema_hash,
    warn_once,
)

AgentT = TypeVar("AgentT", bound=Agent)
_GUARDED_AGENTS: weakref.WeakSet[Agent] = weakref.WeakSet()
_DUPLICATE_WARNED: weakref.WeakSet[Agent] = weakref.WeakSet()


@dataclass(frozen=True)
class _AG2Config:
    client: AsyncClient
    agent_id: str
    agent_name: str
    tools: tuple[Tool, ...]
    tool_side_effects: Mapping[str, SideEffectClass]
    default_side_effect: SideEffectClass
    tool_messages: ToolGuardMessages
    output_messages: OutputGuardMessages
    approval_timeout_s: float
    poll_interval_s: float | None
    context: Mapping[str, Any]
    output_fail_closed: bool
    on_warning: Callable[[AdapterWarning], None] | None
    log: Callable[[AdapterLogEvent], None] | None
    warned: set[str]


class _TrustLoopAG2Middleware(BaseMiddleware):
    def __init__(
        self,
        event: BaseEvent,
        context: Context,
        *,
        config: _AG2Config,
    ) -> None:
        super().__init__(event, context)
        self._config = config

    async def on_tool_execution(
        self,
        call_next: ToolExecution,
        event: ToolCallEvent,
        context: Context,
    ) -> ToolResultType:
        start = time.monotonic()
        parameters = copy.deepcopy(event.serialized_arguments)
        schema = await self._schema_for(event.name, context)
        invocation_id = event.id
        side_effect = resolve_side_effect(
            event.name,
            self._config.tool_side_effects,
            self._config.default_side_effect,
            framework="ag2",
            warned=self._config.warned,
            on_warning=self._config.on_warning,
        )
        sources, provenance = build_argument_evidence("ag2", parameters)
        callback_state: CallbackState[ToolResultType] = CallbackState()

        async def execute(_: dict[str, Any]) -> ToolResultType:
            return await callback_state.run_async(
                lambda: call_next(event, context)
            )

        try:
            result = await self._config.client.with_authorized_action(
                agent_id=self._config.agent_id,
                operation=event.name,
                tool_identity=ToolIdentity(
                    server_id="ag2",
                    tool_name=event.name,
                    schema_hash=tool_schema_hash(schema),
                ),
                execute=execute,
                invocation_id=invocation_id,
                parameters=parameters,
                side_effect=side_effect,
                principal=Principal(
                    workspace_id="",
                    environment_id="",
                    agent_id=self._config.agent_id,
                ),
                sources=sources,
                provenance=provenance,
                context=copied_context(
                    self._config.context,
                    framework="ag2",
                    framework_agent_name=self._config.agent_name,
                    tool_call_id=invocation_id,
                ),
                timeout=self._config.approval_timeout_s,
                poll_interval=self._config.poll_interval_s,
            )
        except SdkError:
            if callback_state.error is not None:
                raise callback_state.error
            if callback_state.completed:
                warn_once(
                    AdapterWarningCode.lease_completion_failed_after_execution,
                    message=(
                        "The tool executed, but TrustLoopGuard could not report "
                        "lease completion. The tool will not be retried."
                    ),
                    framework="ag2",
                    warned=self._config.warned,
                    on_warning=self._config.on_warning,
                    tool_name=event.name,
                    key=f"lease-completion:{event.name}:{invocation_id}",
                )
                emit_log(
                    self._config.log,
                    AdapterLogEvent(
                        framework="ag2",
                        boundary="tool",
                        agent_id=self._config.agent_id,
                        trace_id="",
                        effect="completion_error",
                        executed=True,
                        latency_ms=elapsed_ms(start),
                        tool_name=event.name,
                        invocation_id=invocation_id,
                    ),
                )
                return cast(ToolResultType, callback_state.value)
            emit_log(
                self._config.log,
                AdapterLogEvent(
                    framework="ag2",
                    boundary="tool",
                    agent_id=self._config.agent_id,
                    trace_id="",
                    effect="error",
                    executed=False,
                    latency_ms=elapsed_ms(start),
                    tool_name=event.name,
                    invocation_id=invocation_id,
                ),
            )
            return ToolResultEvent.from_call(
                event, self._config.tool_messages.unavailable
            )

        emit_log(
            self._config.log,
            AdapterLogEvent(
                framework="ag2",
                boundary="tool",
                agent_id=self._config.agent_id,
                trace_id=result.decision.trace_id,
                effect=result.decision.effect.value,
                executed=result.executed,
                latency_ms=elapsed_ms(start),
                tool_name=event.name,
                invocation_id=invocation_id,
            ),
        )
        if result.executed:
            return cast(ToolResultType, result.value)
        return ToolResultEvent.from_call(
            event,
            safe_tool_message(result.decision.effect, self._config.tool_messages),
        )

    async def on_turn(
        self,
        call_next: AgentTurn,
        event: BaseEvent,
        context: Context,
    ) -> ModelResponse:
        response = await call_next(event, context)
        if response.message is None or not isinstance(
            response.message.content, str
        ):
            return response

        messages = self._config.output_messages

        async def deny(_: Any) -> str:
            return messages.deny

        async def require_approval(_: Any) -> str:
            return messages.require_approval

        async def defer(_: Any) -> str:
            return messages.defer

        async def revise(
            revised: str | None,
            _: str,
            __: Any,
        ) -> str:
            return revised if isinstance(revised, str) else messages.deny

        async def unavailable(_: SdkError, draft: str) -> str:
            return messages.unavailable if self._config.output_fail_closed else draft

        def output_log(event_log: GuardLogEvent) -> None:
            effect = (
                "error" if event_log.branch == "error" else event_log.effect
            )
            emit_log(
                self._config.log,
                AdapterLogEvent(
                    framework="ag2",
                    boundary="output",
                    agent_id=self._config.agent_id,
                    trace_id=event_log.trace_id,
                    effect=effect,
                    executed=event_log.branch in {"permit", "revise"},
                    latency_ms=event_log.latency_ms,
                ),
            )

        response.message.content = await guard_async(
            client=self._config.client,
            agent_id=self._config.agent_id,
            input=_input_text(event),
            draft=response.message.content,
            context=copied_context(
                self._config.context,
                framework="ag2",
                framework_agent_name=self._config.agent_name,
            ),
            on_block=deny,
            on_require_approval=require_approval,
            on_defer=defer,
            on_revise=revise,
            on_error=unavailable if self._config.output_fail_closed else None,
            log=output_log,
        )
        return response

    async def _schema_for(
        self,
        tool_name: str,
        context: Context,
    ) -> Mapping[str, Any]:
        for tool in self._config.tools:
            if isinstance(tool, FunctionTool):
                schema = tool.schema
                if schema.function.name == tool_name:
                    return schema.function.parameters
                continue
            try:
                schemas: Sequence[Any] = tuple(await tool.schemas(context))
            except Exception:
                continue
            for schema in schemas:
                if (
                    isinstance(schema, FunctionToolSchema)
                    and schema.function.name == tool_name
                ):
                    return schema.function.parameters
        warn_once(
            AdapterWarningCode.tool_schema_unavailable,
            message=(
                "The tool schema was not visible through the AG2 agent registry; "
                "the call is still guarded with an empty-schema identity."
            ),
            framework="ag2",
            warned=self._config.warned,
            on_warning=self._config.on_warning,
            tool_name=tool_name,
        )
        return {}


def guard_ag2(
    agent: AgentT,
    *,
    client: AsyncClient,
    agent_id: str | None = None,
    tool_side_effects: Mapping[str, SideEffectClass] | None = None,
    default_side_effect: SideEffectClass = SideEffectClass.api_mutation,
    tool_messages: ToolGuardMessages | None = None,
    output_messages: OutputGuardMessages | None = None,
    approval_timeout_s: float = 60.0,
    poll_interval_s: float | None = None,
    context: Mapping[str, Any] | None = None,
    output_fail_closed: bool = True,
    on_warning: Callable[[AdapterWarning], None] | None = None,
    log: Callable[[AdapterLogEvent], None] | None = None,
) -> AgentT:
    """Attach one outer TrustLoopGuard middleware and return ``agent`` unchanged."""

    if not isinstance(client, AsyncClient):
        raise TypeError("guard_ag2 requires trustloopguard.AsyncClient")
    if agent in _GUARDED_AGENTS:
        if agent not in _DUPLICATE_WARNED:
            _DUPLICATE_WARNED.add(agent)
            emit_warning(
                on_warning,
                AdapterWarning(
                    code=AdapterWarningCode.already_guarded,
                    message="This AG2 agent already has TrustLoopGuard middleware.",
                    framework="ag2",
                ),
            )
        return agent

    resolved_agent_id = agent_id or agent.name
    if not isinstance(resolved_agent_id, str) or not resolved_agent_id.strip():
        raise ValueError("agent_id is required when the AG2 agent has no name")
    config = _AG2Config(
        client=client,
        agent_id=resolved_agent_id,
        agent_name=agent.name,
        tools=tuple(agent.tools),
        tool_side_effects=dict(tool_side_effects or {}),
        default_side_effect=default_side_effect,
        tool_messages=tool_messages or ToolGuardMessages(),
        output_messages=output_messages or OutputGuardMessages(),
        approval_timeout_s=approval_timeout_s,
        poll_interval_s=poll_interval_s,
        context=copy.deepcopy(dict(context or {})),
        output_fail_closed=output_fail_closed,
        on_warning=on_warning,
        log=log,
        warned=set(),
    )
    agent.insert_middleware(
        Middleware(_TrustLoopAG2Middleware, config=config)
    )
    _GUARDED_AGENTS.add(agent)
    return agent


def _input_text(event: BaseEvent) -> str:
    if not isinstance(event, ModelRequest):
        return ""
    return "\n".join(
        part.content for part in event.parts if isinstance(part, TextInput)
    )


__all__ = ["guard_ag2"]
