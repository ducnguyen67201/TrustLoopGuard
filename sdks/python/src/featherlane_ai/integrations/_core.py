"""Dependency-free primitives shared by Python agent framework adapters."""

from __future__ import annotations

import copy
import json
import math
import time
from collections.abc import Callable, Mapping, Sequence
from dataclasses import dataclass
from decimal import Decimal
from enum import Enum
from typing import Any, Generic, Literal, TypeVar, cast

from featherlane_ai._generated.types import (
    AuthorizationEffect,
    Labels,
    Origin,
    ProvenanceMap,
    SideEffectClass,
    Source,
)

Framework = Literal["ag2", "agno"]
Boundary = Literal["tool", "output"]
ResultT = TypeVar("ResultT")


@dataclass(frozen=True)
class ToolGuardMessages:
    """Framework-visible results for tool proposals that cannot execute."""

    deny: str = "This action was blocked by policy and was not executed."
    require_approval: str = (
        "This action still requires approval and was not executed."
    )
    defer: str = "This action needs more verified context and was not executed."
    transform: str = (
        "This tool call requires revised arguments and was not executed."
    )
    unavailable: str = (
        "Safety checks are unavailable, so this action was not executed."
    )


@dataclass(frozen=True)
class OutputGuardMessages:
    """Safe replacements for final plain-text output."""

    deny: str = "I can't help with that request."
    require_approval: str = (
        "A human teammate should review this before we continue."
    )
    defer: str = (
        "Required evidence or system state is unavailable. Please try again later."
    )
    unavailable: str = "I can't help with that request."


class AdapterWarningCode(str, Enum):
    already_guarded = "already_guarded"
    tool_schema_unavailable = "tool_schema_unavailable"
    tool_side_effect_defaulted = "tool_side_effect_defaulted"
    provider_hosted_tool_unavailable = "provider_hosted_tool_unavailable"
    dynamic_tool_unavailable = "dynamic_tool_unavailable"
    structured_output_unavailable = "structured_output_unavailable"
    lease_completion_failed_after_execution = (
        "lease_completion_failed_after_execution"
    )


@dataclass(frozen=True)
class AdapterWarning:
    code: AdapterWarningCode
    message: str
    framework: Framework
    tool_name: str | None = None


@dataclass(frozen=True)
class AdapterLogEvent:
    """Bounded adapter telemetry. Protected values never belong here."""

    framework: Framework
    boundary: Boundary
    agent_id: str
    trace_id: str
    effect: str
    executed: bool
    latency_ms: int
    tool_name: str | None = None
    invocation_id: str | None = None


@dataclass
class CallbackState(Generic[ResultT]):
    """Classifies SDK failure relative to customer callback execution."""

    started: bool = False
    completed: bool = False
    value: ResultT | None = None
    error: BaseException | None = None

    def run(self, callback: Callable[[], ResultT]) -> ResultT:
        self.started = True
        try:
            value = callback()
        except BaseException as error:
            self.error = error
            raise
        self.value = value
        self.completed = True
        return value

    async def run_async(self, callback: Callable[[], Any]) -> ResultT:
        self.started = True
        try:
            value = await callback()
        except BaseException as error:
            self.error = error
            raise
        self.value = cast(ResultT, value)
        self.completed = True
        return self.value


def tool_schema_hash(schema: Mapping[str, Any] | None) -> str:
    """Return the TypeScript-compatible canonical FNV-1a tool schema identity."""

    canonical = _canonical_json(schema or {})
    value = 0xCBF29CE484222325
    encoded = canonical.encode("utf-16-le", errors="surrogatepass")
    for index in range(0, len(encoded), 2):
        code_unit = encoded[index] | (encoded[index + 1] << 8)
        value ^= code_unit
        value = (value * 0x100000001B3) & 0xFFFFFFFFFFFFFFFF
    return f"featherlane-ai-schema:fnv1a64:{value:016x}"


def safe_tool_message(
    effect: AuthorizationEffect,
    messages: ToolGuardMessages,
) -> str:
    if effect is AuthorizationEffect.deny:
        return messages.deny
    if effect is AuthorizationEffect.require_approval:
        return messages.require_approval
    if effect is AuthorizationEffect.defer:
        return messages.defer
    if effect is AuthorizationEffect.transform:
        return messages.transform
    raise ValueError("permit decisions do not have a non-execution message")


def build_argument_evidence(
    framework: Framework,
    parameters: Mapping[str, Any],
) -> tuple[list[Source], ProvenanceMap]:
    source_id = f"{framework}.tool_arguments"
    sources = [
        Source(id=source_id, origin=Origin.unknown, labels=Labels())
    ]
    provenance = ProvenanceMap(
        {str(name): [source_id] for name in parameters}
    )
    return sources, provenance


def resolve_side_effect(
    tool_name: str,
    configured: Mapping[str, SideEffectClass],
    default: SideEffectClass,
    *,
    framework: Framework,
    warned: set[str],
    on_warning: Callable[[AdapterWarning], None] | None,
) -> SideEffectClass:
    selected = configured.get(tool_name)
    if selected is not None:
        return selected
    warning_key = f"side-effect:{tool_name}"
    if warning_key not in warned:
        warned.add(warning_key)
        emit_warning(
            on_warning,
            AdapterWarning(
                code=AdapterWarningCode.tool_side_effect_defaulted,
                message=(
                    "No side-effect class was configured; using the "
                    "conservative adapter default."
                ),
                framework=framework,
                tool_name=tool_name,
            ),
        )
    return default


def warn_once(
    code: AdapterWarningCode,
    *,
    message: str,
    framework: Framework,
    warned: set[str],
    on_warning: Callable[[AdapterWarning], None] | None,
    tool_name: str | None = None,
    key: str | None = None,
) -> None:
    warning_key = key or f"{code.value}:{tool_name or ''}"
    if warning_key in warned:
        return
    warned.add(warning_key)
    emit_warning(
        on_warning,
        AdapterWarning(
            code=code,
            message=message,
            framework=framework,
            tool_name=tool_name,
        ),
    )


def emit_warning(
    callback: Callable[[AdapterWarning], None] | None,
    warning: AdapterWarning,
) -> None:
    if callback is None:
        return
    try:
        callback(warning)
    except Exception:
        pass


def emit_log(
    callback: Callable[[AdapterLogEvent], None] | None,
    event: AdapterLogEvent,
) -> None:
    if callback is None:
        return
    try:
        callback(event)
    except Exception:
        pass


def elapsed_ms(start: float) -> int:
    return int((time.monotonic() - start) * 1000)


def copied_context(
    configured: Mapping[str, Any] | None,
    **framework_values: Any,
) -> dict[str, Any]:
    result = copy.deepcopy(dict(configured or {}))
    result.update(
        {
            key: copy.deepcopy(value)
            for key, value in framework_values.items()
            if value is not None
        }
    )
    return result


def _canonical_json(value: Any) -> str:
    seen: set[int] = set()

    def normalize(current: Any, *, object_value: bool = False) -> Any:
        if current is None or isinstance(current, (str, bool)):
            return current
        if isinstance(current, int):
            return current
        if isinstance(current, float):
            if not math.isfinite(current):
                return None
            return int(current) if current.is_integer() else current
        if callable(current):
            if object_value:
                return _OMIT
            name = getattr(current, "__name__", "") or "anonymous"
            return f"[function:{name}]"
        if isinstance(current, Mapping):
            identity = id(current)
            if identity in seen:
                return "[circular]"
            seen.add(identity)
            normalized: dict[str, Any] = {}
            for key in sorted(current):
                if not isinstance(key, str):
                    raise TypeError("tool schemas must use string object keys")
                nested = normalize(current[key], object_value=True)
                if nested is not _OMIT:
                    normalized[key] = nested
            return normalized
        if isinstance(current, Sequence) and not isinstance(
            current, (str, bytes, bytearray)
        ):
            identity = id(current)
            if identity in seen:
                return "[circular]"
            seen.add(identity)
            return [normalize(item) for item in current]
        raise TypeError(
            f"tool schema contains unsupported value: {type(current).__name__}"
        )

    return _json_stringify(normalize(value))


def _json_stringify(value: Any) -> str:
    if value is None:
        return "null"
    if value is True:
        return "true"
    if value is False:
        return "false"
    if isinstance(value, (int, float)):
        return _javascript_number(value)
    if isinstance(value, str):
        return json.dumps(value, ensure_ascii=False)
    if isinstance(value, list):
        return "[" + ",".join(_json_stringify(item) for item in value) + "]"
    if isinstance(value, dict):
        return (
            "{"
            + ",".join(
                f"{json.dumps(key, ensure_ascii=False)}:{_json_stringify(nested)}"
                for key, nested in value.items()
            )
            + "}"
        )
    raise TypeError(f"unsupported normalized JSON value: {type(value).__name__}")


def _javascript_number(value: int | float) -> str:
    number = float(value)
    if not math.isfinite(number):
        return "null"
    if number == 0:
        return "0"
    if isinstance(value, int) and -(2**53) < value < 2**53:
        return str(value)

    text = repr(number).lower()
    magnitude = abs(number)
    if 1e-6 <= magnitude < 1e21:
        if "e" in text:
            return format(Decimal(text), "f")
        return text[:-2] if text.endswith(".0") else text

    mantissa, exponent = text.split("e")
    exponent_sign = ""
    if exponent[0] in {"+", "-"}:
        exponent_sign = exponent[0]
        exponent = exponent[1:]
    exponent = exponent.lstrip("0") or "0"
    return f"{mantissa}e{exponent_sign}{exponent}"


_OMIT = object()
