"""Retry-policy tests for the Python SDK.

Mirrors `crates/tl-sdk-rust/src/retry.rs::tests`. The two suites must
agree on every number — divergence breaks the parity claim in
docs/SDK_DRIVEN.md.
"""

from __future__ import annotations

import httpx
import pytest
import respx

from trustloopguard import (
    Action,
    Client,
    EventKind,
    GuardEvent,
    Labels,
    Origin,
    Principal,
    ProvenanceMap,
    RetryConfig,
    SideEffectClass,
    Source,
    Unauthorized,
    Unavailable,
)
from trustloopguard._generated.types import ApiError, ApiErrorCode
from trustloopguard.errors import Invalid, RateLimited

def output_event(text: str = "hello") -> GuardEvent:
    return GuardEvent(
        kind=EventKind.output_proposed,
        principal=Principal(workspace_id="default", environment_id="production", agent_id="a"),
        action=Action(operation="output", parameters={"text": text}, side_effect=SideEffectClass.none),
        sources=[Source(id="input", origin=Origin.user, labels=Labels())],
        provenance=ProvenanceMap({"text": ["input"]}),
        context={"channel": "chat", "domain": "customer_support"},
    )


def _rate_limited(retry_after: float | None = None) -> RateLimited:
    return RateLimited(
        ApiError(
            code=ApiErrorCode.rate_limited,
            message="slow down",
            retriable=True,
            details=None,
        ),
        retry_after=retry_after,
    )


def _unavailable() -> Unavailable:
    return Unavailable(
        ApiError(
            code=ApiErrorCode.unavailable,
            message="upstream down",
            retriable=True,
            details=None,
        )
    )


def _invalid() -> Invalid:
    return Invalid(
        ApiError(
            code=ApiErrorCode.invalid,
            message="bad input",
            retriable=False,
            details=None,
        )
    )


def test_non_retriable_errors_stop_immediately() -> None:
    cfg = RetryConfig()
    assert cfg.next_delay(1, 0.0, _invalid(), 0.5) is None


def test_retries_unavailable_with_exponential_backoff() -> None:
    cfg = RetryConfig(
        max_attempts=4, base_delay_s=0.2, max_delay_s=8.0, total_budget_s=30.0
    )
    # jitter=0.5 → multiplier 1.0, so delay equals capped exp.
    assert cfg.next_delay(1, 0.0, _unavailable(), 0.5) == pytest.approx(0.2)
    assert cfg.next_delay(2, 0.2, _unavailable(), 0.5) == pytest.approx(0.4)
    assert cfg.next_delay(3, 0.6, _unavailable(), 0.5) == pytest.approx(0.8)


def test_caps_per_retry_delay_at_max_delay() -> None:
    cfg = RetryConfig(max_attempts=10, base_delay_s=1.0, max_delay_s=4.0, total_budget_s=60.0)
    # attempt=5 → 1s * 2^4 = 16s, capped to 4s.
    assert cfg.next_delay(5, 0.0, _unavailable(), 0.5) == pytest.approx(4.0)


def test_honors_retry_after_when_longer_than_jittered() -> None:
    cfg = RetryConfig()
    d = cfg.next_delay(1, 0.0, _rate_limited(retry_after=10.0), 0.5)
    assert d is not None
    assert d >= 10.0


def test_ignores_retry_after_when_jitter_already_longer() -> None:
    cfg = RetryConfig()
    d = cfg.next_delay(3, 0.0, _rate_limited(retry_after=0.0), 0.5)
    assert d is not None
    assert d >= 0.6


def test_stops_after_max_attempts() -> None:
    cfg = RetryConfig(max_attempts=2)
    assert cfg.next_delay(2, 0.0, _unavailable(), 0.5) is None


def test_stops_when_budget_exhausted() -> None:
    cfg = RetryConfig(total_budget_s=1.0)
    assert cfg.next_delay(1, 1.0, _unavailable(), 0.5) is None


def test_shrinks_last_delay_to_remaining_budget() -> None:
    cfg = RetryConfig(
        max_attempts=5, base_delay_s=2.0, max_delay_s=10.0, total_budget_s=3.0
    )
    d = cfg.next_delay(1, 2.5, _unavailable(), 0.5)
    assert d is not None
    assert d == pytest.approx(0.5, abs=1e-6)


def test_jitter_fraction_clamps_to_unit_interval() -> None:
    cfg = RetryConfig()
    d_low = cfg.next_delay(1, 0.0, _unavailable(), -1.0)
    d_hi = cfg.next_delay(1, 0.0, _unavailable(), 2.0)
    assert d_low == pytest.approx(0.15)
    assert d_hi == pytest.approx(0.25)


@respx.mock
def test_client_retries_503_until_success() -> None:
    route_503 = respx.post("https://api.example.test/v1/events").mock(
        side_effect=[
            httpx.Response(503),
            httpx.Response(503),
            httpx.Response(
                200,
                json={
                    "trace_id": "t-1",
                    "domain": "content",
                    "effect": "permit",
                    "reason": "ok",
                    "findings": [],
                    "latency_ms": 1,
                },
            ),
        ]
    )
    with Client(
        "https://api.example.test",
        retry=RetryConfig(
            max_attempts=4, base_delay_s=0.001, max_delay_s=0.01, total_budget_s=2.0
        ),
    ) as client:
        decision = client.submit_event(output_event())
    assert decision.trace_id == "t-1"
    assert route_503.call_count == 3


@respx.mock
def test_client_does_not_retry_401() -> None:
    route = respx.post("https://api.example.test/v1/events").mock(
        return_value=httpx.Response(
            401,
            text='{"code":"unauthorized","message":"bad","retriable":false}',
        )
    )
    with Client(
        "https://api.example.test",
        retry=RetryConfig(max_attempts=4, base_delay_s=0.001),
    ) as client:
        with pytest.raises(Unauthorized):
            client.submit_event(output_event("y"))
    assert route.call_count == 1
