"""``guard()`` — one-line integration helper for the Python SDK.

Mirrors the TypeScript helper. Sync (``guard``) + async (``guard_async``)
variants share the same callback shape.

Verdict → callback mapping::

    allow    → on_allow ?? draft
    rewrite  → on_revise ?? (decision.safe_output or draft)
    block    → on_block(decision)              (required)
    escalate → on_escalate(decision)           (required)

Transport / decode / retry-exhausted errors route to ``on_error``,
**default fail-open** (return original draft). Pass an explicit
handler for fail-closed behaviour.

Example::

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
import time
from dataclasses import dataclass
from typing import Any, Awaitable, Callable, Literal

from trustloopguard._generated.types import (
    Channel,
    CheckRequest,
    Decision,
    Verdict,
)
from trustloopguard.client import AsyncClient, Client
from trustloopguard.errors import SdkError

_logger = logging.getLogger("trustloopguard")

# -- Sync callback signatures ----------------------------------------------

OnAllowSync = Callable[[str, Decision], str]
OnReviseSync = Callable[[str | None, str, Decision], str]
OnBlockSync = Callable[[Decision], str]
OnEscalateSync = Callable[[Decision], str]
OnErrorSync = Callable[[SdkError, str], str]

# -- Async callback signatures ---------------------------------------------

OnAllowAsync = Callable[[str, Decision], Awaitable[str]]
OnReviseAsync = Callable[[str | None, str, Decision], Awaitable[str]]
OnBlockAsync = Callable[[Decision], Awaitable[str]]
OnEscalateAsync = Callable[[Decision], Awaitable[str]]
OnErrorAsync = Callable[[SdkError, str], Awaitable[str]]


@dataclass
class GuardLogEvent:
    """Structured event emitted once per ``guard`` invocation."""

    trace_id: str
    verdict: str
    branch: Literal["allow", "revise", "block", "escalate", "error"]
    latency_ms: int


def _build_request(
    *,
    agent_id: str,
    input: str,
    draft: str,
    channel: Channel | None,
    domain: str | None,
    context: dict[str, Any] | None,
    trace_id: str | None,
) -> CheckRequest:
    return CheckRequest(
        agent_id=agent_id,
        channel=channel or Channel.chat,
        input=input,
        proposed_output=draft,
        domain=domain,
        policies=[],
        context=context or {},
        trace_id=trace_id,
    )


def _branch_for(verdict: str) -> Literal["allow", "revise", "block", "escalate"]:
    if verdict == "rewrite":
        return "revise"
    return verdict  # type: ignore[return-value]


# -- Sync ------------------------------------------------------------------


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
) -> str:
    """Run a check and dispatch the appropriate callback. Returns the
    string the caller should actually send to the customer.

    See module docstring for the full verdict-to-callback table.
    """
    start = time.monotonic()
    req = _build_request(
        agent_id=agent_id,
        input=input,
        draft=draft,
        channel=channel,
        domain=domain,
        context=context,
        trace_id=trace_id,
    )

    try:
        decision = client.check(req)
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
    log: Callable[[GuardLogEvent], None] | None = None,
) -> str:
    """Async sibling of ``guard``. All callbacks must be coroutines."""
    start = time.monotonic()
    req = _build_request(
        agent_id=agent_id,
        input=input,
        draft=draft,
        channel=channel,
        domain=domain,
        context=context,
        trace_id=trace_id,
    )

    try:
        decision = await client.check(req)
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
