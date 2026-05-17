"""Smoke tests for the TrustLoopGuard Python SDK. No live network calls."""

from __future__ import annotations

import json

import httpx
import respx

from trustloopguard import (
    Channel,
    CheckRequest,
    CreateRunEventRequest,
    Client,
    CreateRunRequest,
    Decision,
    RunKind,
    RunEventKind,
    RunStatus,
    RunSummary,
    Verdict,
)


@respx.mock
def test_check_allow_round_trip() -> None:
    respx.post("https://api.example.test/v1/check").mock(
        return_value=httpx.Response(
            200,
            json={
                "trace_id": "trace-1",
                "verdict": "allow",
                "reason": "no policies triggered",
                "triggered_policies": [],
                "safe_output": None,
                "latency_ms": 1,
            },
        )
    )
    with Client("https://api.example.test", api_key="test") as client:
        decision: Decision = client.check(
            CheckRequest(
                agent_id="agent-a",
                channel=Channel.chat,
                input="hi",
                proposed_output="hello",
            )
        )
    assert decision.verdict is Verdict.allow
    assert decision.trace_id == "trace-1"


@respx.mock
def test_bearer_header_is_sent() -> None:
    route = respx.post("https://api.example.test/v1/check").mock(
        return_value=httpx.Response(
            200,
            json={
                "trace_id": "t",
                "verdict": "allow",
                "reason": "",
                "triggered_policies": [],
                "safe_output": None,
                "latency_ms": 0,
            },
        )
    )
    with Client("https://api.example.test", api_key="sk-abc") as client:
        client.check(
            CheckRequest(
                agent_id="agent-a",
                channel=Channel.voice,
                input="",
                proposed_output="",
            )
        )
    assert route.calls.last.request.headers["authorization"] == "Bearer sk-abc"


@respx.mock
def test_start_and_finish_run() -> None:
    run_body = {
        "id": "018f1111-1111-7111-8111-111111111111",
        "workspace_id": "ws_test",
        "agent_id": "support-agent",
        "kind": "chat_session",
        "status": "running",
        "external_id": "chat-123",
        "metadata": {},
        "started_at": "2026-05-17T00:00:00Z",
        "ended_at": None,
        "created_at": "2026-05-17T00:00:00Z",
        "updated_at": "2026-05-17T00:00:00Z",
        "trace_count": 0,
        "blocked_count": 0,
        "rewritten_count": 0,
        "escalated_count": 0,
        "p95_latency_ms": None,
    }
    create = respx.post("https://api.example.test/v1/runs").mock(
        return_value=httpx.Response(201, json=run_body)
    )
    update = respx.patch(
        "https://api.example.test/v1/runs/018f1111-1111-7111-8111-111111111111"
    ).mock(
        return_value=httpx.Response(
            200,
            json={**run_body, "status": "completed", "ended_at": "2026-05-17T00:01:00Z"},
        )
    )
    create_event = respx.post(
        "https://api.example.test/v1/runs/018f1111-1111-7111-8111-111111111111/events"
    ).mock(
        return_value=httpx.Response(
            201,
            json={
                "id": "018f2222-2222-7222-8222-222222222222",
                "workspace_id": "ws_test",
                "run_id": "018f1111-1111-7111-8111-111111111111",
                "sequence": 1,
                "kind": "user_turn",
                "label": "Turn 1",
                "input_summary": "Customer asks about a refund",
                "output_summary": None,
                "metadata": {},
                "occurred_at": "2026-05-17T00:00:01Z",
                "created_at": "2026-05-17T00:00:01Z",
            },
        )
    )

    with Client("https://api.example.test", api_key="test") as client:
        run: RunSummary = client.start_run(
            CreateRunRequest(
                agent_id="support-agent",
                kind=RunKind.chat_session,
                external_id="chat-123",
            )
        )
        event = client.create_run_event(
            run.id,
            CreateRunEventRequest(
                kind=RunEventKind.user_turn,
                label="Turn 1",
                input_summary="Customer asks about a refund",
            ),
        )
        finished = client.finish_run(run.id)

    assert run.status is RunStatus.running
    assert event.kind is RunEventKind.user_turn
    assert finished.status is RunStatus.completed
    assert create.calls.last.request.headers["authorization"] == "Bearer test"
    assert json.loads(create_event.calls.last.request.content) == {
        "kind": "user_turn",
        "label": "Turn 1",
        "input_summary": "Customer asks about a refund",
    }
    assert update.calls.last.request.content == b'{"status":"completed"}'
