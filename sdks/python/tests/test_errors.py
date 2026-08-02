"""Error-mapping tests for the Python SDK.

These cover the same ground as `crates/tl-sdk-rust/src/error.rs` and
should evolve in lockstep with the Rust tests so the parity claim in
docs/SDK_DRIVEN.md keeps holding.
"""

from __future__ import annotations

import httpx
import pytest
import respx

from featherlane_ai import (
    Action,
    ApiErrorCode,
    Client,
    EventKind,
    GuardEvent,
    Internal,
    Invalid,
    Labels,
    Origin,
    Principal,
    ProvenanceMap,
    RateLimited,
    SdkError,
    SideEffectClass,
    Source,
    Unauthorized,
    Unavailable,
)
from featherlane_ai.errors import (
    code_from_http_status,
    from_response,
    parse_retry_after,
    synthesize_api_error,
)

def output_event(text: str = "hello") -> GuardEvent:
    return GuardEvent(
        kind=EventKind.output_proposed,
        principal=Principal(workspace_id="default", environment_id="production", agent_id="a"),
        action=Action(operation="output", parameters={"text": text}, side_effect=SideEffectClass.none),
        sources=[Source(id="input", origin=Origin.user, labels=Labels())],
        provenance=ProvenanceMap({"text": ["input"]}),
        context={"channel": "chat", "domain": "customer_support"},
    )


def test_status_to_code_table_matches_rust() -> None:
    assert code_from_http_status(400) is ApiErrorCode.invalid
    assert code_from_http_status(401) is ApiErrorCode.unauthorized
    assert code_from_http_status(429) is ApiErrorCode.rate_limited
    assert code_from_http_status(503) is ApiErrorCode.unavailable
    assert code_from_http_status(599) is ApiErrorCode.internal
    assert code_from_http_status(418) is ApiErrorCode.invalid


def test_synthesized_error_carries_default_retriable_flag() -> None:
    err = synthesize_api_error(503, "")
    assert err.code is ApiErrorCode.unavailable
    assert err.retriable is True

    err400 = synthesize_api_error(400, "bad input")
    assert err400.retriable is False
    assert err400.message == "bad input"


def test_canonical_body_routed_to_typed_exception() -> None:
    body = '{"code":"unauthorized","message":"bad token","retriable":false}'
    exc = from_response(401, body)
    assert isinstance(exc, Unauthorized)
    assert exc.code is ApiErrorCode.unauthorized
    assert exc.is_retriable() is False


def test_rate_limit_carries_retry_after() -> None:
    body = '{"code":"rate_limited","message":"slow down","retriable":true}'
    exc = from_response(429, body, retry_after=7.0)
    assert isinstance(exc, RateLimited)
    assert exc.retry_after == 7.0
    assert exc.is_retriable() is True


def test_unrecognized_body_falls_back_to_status() -> None:
    exc = from_response(503, "<html>upstream down</html>")
    assert isinstance(exc, Unavailable)
    assert exc.is_retriable() is True

    exc500 = from_response(500, "")
    assert isinstance(exc500, Internal)


def test_unknown_code_in_body_falls_back_to_status() -> None:
    body = '{"code":"teapot","message":"i\'m a teapot","retriable":false}'
    exc = from_response(418, body)
    # 418 maps to Invalid via the status fallback (no canonical code).
    assert isinstance(exc, Invalid)


def test_parse_retry_after_handles_seconds_and_garbage() -> None:
    assert parse_retry_after("3") == 3.0
    assert parse_retry_after("  2.5 ") == 2.5
    assert parse_retry_after(None) is None
    assert parse_retry_after("not-a-number") is None


@respx.mock
def test_client_raises_typed_error_on_401() -> None:
    respx.post("https://api.example.test/v1/events").mock(
        return_value=httpx.Response(
            401,
            text='{"code":"unauthorized","message":"bad token","retriable":false}',
        )
    )
    with Client("https://api.example.test", api_key="oops") as client:
        with pytest.raises(Unauthorized) as exc_info:
            client.submit_event(output_event("y"))
    assert exc_info.value.code is ApiErrorCode.unauthorized
    # SdkError is the common base — callers can `except SdkError` if they
    # don't care about the specific variant.
    assert isinstance(exc_info.value, SdkError)


@respx.mock
def test_client_carries_retry_after_on_429() -> None:
    respx.post("https://api.example.test/v1/events").mock(
        return_value=httpx.Response(
            429,
            headers={"retry-after": "5"},
            text='{"code":"rate_limited","message":"too many","retriable":true}',
        )
    )
    with Client("https://api.example.test") as client:
        with pytest.raises(RateLimited) as exc_info:
            client.submit_event(output_event(""))
    assert exc_info.value.retry_after == 5.0
