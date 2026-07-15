"""Result type for guarded execution through the unified authorization kernel."""

from __future__ import annotations

from dataclasses import dataclass
from typing import Generic, TypeVar

from trustloopguard._generated.types import AuthorizationDecision

ResultT = TypeVar("ResultT")


@dataclass(frozen=True)
class AuthorizationResult(Generic[ResultT]):
    decision: AuthorizationDecision
    executed: bool
    value: ResultT | None = None
