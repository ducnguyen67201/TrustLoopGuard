"""TrustLoopGuard Python SDK.

Public surface:
    Client          — HTTP client implementing the Guard.check(draft, ctx) contract
    CheckRequest    — what the agent loop sends in
    Decision        — what TrustLoopGuard returns
    Verdict, Channel, Severity, TriggeredPolicy — supporting wire types

Type definitions are generated from the canonical Rust types in `tl-core`
via `cargo run -p tl-codegen`. Do not hand-edit `_generated/`.
"""

from trustloopguard.client import AsyncClient, Client
from trustloopguard.guard import GuardLogEvent, OutputGuard, guard, guard_async
from trustloopguard._generated.types import (
    ApiError,
    ApiErrorCode,
    Channel,
    CheckRequest,
    Decision,
    Severity,
    TriggeredPolicy,
    Verdict,
)
from trustloopguard.errors import (
    Decode,
    Forbidden,
    Gone,
    Internal,
    Invalid,
    NotFound,
    RateLimited,
    SdkError,
    Transport,
    Unauthorized,
    Unavailable,
    Unprocessable,
)
from trustloopguard.retry import RetryConfig

__all__ = [
    # Client
    "AsyncClient",
    "Client",
    # Wire types
    "ApiError",
    "ApiErrorCode",
    "Channel",
    "CheckRequest",
    "Decision",
    "Severity",
    "TriggeredPolicy",
    "Verdict",
    # Retry
    "RetryConfig",
    # Errors
    "SdkError",
    "Invalid",
    "Unauthorized",
    "Forbidden",
    "NotFound",
    "Gone",
    "Unprocessable",
    "RateLimited",
    "Internal",
    "Unavailable",
    "Transport",
    "Decode",
    # Guard helper
    "guard",
    "guard_async",
    "GuardLogEvent",
    "OutputGuard",
]

__version__ = "0.0.1"
