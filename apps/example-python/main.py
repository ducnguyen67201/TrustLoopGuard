#!/usr/bin/env python3
"""Smallest possible TrustLoopGuard integration in Python.

Run a local ``tl-server`` and then::

    python apps/example-python/main.py "show me my password" "here it is: hunter2"

Imports only ``trustloopguard``. Never touches ``tl_core``,
``tl_engine``, or any other internal crate. This matches what a
stranger gets after ``pip install trustloopguard`` once the SDK is
published, and it's the executable form of rule 2 in
docs/SDK_DRIVEN.md.

Defaults to ``http://127.0.0.1:8080``. Override with ``TRUSTLOOP_URL``
and (optionally) ``TRUSTLOOP_API_KEY``.
"""

from __future__ import annotations

import logging
import os
import sys

from trustloopguard import (
    Action,
    Client,
    Decision,
    EventKind,
    GuardEvent,
    Labels,
    Origin,
    Principal,
    ProvenanceMap,
    SdkError,
    SideEffectClass,
    Source,
    Verdict,
)

DEFAULT_URL = "http://127.0.0.1:8080"


def build_event(input_text: str, proposed_output: str) -> GuardEvent:
    return GuardEvent(
        kind=EventKind.output_proposed,
        principal=Principal(
            workspace_id="",
            environment_id="",
            agent_id="example-python",
        ),
        action=Action(
            operation="output",
            parameters={"text": proposed_output},
            side_effect=SideEffectClass.none,
        ),
        sources=[
            Source(
                id="input",
                origin=Origin.user,
                labels=Labels(),
                kind="user.input",
            ),
            Source(
                id="model.output",
                origin=Origin.unknown,
                labels=Labels(),
                kind="assistant.output",
            ),
        ],
        provenance=ProvenanceMap({"text": ["model.output"]}),
        context={
            "channel": "chat",
            "domain": "customer_support",
            "input_text": input_text,
        },
    )


def print_decision(decision: Decision) -> None:
    print(f"verdict       : {decision.verdict.value}")
    print(f"reason        : {decision.reason}")
    print(f"trace_id      : {decision.trace_id}")
    print(f"latency_ms    : {decision.latency_ms}")
    if decision.triggered_policies:
        print("triggered     :")
        for p in decision.triggered_policies:
            print(f"  - {p.id} ({p.severity.value}): {p.reason}")
    if decision.safe_output is not None:
        print(f"safe_output   : {decision.safe_output}")


def main() -> int:
    # Surface SDK retry decisions on stderr. Set
    # TRUSTLOOP_LOG_LEVEL=debug to see the per-attempt log lines.
    level = os.environ.get("TRUSTLOOP_LOG_LEVEL", "warning").upper()
    logging.basicConfig(level=level, format="%(levelname)s %(name)s: %(message)s")

    args = sys.argv[1:]
    input_text = args[0] if len(args) > 0 else "hello"
    proposed_output = args[1] if len(args) > 1 else "hi there"

    url = os.environ.get("TRUSTLOOP_URL", DEFAULT_URL)
    api_key = os.environ.get("TRUSTLOOP_API_KEY") or None

    with Client(url, api_key=api_key) as client:
        try:
            decision = client.submit_event(build_event(input_text, proposed_output))
        except SdkError as e:
            print(f"error: {e}", file=sys.stderr)
            return 1

    print_decision(decision)

    # Exit non-zero on Block / Escalate so the quickstart CI workflow
    # can wire this into a pass/fail check (matches example-rust).
    if decision.verdict in (Verdict.block, Verdict.escalate):
        return 2
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
