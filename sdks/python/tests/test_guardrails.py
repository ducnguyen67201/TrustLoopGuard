"""Tests for the agent-bound guardrail SDK methods. No live network calls."""

from __future__ import annotations

import httpx
import pytest
import respx

from trustloopguard import (
    Client,
    GuardrailGenerateResponse,
    GuardrailListResponse,
    NotFound,
    Unavailable,
    Unprocessable,
)


GENERATE_BODY = {
    "generated": [
        {
            "id": "no-pii-leak",
            "description": "Block email patterns.",
            "severity": "high",
            "enabled": False,
            "source_yaml": "id: no-pii-leak\n",
        },
        {
            "id": "no-medical-claims",
            "description": "Block medical claims.",
            "severity": "high",
            "enabled": False,
            "source_yaml": "id: no-medical-claims\n",
        },
    ]
}


@respx.mock
def test_generate_returns_typed_response() -> None:
    respx.post(
        "https://api.example.test/v1/agents/baker-9000/guardrails/generate"
    ).mock(return_value=httpx.Response(200, json=GENERATE_BODY))
    with Client("https://api.example.test", api_key="t") as client:
        resp: GuardrailGenerateResponse = client.generate_guardrails("baker-9000")
    assert len(resp.generated) == 2
    assert all(not p.enabled for p in resp.generated)
    assert {p.id for p in resp.generated} == {"no-pii-leak", "no-medical-claims"}


@respx.mock
def test_generate_url_encodes_agent_id() -> None:
    # Slash + space must be percent-encoded.
    route = respx.post(
        "https://api.example.test/v1/agents/team%2Fbaker%20one/guardrails/generate"
    ).mock(return_value=httpx.Response(200, json=GENERATE_BODY))
    with Client("https://api.example.test") as client:
        client.generate_guardrails("team/baker one")
    assert route.called


@respx.mock
def test_generate_404_raises_not_found() -> None:
    respx.post(
        "https://api.example.test/v1/agents/ghost/guardrails/generate"
    ).mock(
        return_value=httpx.Response(
            404,
            json={
                "code": "not_found",
                "message": "agent ghost not found",
                "retriable": False,
                "details": None,
            },
        )
    )
    with Client("https://api.example.test") as client:
        with pytest.raises(NotFound):
            client.generate_guardrails("ghost")


@respx.mock
def test_generate_422_raises_unprocessable() -> None:
    respx.post(
        "https://api.example.test/v1/agents/silent/guardrails/generate"
    ).mock(
        return_value=httpx.Response(
            422,
            json={
                "code": "unprocessable",
                "message": "agent has no system_prompt",
                "retriable": False,
                "details": None,
            },
        )
    )
    with Client("https://api.example.test") as client:
        with pytest.raises(Unprocessable):
            client.generate_guardrails("silent")


@respx.mock
def test_generate_503_raises_unavailable() -> None:
    respx.post(
        "https://api.example.test/v1/agents/a/guardrails/generate"
    ).mock(
        return_value=httpx.Response(
            503,
            json={
                "code": "unavailable",
                "message": "no LLM",
                "retriable": True,
                "details": None,
            },
        )
    )
    # max_attempts=1 so we don't actually retry the 503 here.
    from trustloopguard import RetryConfig

    with Client(
        "https://api.example.test", retry=RetryConfig(max_attempts=1)
    ) as client:
        with pytest.raises(Unavailable):
            client.generate_guardrails("a")


@respx.mock
def test_list_returns_owned_policies() -> None:
    respx.get("https://api.example.test/v1/agents/baker-9000/guardrails").mock(
        return_value=httpx.Response(
            200,
            json={
                "policies": [
                    {
                        "id": "no-pii-leak",
                        "description": "Block emails.",
                        "severity": "high",
                        "enabled": False,
                    }
                ]
            },
        )
    )
    with Client("https://api.example.test") as client:
        out: GuardrailListResponse = client.list_guardrails("baker-9000")
    assert len(out.policies) == 1
    assert out.policies[0].id == "no-pii-leak"


@respx.mock
def test_list_for_unknown_agent_returns_empty() -> None:
    respx.get("https://api.example.test/v1/agents/ghost/guardrails").mock(
        return_value=httpx.Response(200, json={"policies": []})
    )
    with Client("https://api.example.test") as client:
        out = client.list_guardrails("ghost")
    assert out.policies == []
