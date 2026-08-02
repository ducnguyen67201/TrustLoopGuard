from __future__ import annotations

from dataclasses import asdict

import pytest

from featherlane_ai import AuthorizationEffect, SideEffectClass
from featherlane_ai.integrations import (
    AdapterLogEvent,
    AdapterWarning,
    AdapterWarningCode,
    ToolGuardMessages,
    tool_schema_hash,
)
from featherlane_ai.integrations._core import (
    build_argument_evidence,
    emit_log,
    emit_warning,
    resolve_side_effect,
    safe_tool_message,
)


@pytest.mark.parametrize(
    ("schema", "expected"),
    [
        ({}, "featherlane-ai-schema:fnv1a64:08f44b07b5901a25"),
        (
            {
                "type": "object",
                "properties": {
                    "b": {"type": "integer"},
                    "a": {"type": "string"},
                },
                "required": ["a"],
            },
            "featherlane-ai-schema:fnv1a64:f95ce20f84567dd1",
        ),
        (
            {
                "type": "object",
                "properties": {
                    "items": {
                        "type": "array",
                        "items": {"type": "string"},
                    }
                },
            },
            "featherlane-ai-schema:fnv1a64:4e2d56feadc9be83",
        ),
        (
            {
                "type": "object",
                "description": "Café 😀",
                "properties": {
                    "city": {
                        "description": "Hà Nội",
                        "type": "string",
                    }
                },
            },
            "featherlane-ai-schema:fnv1a64:8df48d13fea63701",
        ),
        (
            {
                "type": "number",
                "minimum": 1e-7,
                "maximum": 1e20,
                "multipleOf": 0.1,
            },
            "featherlane-ai-schema:fnv1a64:9c0380fc74da9183",
        ),
    ],
)
def test_tool_schema_hash_matches_typescript_fixtures(
    schema: dict[str, object],
    expected: str,
) -> None:
    assert tool_schema_hash(schema) == expected


def test_tool_schema_hash_is_independent_of_object_key_order() -> None:
    first = {
        "type": "object",
        "properties": {
            "b": {"type": "integer"},
            "a": {"type": "string"},
        },
        "required": ["a"],
    }
    second = {
        "required": ["a"],
        "properties": {
            "a": {"type": "string"},
            "b": {"type": "integer"},
        },
        "type": "object",
    }
    assert tool_schema_hash(first) == tool_schema_hash(second)


def test_safe_tool_messages_cover_every_non_execution_effect() -> None:
    messages = ToolGuardMessages()
    assert safe_tool_message(AuthorizationEffect.deny, messages) == messages.deny
    assert (
        safe_tool_message(AuthorizationEffect.require_approval, messages)
        == messages.require_approval
    )
    assert safe_tool_message(AuthorizationEffect.defer, messages) == messages.defer
    assert (
        safe_tool_message(AuthorizationEffect.transform, messages)
        == messages.transform
    )
    with pytest.raises(ValueError, match="permit"):
        safe_tool_message(AuthorizationEffect.permit, messages)


def test_argument_evidence_maps_each_top_level_parameter() -> None:
    sources, provenance = build_argument_evidence(
        "agno",
        {"order_id": "secret-order", "options": {"priority": True}},
    )
    assert len(sources) == 1
    assert sources[0].id == "agno.tool_arguments"
    assert sources[0].origin.value == "unknown"
    assert provenance.root == {
        "order_id": ["agno.tool_arguments"],
        "options": ["agno.tool_arguments"],
    }


def test_missing_side_effect_defaults_once_and_warning_has_no_parameters() -> None:
    warnings: list[AdapterWarning] = []
    warned: set[str] = set()
    for _ in range(2):
        selected = resolve_side_effect(
            "confirm_order",
            {},
            SideEffectClass.api_mutation,
            framework="ag2",
            warned=warned,
            on_warning=warnings.append,
        )
    assert selected is SideEffectClass.api_mutation
    assert len(warnings) == 1
    assert "secret-order" not in repr(warnings[0])


def test_observability_callback_failures_are_swallowed_and_payloads_are_bounded() -> None:
    warning = AdapterWarning(
        code=AdapterWarningCode.tool_schema_unavailable,
        message="The public schema was unavailable.",
        framework="ag2",
        tool_name="confirm_order",
    )
    event = AdapterLogEvent(
        framework="ag2",
        boundary="tool",
        agent_id="sales-agent",
        trace_id="trace-1",
        effect="deny",
        executed=False,
        latency_ms=4,
        tool_name="confirm_order",
        invocation_id="call-1",
    )

    def fail(_: object) -> None:
        raise RuntimeError("observer failed")

    emit_warning(fail, warning)
    emit_log(fail, event)
    assert "secret-order" not in repr(asdict(event))
