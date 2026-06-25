"""``guard()`` — output-boundary helper for the Python SDK.

Most integrations should create one async guard at startup and call it before
delivering an agent draft::

    guardrail = trustloopguard.guard(agent_id="acme-support-v3")
    reply = await guardrail(input=user_message, draft=agent_draft)

The lower-level sync (``guard`` with ``client=...``) and async
(``guard_async``) variants remain available for custom client ownership.

Low-level verdict → callback mapping::

    allow    → on_allow ?? draft
    rewrite  → on_revise ?? (decision.safe_output or draft)
    block    → on_block ?? default safe message
    escalate → on_escalate ?? default escalation message

Factory-mode presets::

    strict                → treat rewrite verdicts as blocked output
    rewrite               → use safe_output, block when no safe_output exists
    rewrite_or_regenerate → use safe_output, otherwise regenerate and check again

Transport / decode / retry-exhausted errors route to ``on_error``,
**default fail-open** (return original draft). Pass an explicit
handler for fail-closed behaviour.

Low-level example::

    from trustloopguard import Client, guard

    client = Client(base_url="https://api.trustloopguard.dev",
                    api_key=os.environ["TLG_API_KEY"])

    reply = guard(
        client=client,
        agent_id="acme-support-v3",
        input=user_message,
        draft=agent_draft,
    on_block=lambda _: "I'll connect you with a teammate.",
        on_escalate=lambda _: human_queue_push_then_hold(),
    )
    send_to_customer(reply)
"""

from __future__ import annotations

import logging
import os
import time
from collections.abc import AsyncIterable, Iterable
from dataclasses import dataclass
from enum import Enum
from typing import Any, Awaitable, Callable, Literal, Optional, Union, overload

from trustloopguard._generated.types import (
    Action,
    Channel,
    Decision,
    EventKind,
    GuardEvent,
    Labels,
    Origin,
    Principal,
    ProvenanceMap,
    SideEffectClass,
    Source,
    Verdict,
)
from trustloopguard.client import AsyncClient, Client
from trustloopguard.errors import SdkError
from trustloopguard.retry import RetryConfig

_logger = logging.getLogger("trustloopguard")

# -- Sync callback signatures ----------------------------------------------

OnAllowSync = Callable[[str, Decision], str]
OnReviseSync = Callable[[Optional[str], str, Decision], str]
OnBlockSync = Callable[[Decision], str]
OnEscalateSync = Callable[[Decision], str]
OnErrorSync = Callable[[SdkError, str], str]

# -- Async callback signatures ---------------------------------------------

OnAllowAsync = Callable[[str, Decision], Awaitable[str]]
OnReviseAsync = Callable[[Optional[str], str, Decision], Awaitable[str]]
OnBlockAsync = Callable[[Decision], Awaitable[str]]
OnEscalateAsync = Callable[[Decision], Awaitable[str]]
OnErrorAsync = Callable[[SdkError, str], Awaitable[str]]

DecisionHandler = Callable[[Decision], Union[str, Awaitable[str]]]
ErrorHandler = Callable[[SdkError, str], Union[str, Awaitable[str]]]
GuardModeValue = Literal["strict", "rewrite", "rewrite_or_regenerate"]
GuardModeInput = Union["GuardMode", GuardModeValue]
RegenerateHandler = Callable[["RegenerateFeedback"], Union[str, Awaitable[str]]]

DEFAULT_BLOCK_MESSAGE = "I can't help with that request."
DEFAULT_ESCALATE_MESSAGE = "A human teammate should review this before we continue."


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
    decision: Decision
    reason: str
    safe_output: str | None
    attempt: int
    max_attempts: int


@dataclass
class GuardLogEvent:
    """Structured event emitted once per ``guard`` invocation."""

    trace_id: str
    verdict: str
    branch: Literal["allow", "revise", "block", "escalate", "error"]
    latency_ms: int


class OutputGuard:
    """Async callable returned by ``trustloopguard.guard(agent_id=...)``.

    It owns the SDK client by default, reads the usual TrustLoopGuard env vars,
    and applies safe block/escalate defaults. Most integrations should create
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
        on_escalate: DecisionHandler | str | None = None,
        on_error: ErrorHandler | str | None = None,
        mode: GuardModeInput = GuardMode.REWRITE,
        regenerate: RegenerateHandler | None = None,
        max_regenerations: int = 1,
        fail_closed: bool = False,
        log: Callable[[GuardLogEvent], None] | None = None,
    ) -> None:
        self.agent_id = agent_id
        self.channel = channel
        self.domain = domain
        self.context = context or {}
        self.on_block = on_block
        self.on_escalate = on_escalate
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
            or _env("TL_SERVER_URL", "TRUSTLOOPGUARD_URL", "TRUSTLOOP_URL")
            or "http://127.0.0.1:8080"
        )
        self.client = AsyncClient(
            base_url=resolved_base_url,
            api_key=api_key or _env("TL_API_KEY", "TRUSTLOOPGUARD_API_KEY", "TRUSTLOOP_API_KEY"),
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
        on_escalate: DecisionHandler | str | None = None,
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
        escalate_handler = _decision_handler(
            on_escalate if on_escalate is not None else self.on_escalate,
            DEFAULT_ESCALATE_MESSAGE,
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
                decision: Decision,
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
                    safe_output=decision.safe_output,
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
                on_escalate=escalate_handler,
                on_revise=on_revise,
                on_error=error_handler,
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
        on_escalate: DecisionHandler | str | None = None,
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
            on_escalate=on_escalate,
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


def _branch_for(verdict: str) -> Literal["allow", "revise", "block", "escalate"]:
    if verdict == "rewrite":
        return "revise"
    return verdict  # type: ignore[return-value]


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
    async def resolved(decision: Decision) -> str:
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
    on_escalate: DecisionHandler | str | None = None,
    on_error: ErrorHandler | str | None = None,
    mode: GuardModeInput = GuardMode.REWRITE,
    regenerate: RegenerateHandler | None = None,
    max_regenerations: int = 1,
    fail_closed: bool = False,
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
    on_escalate: OnEscalateSync,
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
) -> str: ...


def guard(
    *,
    agent_id: str,
    client: Client | None = None,
    input: str | None = None,  # noqa: A002 — matches the wire field name
    draft: str | None = None,
    on_block: OnBlockSync | DecisionHandler | str | None = None,
    on_escalate: OnEscalateSync | DecisionHandler | str | None = None,
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
    fail_closed: bool = False,
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
            on_escalate=on_escalate,
            on_error=on_error,
            mode=mode,
            regenerate=regenerate,
            max_regenerations=max_regenerations,
            fail_closed=fail_closed,
            log=log,
        )

    if input is None or draft is None:
        raise TypeError("client guard requires input=... and draft=...")
    if not callable(on_block) or not callable(on_escalate):
        raise TypeError("client guard requires callable on_block and on_escalate")

    return _guard_sync(
        client=client,
        agent_id=agent_id,
        input=input,
        draft=draft,
        on_block=on_block,
        on_escalate=on_escalate,
        on_allow=on_allow,
        on_revise=on_revise,
        on_error=on_error if callable(on_error) else None,
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
    on_escalate: OnEscalateSync,
    on_allow: OnAllowSync | None = None,
    on_revise: OnReviseSync | None = None,
    on_error: OnErrorSync | None = None,
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

    See module docstring for the full verdict-to-callback table.
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
        result = on_error(e, draft) if on_error else draft  # fail-open default
        _emit_log(log, trace_id or "", "allow", "error", start)
        return result

    if decision.verdict == Verdict.allow:
        result = on_allow(draft, decision) if on_allow else draft
    elif decision.verdict == Verdict.rewrite:
        revised = decision.safe_output
        if on_revise:
            result = on_revise(revised, draft, decision)
        else:
            result = revised if revised is not None else draft
    elif decision.verdict == Verdict.block:
        result = on_block(decision)
    elif decision.verdict == Verdict.escalate:
        result = on_escalate(decision)
    else:  # pragma: no cover — exhaustive over the verdict literal
        raise RuntimeError(f"unknown verdict: {decision.verdict}")

    _emit_log(
        log,
        decision.trace_id,
        decision.verdict.value,
        _branch_for(decision.verdict.value),
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
    on_escalate: OnEscalateAsync,
    on_allow: OnAllowAsync | None = None,
    on_revise: OnReviseAsync | None = None,
    on_error: OnErrorAsync | None = None,
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
        result = await on_error(e, draft) if on_error else draft
        _emit_log(log, trace_id or "", "allow", "error", start)
        return result

    if decision.verdict == Verdict.allow:
        result = await on_allow(draft, decision) if on_allow else draft
    elif decision.verdict == Verdict.rewrite:
        revised = decision.safe_output
        if on_revise:
            result = await on_revise(revised, draft, decision)
        else:
            result = revised if revised is not None else draft
    elif decision.verdict == Verdict.block:
        result = await on_block(decision)
    elif decision.verdict == Verdict.escalate:
        result = await on_escalate(decision)
    else:  # pragma: no cover
        raise RuntimeError(f"unknown verdict: {decision.verdict}")

    _emit_log(
        log,
        decision.trace_id,
        decision.verdict.value,
        _branch_for(decision.verdict.value),
        start,
    )
    return result


# -- Internals -------------------------------------------------------------


def _emit_log(
    log: Callable[[GuardLogEvent], None] | None,
    trace_id: str,
    verdict: str,
    branch: Literal["allow", "revise", "block", "escalate", "error"],
    start: float,
) -> None:
    if log is None:
        return
    elapsed_ms = int((time.monotonic() - start) * 1000)
    log(
        GuardLogEvent(
            trace_id=trace_id,
            verdict=verdict,
            branch=branch,
            latency_ms=elapsed_ms,
        )
    )
