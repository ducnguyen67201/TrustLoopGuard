# This file is overwritten by `cargo run -p tl-codegen`.
# Do not hand-edit. The placeholder below makes the package importable
# before the first codegen run; CI will regenerate it from docs/openapi.yaml.

from __future__ import annotations

from enum import Enum
from typing import Any

from pydantic import BaseModel, Field


class Channel(str, Enum):
    voice = "voice"
    chat = "chat"
    email = "email"
    other = "other"


class Verdict(str, Enum):
    allow = "allow"
    block = "block"
    rewrite = "rewrite"
    escalate = "escalate"


class Severity(str, Enum):
    low = "low"
    medium = "medium"
    high = "high"
    critical = "critical"


class TriggeredPolicy(BaseModel):
    id: str
    severity: Severity
    reason: str


class CheckRequest(BaseModel):
    agent_id: str
    channel: Channel
    input: str
    proposed_output: str
    policies: list[str] = Field(default_factory=list)
    context: Any = None
    trace_id: str | None = None


class Decision(BaseModel):
    trace_id: str
    verdict: Verdict
    reason: str
    triggered_policies: list[TriggeredPolicy]
    safe_output: str | None = None
    latency_ms: int
