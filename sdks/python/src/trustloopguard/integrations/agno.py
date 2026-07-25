"""TrustLoopGuard tool and output hooks for Agno Agent."""

from __future__ import annotations

import copy
import inspect
import time
import uuid
import weakref
from collections.abc import Callable, Mapping, Sequence
from dataclasses import dataclass
from typing import Any, TypeVar, cast, overload

from agno.agent import Agent
from agno.run import RunContext
from agno.run.agent import RunOutput
from agno.tools.function import Function
from agno.tools.toolkit import Toolkit

from trustloopguard._generated.types import (
    Principal,
    SideEffectClass,
    ToolIdentity,
)
from trustloopguard.client import AsyncClient, Client
from trustloopguard.errors import SdkError
from trustloopguard.guard import GuardLogEvent, guard, guard_async
from trustloopguard.integrations._core import (
    AdapterLogEvent,
    AdapterWarning,
    AdapterWarningCode,
    CallbackState,
    Framework,
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

AgnoAgentT = TypeVar("AgnoAgentT", bound=Agent)


class _WeakIdentitySet:
    """Weak identity registry for Agno's intentionally unhashable Agent."""

    def __init__(self) -> None:
        self._references: list[weakref.ReferenceType[Agent]] = []

    def __contains__(self, agent: Agent) -> bool:
        self._discard_dead()
        return any(reference() is agent for reference in self._references)

    def add(self, agent: Agent) -> None:
        if agent not in self:
            self._references.append(weakref.ref(agent))

    def _discard_dead(self) -> None:
        self._references = [
            reference
            for reference in self._references
            if reference() is not None
        ]


_GUARDED_AGENTS = _WeakIdentitySet()
_DUPLICATE_WARNED = _WeakIdentitySet()


@dataclass(frozen=True)
class _AgnoConfig:
    client: Client | AsyncClient
    agent_id: str
    agent_name: str | None
    schemas: Mapping[str, Mapping[str, Any]]
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


class _SyncAgnoAdapter:
    def __init__(self, config: _AgnoConfig) -> None:
        self._config = config

    def tool_hook(
        self,
        agent: Agent,
        run_context: RunContext,
        function_name: str,
        function_call: Callable[..., Any],
        arguments: dict[str, Any],
    ) -> Any:
        start = time.monotonic()
        invocation_id = str(uuid.uuid4())
        parameters = copy.deepcopy(arguments)
        side_effect = resolve_side_effect(
            function_name,
            self._config.tool_side_effects,
            self._config.default_side_effect,
            framework="agno",
            warned=self._config.warned,
            on_warning=self._config.on_warning,
        )
        sources, provenance = build_argument_evidence("agno", parameters)
        callback_state: CallbackState[Any] = CallbackState()

        def execute(approved: dict[str, Any]) -> Any:
            return callback_state.run(lambda: function_call(**approved))

        try:
            result = cast(Client, self._config.client).with_authorized_action(
                agent_id=self._config.agent_id,
                operation=function_name,
                tool_identity=ToolIdentity(
                    server_id="agno",
                    tool_name=function_name,
                    schema_hash=tool_schema_hash(
                        self._schema_for(function_name, run_context)
                    ),
                ),
                execute=execute,
                invocation_id=invocation_id,
                parameters=parameters,
                side_effect=side_effect,
                principal=_principal(self._config.agent_id, run_context),
                sources=sources,
                provenance=provenance,
                context=_context(self._config, agent, run_context),
                timeout=self._config.approval_timeout_s,
                poll_interval=self._config.poll_interval_s,
            )
        except SdkError:
            return self._sdk_failure(
                callback_state,
                function_name=function_name,
                invocation_id=invocation_id,
                start=start,
            )
        return self._tool_result(
            result,
            function_name=function_name,
            invocation_id=invocation_id,
            start=start,
        )

    def output_hook(
        self,
        run_output: RunOutput,
        agent: Agent,
        run_context: RunContext,
    ) -> None:
        if not isinstance(run_output.content, str):
            if run_output.content is not None:
                warn_once(
                    AdapterWarningCode.structured_output_unavailable,
                    message=(
                        "Agno returned structured content; the adapter left it "
                        "unchanged because replacing it with text would break its type."
                    ),
                    framework="agno",
                    warned=self._config.warned,
                    on_warning=self._config.on_warning,
                )
            return
        messages = self._config.output_messages

        def output_log(event_log: GuardLogEvent) -> None:
            _emit_output_log(self._config, event_log)

        run_output.content = cast(
            str,
            guard(
                client=cast(Client, self._config.client),
                agent_id=self._config.agent_id,
                input=_run_input_text(run_output),
                draft=run_output.content,
                context=_context(self._config, agent, run_context),
                on_block=lambda _: messages.deny,
                on_require_approval=lambda _: messages.require_approval,
                on_defer=lambda _: messages.defer,
                on_revise=lambda revised, _draft, _decision: (
                    revised if isinstance(revised, str) else messages.deny
                ),
                on_error=(
                    (lambda _error, _draft: messages.unavailable)
                    if self._config.output_fail_closed
                    else None
                ),
                log=output_log,
            ),
        )

    def _schema_for(
        self,
        function_name: str,
        run_context: RunContext,
    ) -> Mapping[str, Any]:
        schema = self._config.schemas.get(function_name)
        if schema is not None:
            return schema
        runtime_schemas = _collect_schemas(
            run_context.tools,
            async_mode=False,
            framework="agno",
            warned=self._config.warned,
            on_warning=self._config.on_warning,
            runtime=True,
        )
        schema = runtime_schemas.get(function_name)
        if schema is not None:
            return schema
        warn_once(
            AdapterWarningCode.tool_schema_unavailable,
            message=(
                "The Agno tool schema was unavailable; the local call is still "
                "guarded with an empty-schema identity."
            ),
            framework="agno",
            warned=self._config.warned,
            on_warning=self._config.on_warning,
            tool_name=function_name,
        )
        return {}

    def _sdk_failure(
        self,
        state: CallbackState[Any],
        *,
        function_name: str,
        invocation_id: str,
        start: float,
    ) -> Any:
        if state.error is not None:
            raise state.error
        if state.completed:
            _warn_completion_failure(
                self._config, function_name, invocation_id
            )
            _emit_tool_log(
                self._config,
                trace_id="",
                effect="completion_error",
                executed=True,
                function_name=function_name,
                invocation_id=invocation_id,
                start=start,
            )
            return state.value
        _emit_tool_log(
            self._config,
            trace_id="",
            effect="error",
            executed=False,
            function_name=function_name,
            invocation_id=invocation_id,
            start=start,
        )
        return self._config.tool_messages.unavailable

    def _tool_result(
        self,
        result: Any,
        *,
        function_name: str,
        invocation_id: str,
        start: float,
    ) -> Any:
        _emit_tool_log(
            self._config,
            trace_id=result.decision.trace_id,
            effect=result.decision.effect.value,
            executed=result.executed,
            function_name=function_name,
            invocation_id=invocation_id,
            start=start,
        )
        if result.executed:
            return result.value
        return safe_tool_message(
            result.decision.effect, self._config.tool_messages
        )


class _AsyncAgnoAdapter:
    def __init__(self, config: _AgnoConfig) -> None:
        self._config = config

    async def tool_hook(
        self,
        agent: Agent,
        run_context: RunContext,
        function_name: str,
        function_call: Callable[..., Any],
        arguments: dict[str, Any],
    ) -> Any:
        start = time.monotonic()
        invocation_id = str(uuid.uuid4())
        parameters = copy.deepcopy(arguments)
        side_effect = resolve_side_effect(
            function_name,
            self._config.tool_side_effects,
            self._config.default_side_effect,
            framework="agno",
            warned=self._config.warned,
            on_warning=self._config.on_warning,
        )
        sources, provenance = build_argument_evidence("agno", parameters)
        callback_state: CallbackState[Any] = CallbackState()

        async def call_function(approved: dict[str, Any]) -> Any:
            value = function_call(**approved)
            return await value if inspect.isawaitable(value) else value

        async def execute(approved: dict[str, Any]) -> Any:
            return await callback_state.run_async(
                lambda: call_function(approved)
            )

        try:
            result = await cast(
                AsyncClient, self._config.client
            ).with_authorized_action(
                agent_id=self._config.agent_id,
                operation=function_name,
                tool_identity=ToolIdentity(
                    server_id="agno",
                    tool_name=function_name,
                    schema_hash=tool_schema_hash(
                        self._schema_for(function_name, run_context)
                    ),
                ),
                execute=execute,
                invocation_id=invocation_id,
                parameters=parameters,
                side_effect=side_effect,
                principal=_principal(self._config.agent_id, run_context),
                sources=sources,
                provenance=provenance,
                context=_context(self._config, agent, run_context),
                timeout=self._config.approval_timeout_s,
                poll_interval=self._config.poll_interval_s,
            )
        except SdkError:
            return self._sdk_failure(
                callback_state,
                function_name=function_name,
                invocation_id=invocation_id,
                start=start,
            )
        return self._tool_result(
            result,
            function_name=function_name,
            invocation_id=invocation_id,
            start=start,
        )

    async def output_hook(
        self,
        run_output: RunOutput,
        agent: Agent,
        run_context: RunContext,
    ) -> None:
        if not isinstance(run_output.content, str):
            if run_output.content is not None:
                warn_once(
                    AdapterWarningCode.structured_output_unavailable,
                    message=(
                        "Agno returned structured content; the adapter left it "
                        "unchanged because replacing it with text would break its type."
                    ),
                    framework="agno",
                    warned=self._config.warned,
                    on_warning=self._config.on_warning,
                )
            return
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
            _emit_output_log(self._config, event_log)

        run_output.content = await guard_async(
            client=cast(AsyncClient, self._config.client),
            agent_id=self._config.agent_id,
            input=_run_input_text(run_output),
            draft=run_output.content,
            context=_context(self._config, agent, run_context),
            on_block=deny,
            on_require_approval=require_approval,
            on_defer=defer,
            on_revise=revise,
            on_error=unavailable if self._config.output_fail_closed else None,
            log=output_log,
        )

    def _schema_for(
        self,
        function_name: str,
        run_context: RunContext,
    ) -> Mapping[str, Any]:
        schema = self._config.schemas.get(function_name)
        if schema is not None:
            return schema
        runtime_schemas = _collect_schemas(
            run_context.tools,
            async_mode=True,
            framework="agno",
            warned=self._config.warned,
            on_warning=self._config.on_warning,
            runtime=True,
        )
        schema = runtime_schemas.get(function_name)
        if schema is not None:
            return schema
        warn_once(
            AdapterWarningCode.tool_schema_unavailable,
            message=(
                "The Agno tool schema was unavailable; the local call is still "
                "guarded with an empty-schema identity."
            ),
            framework="agno",
            warned=self._config.warned,
            on_warning=self._config.on_warning,
            tool_name=function_name,
        )
        return {}

    def _sdk_failure(
        self,
        state: CallbackState[Any],
        *,
        function_name: str,
        invocation_id: str,
        start: float,
    ) -> Any:
        if state.error is not None:
            raise state.error
        if state.completed:
            _warn_completion_failure(
                self._config, function_name, invocation_id
            )
            _emit_tool_log(
                self._config,
                trace_id="",
                effect="completion_error",
                executed=True,
                function_name=function_name,
                invocation_id=invocation_id,
                start=start,
            )
            return state.value
        _emit_tool_log(
            self._config,
            trace_id="",
            effect="error",
            executed=False,
            function_name=function_name,
            invocation_id=invocation_id,
            start=start,
        )
        return self._config.tool_messages.unavailable

    def _tool_result(
        self,
        result: Any,
        *,
        function_name: str,
        invocation_id: str,
        start: float,
    ) -> Any:
        _emit_tool_log(
            self._config,
            trace_id=result.decision.trace_id,
            effect=result.decision.effect.value,
            executed=result.executed,
            function_name=function_name,
            invocation_id=invocation_id,
            start=start,
        )
        if result.executed:
            return result.value
        return safe_tool_message(
            result.decision.effect, self._config.tool_messages
        )


@overload
def guard_agno(
    agent: AgnoAgentT,
    *,
    client: Client,
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
) -> AgnoAgentT: ...


@overload
def guard_agno(
    agent: AgnoAgentT,
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
) -> AgnoAgentT: ...


def guard_agno(
    agent: AgnoAgentT,
    *,
    client: Client | AsyncClient,
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
) -> AgnoAgentT:
    """Install hooks selected by the TrustLoopGuard client type."""

    if not isinstance(client, (Client, AsyncClient)):
        raise TypeError("guard_agno requires trustloopguard.Client or AsyncClient")
    if agent in _GUARDED_AGENTS:
        if agent not in _DUPLICATE_WARNED:
            _DUPLICATE_WARNED.add(agent)
            emit_warning(
                on_warning,
                AdapterWarning(
                    code=AdapterWarningCode.already_guarded,
                    message="This Agno agent already has TrustLoopGuard hooks.",
                    framework="agno",
                ),
            )
        return agent

    resolved_agent_id = _first_nonempty(
        agent_id,
        getattr(agent, "agent_id", None),
        getattr(agent, "id", None),
        agent.name,
    )
    if resolved_agent_id is None:
        raise ValueError(
            "agent_id is required when the Agno agent has no id or name"
        )
    warned: set[str] = set()
    async_mode = isinstance(client, AsyncClient)
    schemas = _collect_schemas(
        agent.tools,
        async_mode=async_mode,
        framework="agno",
        warned=warned,
        on_warning=on_warning,
        runtime=False,
    )
    config = _AgnoConfig(
        client=client,
        agent_id=resolved_agent_id,
        agent_name=agent.name,
        schemas=schemas,
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
        warned=warned,
    )
    if async_mode:
        adapter: _SyncAgnoAdapter | _AsyncAgnoAdapter = _AsyncAgnoAdapter(
            config
        )
    else:
        adapter = _SyncAgnoAdapter(config)
    agent.tool_hooks = [adapter.tool_hook, *(agent.tool_hooks or [])]
    agent.post_hooks = [*(agent.post_hooks or []), adapter.output_hook]
    _GUARDED_AGENTS.add(agent)
    return agent


def _collect_schemas(
    tools: Any,
    *,
    async_mode: bool,
    framework: Framework,
    warned: set[str],
    on_warning: Callable[[AdapterWarning], None] | None,
    runtime: bool,
) -> dict[str, Mapping[str, Any]]:
    if tools is None:
        return {}
    if callable(tools) and not isinstance(tools, Function):
        warn_once(
            AdapterWarningCode.dynamic_tool_unavailable,
            message=(
                "Agno tools are resolved by a callable; schemas will be read "
                "from public run context when available."
            ),
            framework=framework,
            warned=warned,
            on_warning=on_warning,
        )
        return {}
    if not isinstance(tools, Sequence) or isinstance(
        tools, (str, bytes, bytearray)
    ):
        return {}

    schemas: dict[str, Mapping[str, Any]] = {}
    for tool in tools:
        if isinstance(tool, Function):
            schemas[tool.name] = copy.deepcopy(tool.parameters)
        elif isinstance(tool, Toolkit):
            functions = (
                tool.get_async_functions()
                if async_mode
                else tool.get_functions()
            )
            for name, function in functions.items():
                schemas[name] = copy.deepcopy(function.parameters)
        elif isinstance(tool, dict):
            warn_once(
                AdapterWarningCode.provider_hosted_tool_unavailable,
                message=(
                    "A provider-hosted dictionary tool has no local function "
                    "boundary for TrustLoopGuard to intercept."
                ),
                framework=framework,
                warned=warned,
                on_warning=on_warning,
                key=f"provider-hosted:{id(tool)}",
            )
        elif callable(tool):
            try:
                function = Function.from_callable(tool)
            except Exception:
                continue
            schemas[function.name] = copy.deepcopy(function.parameters)
    if runtime and not schemas:
        warn_once(
            AdapterWarningCode.dynamic_tool_unavailable,
            message=(
                "The runtime tool registry did not expose a matching local "
                "function schema."
            ),
            framework=framework,
            warned=warned,
            on_warning=on_warning,
        )
    return schemas


def _principal(agent_id: str, run_context: RunContext) -> Principal:
    return Principal(
        workspace_id="",
        environment_id="",
        agent_id=agent_id,
        session_id=_nonempty(getattr(run_context, "session_id", None)),
        user_id=_nonempty(getattr(run_context, "user_id", None)),
    )


def _context(
    config: _AgnoConfig,
    agent: Agent,
    run_context: RunContext,
) -> dict[str, Any]:
    return copied_context(
        config.context,
        framework="agno",
        framework_agent_name=agent.name or config.agent_name,
        framework_run_id=_nonempty(getattr(run_context, "run_id", None)),
        framework_session_id=_nonempty(
            getattr(run_context, "session_id", None)
        ),
        framework_user_id=_nonempty(getattr(run_context, "user_id", None)),
    )


def _run_input_text(run_output: RunOutput) -> str:
    if run_output.input is None:
        return ""
    return run_output.input.input_content_string()


def _nonempty(value: Any) -> str | None:
    return value if isinstance(value, str) and value.strip() else None


def _first_nonempty(*values: Any) -> str | None:
    for value in values:
        resolved = _nonempty(value)
        if resolved is not None:
            return resolved
    return None


def _warn_completion_failure(
    config: _AgnoConfig,
    function_name: str,
    invocation_id: str,
) -> None:
    warn_once(
        AdapterWarningCode.lease_completion_failed_after_execution,
        message=(
            "The tool executed, but TrustLoopGuard could not report lease "
            "completion. The tool will not be retried."
        ),
        framework="agno",
        warned=config.warned,
        on_warning=config.on_warning,
        tool_name=function_name,
        key=f"lease-completion:{function_name}:{invocation_id}",
    )


def _emit_tool_log(
    config: _AgnoConfig,
    *,
    trace_id: str,
    effect: str,
    executed: bool,
    function_name: str,
    invocation_id: str,
    start: float,
) -> None:
    emit_log(
        config.log,
        AdapterLogEvent(
            framework="agno",
            boundary="tool",
            agent_id=config.agent_id,
            trace_id=trace_id,
            effect=effect,
            executed=executed,
            latency_ms=elapsed_ms(start),
            tool_name=function_name,
            invocation_id=invocation_id,
        ),
    )


def _emit_output_log(
    config: _AgnoConfig,
    event_log: GuardLogEvent,
) -> None:
    emit_log(
        config.log,
        AdapterLogEvent(
            framework="agno",
            boundary="output",
            agent_id=config.agent_id,
            trace_id=event_log.trace_id,
            effect=(
                "error"
                if event_log.branch == "error"
                else event_log.effect
            ),
            executed=event_log.branch in {"permit", "revise"},
            latency_ms=event_log.latency_ms,
        ),
    )


__all__ = ["guard_agno"]
