"""TrustLoopGuard Python SDK.

Public surface:
    Client          — HTTP client implementing the Guard.check(draft, ctx) contract
    CheckRequest    — what the agent loop sends in
    Decision        — what TrustLoopGuard returns
    Verdict, Channel, Severity, TriggeredPolicy — supporting wire types

Type definitions are generated from the canonical Rust types in `tl-core`
via `cargo run -p tl-codegen`. Do not hand-edit `_generated/`.
"""

from trustloopguard.client import Client
from trustloopguard._generated.types import (
    Channel,
    CheckRequest,
    Decision,
    Severity,
    TriggeredPolicy,
    Verdict,
)

__all__ = [
    "Client",
    "Channel",
    "CheckRequest",
    "Decision",
    "Severity",
    "TriggeredPolicy",
    "Verdict",
]

__version__ = "0.0.1"
