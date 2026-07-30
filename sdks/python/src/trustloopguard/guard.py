"""Output-boundary helpers for the Python SDK.

The shortest integration decorates the function that produces the final reply::

    @trustloopguard.guarded(agent_id="acme-support-v3")
    async def answer(message: str) -> str:
        return await agent.reply(message)

The explicit guard factory remains available when the caller already has
separate input and draft values::

    guardrail = trustloopguard.guard(agent_id="acme-support-v3")
    reply = await guardrail(input=user_message, draft=agent_draft)

The lower-level sync (``guard`` with ``client=...``) and async
(``guard_async``) variants remain available for custom client ownership.

Low-level effect → callback mapping::

    permit           → on_allow ?? draft
    transform        → on_revise ?? (decision.transformed_value or draft)
    deny             → on_block ?? default safe message
    require_approval → on_require_approval ?? default holding message
    defer            → on_defer ?? default retry-later message

Factory-mode presets::

    strict                → treat transform effects as denied output
    rewrite               → use transformed output, deny when none exists
    rewrite_or_regenerate → use transformed output, otherwise regenerate and check again

Transport / decode / retry-exhausted errors route to ``on_error`` and fail
closed with the SDK safe message by default. Set ``fail_closed=False`` only
when returning the unchecked draft during an outage is an explicit choice.

Low-level example::

    from trustloopguard import Client, guard

    client = Client(base_url="https://api.trustloopguard.dev",
                    api_key=os.environ["TLG_API_KEY"])

    reply = guard(
        client=client,
        agent_id="acme-support-v3",
        input=user_message,
        draft=agent_draft,
        on_block=lambda _: "I can't help with that.",
        on_require_approval=lambda _: human_queue_push_then_hold(),
        on_defer=lambda _: "I need more verified information before continuing.",
    )
    send_to_customer(reply)
"""

from __future__ import annotations

import functools
import inspect
import logging
import os
import time
from collections.abc import AsyncIterable, Iterable
from dataclasses import dataclass
from enum import Enum
from typing import Any, Awaitable, Callable, Literal, Optional, ParamSpec, Union, overload

from trustloopguard._generated.types import (
    Action,
    Channel,
    AuthorizationDecision,
    EventKind,
    GuardEvent,
    Labels,
    Origin,
    Principal,
    ProvenanceMap,
    SideEffectClass,
    Source,
    AuthorizationEffect,
)
from trustloopguard.client import AsyncClient, Client
from trustloopguard.errors import SdkError
from trustloopguard.retry import RetryConfig

_logger = logging.getLogger("trustloopguard")
P = ParamSpec("P")

# -- Sync callback signatures ----------------------------------------------

OnAllowSync = Callable[[str, AuthorizationDecision], str]
OnReviseSync = Callable[[Optional[str], str, AuthorizationDecision], str]
OnBlockSync = Callable[[AuthorizationDecision], str]
OnRequireApprovalSync = Callable[[AuthorizationDecision], str]
OnDeferSync = Callable[[AuthorizationDecision], str]
OnErrorSync = Callable[[SdkError, str], str]

# -- Async callback signatures ---------------------------------------------

OnAllowAsync = Callable[[str, AuthorizationDecision], Awaitable[str]]
OnReviseAsync = Callable[[Optional[str], str, AuthorizationDecision], Awaitable[str]]
OnBlockAsync = Callable[[AuthorizationDecision], Awaitable[str]]
OnRequireApprovalAsync = Callable[[AuthorizationDecision], Awaitable[str]]
OnDeferAsync = Callable[[AuthorizationDecision], Awaitable[str]]
OnErrorAsync = Callable[[SdkError, str], Awaitable[str]]

DecisionHandler = Callable[[AuthorizationDecision], Union[str, Awaitable[str]]]
ErrorHandler = Callable[[SdkError, str], Union[str, Awaitable[str]]]
GuardModeValue = Literal["strict", "rewrite", "rewrite_or_regenerate"]
GuardModeInput = Union["GuardMode", GuardModeValue]
RegenerateHandler = Callable[["RegenerateFeedback"], Union[str, Awaitable[str]]]

DEFAULT_BLOCK_MESSAGE = "I can't help with that request."
DEFAULT_REQUIRE_APPROVAL_MESSAGE = "A human teammate should review this before we continue."
DEFAULT_DEFER_MESSAGE = "Required evidence or system state is unavailable. Please try again later."


class GuardMode(str, Enum):
    """High-level output handling preset for ``trustloopguard.guard``."""

    STRICT = "strict"
    REWRITE = "rewrite"
    REWRITE_OR_REGENERATE = "rewrite_or_regenerate"


@dataclass(frozen=True)
class RegenerateFeedback:
    """Context passed to a model-regeneration callback."""

    input: str
    draft: str
    decision: AuthorizationDecision
    reason: str
    safe_output: str | None
    attempt: int
    max_attempts: int


@dataclass
class GuardLogEvent:
    """Structured event emitted once per ``guard`` invocation."""

    trace_id: str
    effect: str
    branch: Literal["permit", "revise", "deny", "require_approval", "defer", "error"]
    latency_ms: int


class OutputGuard:
    """Async callable returned by ``trustloopguard.guard(agent_id=...)``.

    It owns the SDK client by default, reads the usual TrustLoopGuard env vars,
    and applies safe deny/approval/defer defaults. Most integrations should create
    one guard at startup and call it at the output boundary:

    ``safe_reply = await guard(input=user_text, draft=agent_draft)``
    """

    def __init__(
        self,
        *,
        agent_id: str,
        client: AsyncClient | None = None,
        base_url: str | None = None,
        api_key: str | None = None,
        timeout: float = 5.0,
        retry: RetryConfig | None = None,
        channel: Channel | None = None,
        domain: str | None = None,
        context: dict[str, Any] | None = None,
        on_block: DecisionHandler | str | None = None,
        on_require_approval: DecisionHandler | str | None = None,
        on_defer: DecisionHandler | str | None = None,
        on_error: ErrorHandler | str | None = None,
        mode: GuardModeInput = GuardMode.REWRITE,
        regenerate: RegenerateHandler | None = None,
        max_regenerations: int = 1,
        fail_closed: bool = True,
        log: Callable[[GuardLogEvent], None] | None = None,
    ) -> None:
        self.agent_id = agent_id
        self.channel = channel
        self.domain = domain
        self.context = context or {}
        self.on_block = on_block
        self.on_require_approval = on_require_approval
        self.on_defer = on_defer
        self.on_error = on_error
        self.mode = _normalize_mode(mode)
        self.regenerate = regenerate
        self.max_regenerations = max_regenerations
        self.fail_closed = fail_closed
        self.log = log

        if client is not None:
            self.client = client
            self._owns_client = False
            return

        resolved_base_url = (
            base_url
            or _env("TLG_URL", "TL_SERVER_URL", "TRUSTLOOPGUARD_URL", "TRUSTLOOP_URL")
            or "http://127.0.0.1:8080"
        )
        self.client = AsyncClient(
            base_url=resolved_base_url,
            api_key=api_key
            or _env(
                "TLG_API_KEY",
                "TL_API_KEY",
                "TRUSTLOOPGUARD_API_KEY",
                "TRUSTLOOP_API_KEY",
            ),
            timeout=timeout,
            retry=retry,
        )
        self._owns_client = True

    async def __call__(
        self,
        *,
        input: str,  # noqa: A002
        draft: str,
        channel: Channel | None = None,
        domain: str | None = None,
        context: dict[str, Any] | None = None,
        trace_id: str | None = None,
        on_block: DecisionHandler | str | None = None,
        on_require_approval: DecisionHandler | str | None = None,
        on_defer: DecisionHandler | str | None = None,
        on_error: ErrorHandler | str | None = None,
        mode: GuardModeInput | None = None,
        regenerate: RegenerateHandler | None = None,
        max_regenerations: int | None = None,
        log: Callable[[GuardLogEvent], None] | None = None,
    ) -> str:
        selected_mode = _normalize_mode(mode) if mode is not None else self.mode
        selected_regenerate = regenerate or self.regenerate
        selected_max_regenerations = (
            self.max_regenerations if max_regenerations is None else max_regenerations
        )
        block_handler = _decision_handler(
            on_block if on_block is not None else self.on_block,
            DEFAULT_BLOCK_MESSAGE,
        )
        require_approval_handler = _decision_handler(
            on_require_approval if on_require_approval is not None else self.on_require_approval,
            DEFAULT_REQUIRE_APPROVAL_MESSAGE,
        )
        defer_handler = _decision_handler(
            on_defer if on_defer is not None else self.on_defer,
            DEFAULT_DEFER_MESSAGE,
        )
        error_handler = _error_handler(
            on_error if on_error is not None else self.on_error,
            DEFAULT_BLOCK_MESSAGE if self.fail_closed else None,
        )
        selected_log = log if log is not None else self.log

        async def run_attempt(current_draft: str, completed_regenerations: int) -> str:
            async def on_revise(
                revised: str | None,
                checked_draft: str,
                decision: AuthorizationDecision,
            ) -> str:
                if selected_mode == GuardMode.STRICT:
                    return await block_handler(decision)

                if revised is not None:
                    return revised

                if (
                    selected_mode != GuardMode.REWRITE_OR_REGENERATE
                    or selected_regenerate is None
                    or completed_regenerations >= selected_max_regenerations
                ):
                    return await block_handler(decision)

                next_attempt = completed_regenerations + 1
                feedback = RegenerateFeedback(
                    input=input,
                    draft=checked_draft,
                    decision=decision,
                    reason=decision.reason,
                    safe_output=(
                        decision.transformed_value
                        if isinstance(decision.transformed_value, str)
                        else None
                    ),
                    attempt=next_attempt,
                    max_attempts=selected_max_regenerations,
                )
                next_draft = selected_regenerate(feedback)
                if not isinstance(next_draft, str):
                    next_draft = await next_draft
                return await run_attempt(next_draft, next_attempt)

            return await guard_async(
                client=self.client,
                agent_id=self.agent_id,
                input=input,
                draft=current_draft,
                channel=channel or self.channel,
                domain=domain if domain is not None else self.domain,
                context={**self.context, **(context or {})},
                trace_id=trace_id,
                on_block=block_handler,
                on_require_approval=require_approval_handler,
                on_defer=defer_handler,
                on_revise=on_revise,
                on_error=error_handler,
                fail_closed=self.fail_closed,
                log=selected_log,
            )

        return await run_attempt(draft, 0)

    async def stream(
        self,
        *,
        input: str,  # noqa: A002
        draft: AsyncIterable[str] | Iterable[str],
        channel: Channel | None = None,
        domain: str | None = None,
        context: dict[str, Any] | None = None,
        trace_id: str | None = None,
        on_block: DecisionHandler | str | None = None,
        on_require_approval: DecisionHandler | str | None = None,
        on_defer: DecisionHandler | str | None = None,
        on_error: ErrorHandler | str | None = None,
        mode: GuardModeInput | None = None,
        regenerate: RegenerateHandler | None = None,
        max_regenerations: int | None = None,
        log: Callable[[GuardLogEvent], None] | None = None,
    ) -> str:
        """Buffer a draft chunk stream, then guard the complete output.

        ``draft`` is an async or sync iterable of output chunks (e.g. an LLM
        token stream). The stream is consumed in full, then guarded by the same
        path as :meth:`__call__` — no unguarded chunk is ever returned, mirroring
        the gateway's buffered-then-emit model. Returns the guarded string.
        """
        buffered = await _collect_chunks(draft)
        return await self(
            input=input,
            draft=buffered,
            channel=channel,
            domain=domain,
            context=context,
            trace_id=trace_id,
            on_block=on_block,
            on_require_approval=on_require_approval,
            on_defer=on_defer,
            on_error=on_error,
            mode=mode,
            regenerate=regenerate,
            max_regenerations=max_regenerations,
            log=log,
        )

    async def aclose(self) -> None:
        if self._owns_client:
            await self.client.aclose()

    async def __aenter__(self) -> "OutputGuard":
        return self

    async def __aexit__(self, *_: Any) -> None:
        await self.aclose()


async def _collect_chunks(chunks: AsyncIterable[str] | Iterable[str]) -> str:
    """Join an async or sync iterable of string chunks into one string."""
    if isinstance(chunks, AsyncIterable):
        parts = [chunk async for chunk in chunks]
        return "".join(parts)
    return "".join(chunks)


def _build_event(
    *,
    agent_id: str,
    input: str,
    draft: str,
    channel: Channel | None,
    domain: str | None,
    context: dict[str, Any] | None,
    trace_id: str | None,
    run_id: str | None = None,
    run_event_id: str | None = None,
) -> GuardEvent:
    event_context: dict[str, Any] = {
        **(context or {}),
        "channel": (channel or Channel.chat).value,
        "domain": domain or "customer_support",
    }
    return GuardEvent(
        kind=EventKind.output_proposed,
        principal=Principal(
            workspace_id="",
            environment_id="",
            agent_id=agent_id,
            run_id=run_id,
            run_event_id=run_event_id,
        ),
        action=Action(
            operation="output",
            parameters={"text": draft},
            side_effect=SideEffectClass.none,
        ),
        sources=[
            Source(
                id="input",
                origin=Origin.user,
                labels=Labels(),
            )
        ],
        provenance=ProvenanceMap({"text": ["input"]}),
        context=event_context,
    )


def _branch_for(
    effect: str,
) -> Literal["permit", "revise", "deny", "require_approval", "defer"]:
    if effect == "transform":
        return "revise"
    if effect in {"permit", "deny", "require_approval", "defer"}:
        return effect
    raise ValueError(f"unknown authorization effect: {effect}")


def _env(*names: str) -> str | None:
    for name in names:
        value = os.getenv(name)
        if value:
            return value
    return None


def _normalize_mode(mode: GuardModeInput) -> GuardMode:
    if isinstance(mode, GuardMode):
        return mode
    return GuardMode(mode)


def _decision_handler(
    handler: DecisionHandler | str | None,
    default_message: str,
) -> OnBlockAsync:
    async def resolved(decision: AuthorizationDecision) -> str:
        if handler is None:
            return default_message
        if isinstance(handler, str):
            return handler
        result = handler(decision)
        if isinstance(result, str):
            return result
        return await result

    return resolved


def _error_handler(
    handler: ErrorHandler | str | None,
    default_message: str | None,
) -> OnErrorAsync | None:
    if handler is None and default_message is None:
        return None

    async def resolved(err: SdkError, draft: str) -> str:
        if handler is None:
            return default_message if default_message is not None else draft
        if isinstance(handler, str):
            return handler
        result = handler(err, draft)
        if isinstance(result, str):
            return result
        return await result

    return resolved


# -- Decorator -------------------------------------------------------------


def guarded(
    *,
    agent_id: str,
    input_arg: str | None = None,
    client: AsyncClient | None = None,
    base_url: str | None = None,
    api_key: str | None = None,
    timeout: float = 5.0,
    retry: RetryConfig | None = None,
    channel: Channel | None = None,
    domain: str | None = None,
    context: dict[str, Any] | None = None,
    on_block: DecisionHandler | str | None = None,
    on_require_approval: DecisionHandler | str | None = None,
    on_defer: DecisionHandler | str | None = None,
    on_error: ErrorHandler | str | None = None,
    mode: GuardModeInput = GuardMode.REWRITE,
    regenerate: RegenerateHandler | None = None,
    max_regenerations: int = 1,
    fail_closed: bool = True,
    log: Callable[[GuardLogEvent], None] | None = None,
) -> Callable[[Callable[P, Awaitable[str]]], Callable[P, Awaitable[str]]]:
    """Guard the string returned by an async agent function.

    The first parameter other than ``self`` or ``cls`` is treated as the user
    input. Pass ``input_arg`` when the function has more than one meaningful
    argument. The decorated function must receive a string input and return a
    string draft.

    Decorators and standalone guards fail closed on SDK transport errors by
    default. Set ``fail_closed=False`` only for an explicit availability-first
    integration.
    """

    def decorate(func: Callable[P, Awaitable[str]]) -> Callable[P, Awaitable[str]]:
        signature, selected_input_arg = _decorated_input(func, input_arg)
        if not inspect.iscoroutinefunction(func):
            raise TypeError("guarded() requires an async function")

        output_guard = OutputGuard(
            agent_id=agent_id,
            client=client,
            base_url=base_url,
            api_key=api_key,
            timeout=timeout,
            retry=retry,
            channel=channel,
            domain=domain,
            context=context,
            on_block=on_block,
            on_require_approval=on_require_approval,
            on_defer=on_defer,
            on_error=on_error,
            mode=mode,
            regenerate=regenerate,
            max_regenerations=max_regenerations,
            fail_closed=fail_closed,
            log=log,
        )

        @functools.wraps(func)
        async def async_wrapper(*args: P.args, **kwargs: P.kwargs) -> str:
            input_value = _decorated_input_value(
                signature,
                selected_input_arg,
                args,
                kwargs,
            )
            draft = await func(*args, **kwargs)
            if not isinstance(draft, str):
                raise TypeError(f"guarded function '{func.__name__}' return value must be str")
            return await output_guard(input=input_value, draft=draft)

        setattr(async_wrapper, "aclose", output_guard.aclose)
        return async_wrapper

    return decorate


def _decorated_input(
    func: Callable[..., Any],
    input_arg: str | None,
) -> tuple[inspect.Signature, str]:
    signature = inspect.signature(func)
    if input_arg is not None:
        parameter = signature.parameters.get(input_arg)
        if parameter is None or parameter.kind in {
            inspect.Parameter.VAR_POSITIONAL,
            inspect.Parameter.VAR_KEYWORD,
        }:
            raise TypeError(
                f"guarded function '{func.__name__}' has no input parameter '{input_arg}'"
            )
        return signature, input_arg

    for parameter in signature.parameters.values():
        if parameter.name in {"self", "cls"}:
            continue
        if parameter.kind in {
            inspect.Parameter.POSITIONAL_ONLY,
            inspect.Parameter.POSITIONAL_OR_KEYWORD,
            inspect.Parameter.KEYWORD_ONLY,
        }:
            return signature, parameter.name

    raise TypeError(
        f"guarded function '{func.__name__}' requires a string input parameter"
    )


def _decorated_input_value(
    signature: inspect.Signature,
    input_arg: str,
    args: tuple[Any, ...],
    kwargs: dict[str, Any],
) -> str:
    bound = signature.bind(*args, **kwargs)
    bound.apply_defaults()
    value = bound.arguments[input_arg]
    if not isinstance(value, str):
        raise TypeError(f"guarded input argument '{input_arg}' must be str")
    return value


# -- Sync / factory --------------------------------------------------------


@overload
def guard(
    *,
    agent_id: str,
    client: None = None,
    base_url: str | None = None,
    api_key: str | None = None,
    timeout: float = 5.0,
    retry: RetryConfig | None = None,
    channel: Channel | None = None,
    domain: str | None = None,
    context: dict[str, Any] | None = None,
    on_block: DecisionHandler | str | None = None,
    on_require_approval: DecisionHandler | str | None = None,
    on_defer: DecisionHandler | str | None = None,
    on_error: ErrorHandler | str | None = None,
    mode: GuardModeInput = GuardMode.REWRITE,
    regenerate: RegenerateHandler | None = None,
    max_regenerations: int = 1,
    fail_closed: bool = True,
    log: Callable[[GuardLogEvent], None] | None = None,
) -> OutputGuard: ...


@overload
def guard(
    *,
    client: Client,
    agent_id: str,
    input: str,  # noqa: A002 — matches the wire field name
    draft: str,
    on_block: OnBlockSync,
    on_require_approval: OnRequireApprovalSync,
    on_defer: OnDeferSync | None = None,
    on_allow: OnAllowSync | None = None,
    on_revise: OnReviseSync | None = None,
    on_error: OnErrorSync | None = None,
    channel: Channel | None = None,
    domain: str | None = None,
    context: dict[str, Any] | None = None,
    trace_id: str | None = None,
    log: Callable[[GuardLogEvent], None] | None = None,
    run_id: str | None = None,
    run_event_id: str | None = None,
    fail_closed: bool = True,
) -> str: ...


def guard(
    *,
    agent_id: str,
    client: Client | None = None,
    input: str | None = None,  # noqa: A002 — matches the wire field name
    draft: str | None = None,
    on_block: OnBlockSync | DecisionHandler | str | None = None,
    on_require_approval: OnRequireApprovalSync | DecisionHandler | str | None = None,
    on_defer: OnDeferSync | DecisionHandler | str | None = None,
    on_allow: OnAllowSync | None = None,
    on_revise: OnReviseSync | None = None,
    on_error: OnErrorSync | ErrorHandler | str | None = None,
    mode: GuardModeInput = GuardMode.REWRITE,
    regenerate: RegenerateHandler | None = None,
    max_regenerations: int = 1,
    channel: Channel | None = None,
    domain: str | None = None,
    context: dict[str, Any] | None = None,
    trace_id: str | None = None,
    log: Callable[[GuardLogEvent], None] | None = None,
    run_id: str | None = None,
    run_event_id: str | None = None,
    base_url: str | None = None,
    api_key: str | None = None,
    timeout: float = 5.0,
    retry: RetryConfig | None = None,
    fail_closed: bool = True,
) -> str | OutputGuard:
    """Create a simple async guard or run the legacy sync guard.

    New integrations should use the factory form:

        guardrail = trustloopguard.guard(agent_id="support-agent")
        reply = await guardrail(input=user_text, draft=agent_draft)

    The existing sync form remains supported when ``client``, ``input``,
    and ``draft`` are supplied.
    """
    if client is None:
        if input is not None or draft is not None:
            raise TypeError(
                "trustloopguard.guard(...) without client returns a guard; "
                "call the returned guard with input=... and draft=..."
            )
        return OutputGuard(
            agent_id=agent_id,
            base_url=base_url,
            api_key=api_key,
            timeout=timeout,
            retry=retry,
            channel=channel,
            domain=domain,
            context=context,
            on_block=on_block,
            on_require_approval=on_require_approval,
            on_defer=on_defer,
            on_error=on_error,
            mode=mode,
            regenerate=regenerate,
            max_regenerations=max_regenerations,
            fail_closed=fail_closed,
            log=log,
        )

    if input is None or draft is None:
        raise TypeError("client guard requires input=... and draft=...")
    if not callable(on_block) or not callable(on_require_approval):
        raise TypeError("client guard requires callable on_block and on_require_approval")
    resolved_on_defer: OnDeferSync = (
        on_defer if callable(on_defer) else lambda _decision: DEFAULT_DEFER_MESSAGE
    )

    return _guard_sync(
        client=client,
        agent_id=agent_id,
        input=input,
        draft=draft,
        on_block=on_block,
        on_require_approval=on_require_approval,
        on_defer=resolved_on_defer,
        on_allow=on_allow,
        on_revise=on_revise,
        on_error=on_error if callable(on_error) else None,
        fail_closed=fail_closed,
        channel=channel,
        domain=domain,
        context=context,
        trace_id=trace_id,
        run_id=run_id,
        run_event_id=run_event_id,
        log=log,
    )


def _guard_sync(
    *,
    client: Client,
    agent_id: str,
    input: str,  # noqa: A002 — matches the wire field name
    draft: str,
    on_block: OnBlockSync,
    on_require_approval: OnRequireApprovalSync,
    on_defer: OnDeferSync,
    on_allow: OnAllowSync | None = None,
    on_revise: OnReviseSync | None = None,
    on_error: OnErrorSync | None = None,
    fail_closed: bool = True,
    channel: Channel | None = None,
    domain: str | None = None,
    context: dict[str, Any] | None = None,
    trace_id: str | None = None,
    run_id: str | None = None,
    run_event_id: str | None = None,
    log: Callable[[GuardLogEvent], None] | None = None,
) -> str:
    """Run a sync check and dispatch the appropriate callback. Returns
    the string the caller should actually send to the customer.

    See module docstring for the full effect-to-callback table.
    """
    start = time.monotonic()
    event = _build_event(
        agent_id=agent_id,
        input=input,
        draft=draft,
        channel=channel,
        domain=domain,
        context=context,
        trace_id=trace_id,
        run_id=run_id,
        run_event_id=run_event_id,
    )

    try:
        decision = client.submit_event(event)
    except SdkError as e:
        if on_error:
            result = on_error(e, draft)
        else:
            result = DEFAULT_BLOCK_MESSAGE if fail_closed else draft
        _emit_log(log, trace_id or "", "permit", "error", start)
        return result

    if decision.effect == AuthorizationEffect.permit:
        result = on_allow(draft, decision) if on_allow else draft
    elif decision.effect == AuthorizationEffect.transform:
        revised = decision.transformed_value if isinstance(decision.transformed_value, str) else None
        if on_revise:
            result = on_revise(revised, draft, decision)
        else:
            result = revised if revised is not None else draft
    elif decision.effect == AuthorizationEffect.deny:
        result = on_block(decision)
    elif decision.effect == AuthorizationEffect.require_approval:
        result = on_require_approval(decision)
    elif decision.effect == AuthorizationEffect.defer:
        result = on_defer(decision)
    else:  # pragma: no cover — exhaustive over the effect enum
        raise RuntimeError(f"unknown effect: {decision.effect}")

    _emit_log(
        log,
        decision.trace_id,
        decision.effect.value,
        _branch_for(decision.effect.value),
        start,
    )
    return result


# -- Async -----------------------------------------------------------------


async def guard_async(
    *,
    client: AsyncClient,
    agent_id: str,
    input: str,  # noqa: A002
    draft: str,
    on_block: OnBlockAsync,
    on_require_approval: OnRequireApprovalAsync,
    on_defer: OnDeferAsync | None = None,
    on_allow: OnAllowAsync | None = None,
    on_revise: OnReviseAsync | None = None,
    on_error: OnErrorAsync | None = None,
    fail_closed: bool = True,
    channel: Channel | None = None,
    domain: str | None = None,
    context: dict[str, Any] | None = None,
    trace_id: str | None = None,
    run_id: str | None = None,
    run_event_id: str | None = None,
    log: Callable[[GuardLogEvent], None] | None = None,
) -> str:
    """Async sibling of ``guard``. All callbacks must be coroutines."""
    start = time.monotonic()
    event = _build_event(
        agent_id=agent_id,
        input=input,
        draft=draft,
        channel=channel,
        domain=domain,
        context=context,
        trace_id=trace_id,
        run_id=run_id,
        run_event_id=run_event_id,
    )

    try:
        decision = await client.submit_event(event)
    except SdkError as e:
        if on_error:
            result = await on_error(e, draft)
        else:
            result = DEFAULT_BLOCK_MESSAGE if fail_closed else draft
        _emit_log(log, trace_id or "", "permit", "error", start)
        return result

    if decision.effect == AuthorizationEffect.permit:
        result = await on_allow(draft, decision) if on_allow else draft
    elif decision.effect == AuthorizationEffect.transform:
        revised = decision.transformed_value if isinstance(decision.transformed_value, str) else None
        if on_revise:
            result = await on_revise(revised, draft, decision)
        else:
            result = revised if revised is not None else draft
    elif decision.effect == AuthorizationEffect.deny:
        result = await on_block(decision)
    elif decision.effect == AuthorizationEffect.require_approval:
        result = await on_require_approval(decision)
    elif decision.effect == AuthorizationEffect.defer:
        result = await on_defer(decision) if on_defer else DEFAULT_DEFER_MESSAGE
    else:  # pragma: no cover
        raise RuntimeError(f"unknown effect: {decision.effect}")

    _emit_log(
        log,
        decision.trace_id,
        decision.effect.value,
        _branch_for(decision.effect.value),
        start,
    )
    return result


# -- Internals -------------------------------------------------------------


def _emit_log(
    log: Callable[[GuardLogEvent], None] | None,
    trace_id: str,
    effect: str,
    branch: Literal["permit", "revise", "deny", "require_approval", "defer", "error"],
    start: float,
) -> None:
    if log is None:
        return
    elapsed_ms = int((time.monotonic() - start) * 1000)
    log(
        GuardLogEvent(
            trace_id=trace_id,
            effect=effect,
            branch=branch,
            latency_ms=elapsed_ms,
        )
    )
