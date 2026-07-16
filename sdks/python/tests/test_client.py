"""Smoke tests for the TrustLoopGuard Python SDK. No live network calls."""

from __future__ import annotations

import json

import httpx
import respx

from trustloopguard import (
    Action,
    CreateRunEventRequest,
    Client,
    CreateRunRequest,
    AuthorizationDecision,
    EventKind,
    GuardEvent,
    Labels,
    Origin,
    Principal,
    ProvenanceMap,
    RunKind,
    RunEventKind,
    RunStatus,
    RunSummary,
    SideEffectClass,
    Source,
    ToolIdentity,
    AuthorizationEffect,
)

def output_event(text: str = "hello") -> GuardEvent:
    return GuardEvent(
        kind=EventKind.output_proposed,
        principal=Principal(workspace_id="default", environment_id="production", agent_id="agent-a"),
        action=Action(operation="output", parameters={"text": text}, side_effect=SideEffectClass.none),
        sources=[Source(id="input", origin=Origin.user, labels=Labels())],
        provenance=ProvenanceMap({"text": ["input"]}),
        context={"channel": "chat", "domain": "customer_support"},
    )


@respx.mock
def test_submit_event_allow_round_trip() -> None:
    respx.post("https://api.example.test/v1/events").mock(
        return_value=httpx.Response(
            200,
            json={
                "trace_id": "trace-1",
                "domain": "content",
                "effect": "permit",
                "reason": "no policies triggered",
                "findings": [],
                "transformed_value": None,
                "latency_ms": 1,
            },
        )
    )
    with Client("https://api.example.test", api_key="test") as client:
        decision: AuthorizationDecision = client.submit_event(output_event())
    assert decision.effect is AuthorizationEffect.permit
    assert decision.trace_id == "trace-1"


@respx.mock
def test_bearer_header_is_sent() -> None:
    route = respx.post("https://api.example.test/v1/events").mock(
        return_value=httpx.Response(
            200,
            json={
                "trace_id": "t",
                "domain": "content",
                "effect": "permit",
                "reason": "",
                "findings": [],
                "transformed_value": None,
                "latency_ms": 0,
            },
        )
    )
    with Client("https://api.example.test", api_key="sk-abc") as client:
        client.submit_event(output_event())
    assert route.calls.last.request.headers["authorization"] == "Bearer sk-abc"


@respx.mock
def test_authorized_shell_action_resumes_exact_event_and_completes_lease() -> None:
    approval_id = "018f1111-1111-7111-8111-111111111111"
    grant_id = "018f2222-2222-7222-8222-222222222222"
    lease_id = "018f3333-3333-7333-8333-333333333333"
    submitted: list[dict] = []

    def event_response(request: httpx.Request) -> httpx.Response:
        submitted.append(json.loads(request.content))
        if len(submitted) == 1:
            return httpx.Response(
                200,
                json={
                    "trace_id": "trace-pending",
                    "domain": "tool",
                    "effect": "require_approval",
                    "reason": "review required",
                    "findings": [],
                    "approval": {
                        "id": approval_id,
                        "status": "pending",
                        "envelope_hash": "sha256:v1:approval",
                        "expires_at": "2026-07-15T01:00:00Z",
                        "poll_after_ms": 1,
                    },
                    "latency_ms": 1,
                },
            )
        return httpx.Response(
            200,
            json={
                "trace_id": "trace-permit",
                "domain": "tool",
                "effect": "permit",
                "reason": "approved",
                "findings": [],
                "lease": {
                    "id": lease_id,
                    "intent_id": "018f4444-4444-7444-8444-444444444444",
                    "grant_id": grant_id,
                    "attempt_id": "attempt-1",
                    "fingerprint": "sha256:v1:subject",
                    "status": "claimed",
                    "claimed_at": "2026-07-15T00:00:00Z",
                    "expires_at": "2026-07-15T00:05:00Z",
                },
                "latency_ms": 1,
            },
        )

    respx.post("https://api.example.test/v1/events").mock(side_effect=event_response)
    respx.get(f"https://api.example.test/v1/authorization/approvals/{approval_id}").mock(
        return_value=httpx.Response(
            200,
            json={
                "id": approval_id,
                "workspace_id": "ws",
                "environment_id": "production",
                "intent_id": "018f4444-4444-7444-8444-444444444444",
                "status": "approved",
                "envelope": {
                    "schema": "authorization-envelope:v1",
                    "intent_id": "018f4444-4444-7444-8444-444444444444",
                    "domain": "tool",
                    "capability": "tool:claude-code/bash",
                    "principal_id": "agent-1",
                    "subject_id": "tool-use-shell",
                    "subject_hash": "sha256:v1:subject",
                    "exact_fingerprint": "sha256:v1:subject",
                    "fingerprint_version": 1,
                    "requirement_ids": ["tool-policy:approve-delete"],
                    "policy_versions": ["approve-delete"],
                    "issued_at": "2026-07-15T00:00:00Z",
                    "expires_at": "2026-07-15T01:00:00Z",
                },
                "envelope_hash": "sha256:v1:approval",
                "approver_roles": ["owner"],
                "grant_id": grant_id,
                "expires_at": "2026-07-15T01:00:00Z",
                "created_at": "2026-07-15T00:00:00Z",
                "updated_at": "2026-07-15T00:00:01Z",
            },
        )
    )
    completion = respx.post(
        f"https://api.example.test/v1/authorization/leases/{lease_id}/complete"
    ).mock(
        return_value=httpx.Response(
            200,
            json={
                "id": lease_id,
                "intent_id": "018f4444-4444-7444-8444-444444444444",
                "grant_id": grant_id,
                "attempt_id": "attempt-1",
                "fingerprint": "sha256:v1:subject",
                "status": "consumed",
                "claimed_at": "2026-07-15T00:00:00Z",
                "completed_at": "2026-07-15T00:00:02Z",
                "expires_at": "2026-07-15T00:05:00Z",
            },
        )
    )

    with Client("https://api.example.test") as client:
        result = client.with_authorized_shell_action(
            agent_id="agent-1",
            command="rm -rf ./build",
            invocation_id="tool-use-shell",
            tool_identity=ToolIdentity(
                server_id="claude-code",
                tool_name="Bash",
                schema_hash="sha256:v1:bash",
            ),
            execute=lambda parameters: parameters["command"],
            poll_interval=0.001,
        )

    assert result.executed
    assert result.value == "rm -rf ./build"
    assert len(submitted) == 2
    assert submitted[0]["kind"] == "shell.action.proposed"
    assert submitted[1]["action"]["invocation_id"] == "tool-use-shell"
    assert submitted[1]["action"]["authorization"]["grant_id"] == grant_id
    assert json.loads(completion.calls[0].request.content) == {
        "status": "consumed",
        "outcome": {"success": True},
    }


@respx.mock
def test_start_and_finish_run() -> None:
    run_body = {
        "id": "018f1111-1111-7111-8111-111111111111",
        "workspace_id": "ws_test",
        "environment_id": "production",
        "environment": "production",
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
