"""Framework-neutral types shared by TrustLoopGuard agent integrations.

Framework adapters live in their matching modules so importing this namespace
does not require AG2 or Agno to be installed.
"""

from trustloopguard.integrations._core import (
    AdapterLogEvent,
    AdapterWarning,
    AdapterWarningCode,
    OutputGuardMessages,
    ToolGuardMessages,
    tool_schema_hash,
)

__all__ = [
    "AdapterLogEvent",
    "AdapterWarning",
    "AdapterWarningCode",
    "OutputGuardMessages",
    "ToolGuardMessages",
    "tool_schema_hash",
]
