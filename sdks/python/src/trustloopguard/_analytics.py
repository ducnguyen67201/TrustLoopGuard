"""Internal PostHog analytics module for the TrustLoopGuard Python SDK.

Tracks SDK-level usage events so TrustLoopGuard can understand how customers
use the SDK in production. All events are metadata-only — no PII is included.

Set POSTHOG_PROJECT_TOKEN and optionally POSTHOG_HOST to enable. If the token
is absent or posthog is not installed, every call is a no-op.
"""

from __future__ import annotations

import atexit
import logging
import os
from typing import Any

_logger = logging.getLogger("trustloopguard.analytics")

_client: Any = None
_initialized = False


def _get_client() -> Any:
    global _client, _initialized
    if _initialized:
        return _client

    _initialized = True
    token = os.getenv("POSTHOG_PROJECT_TOKEN")
    if not token:
        return None

    try:
        from posthog import Posthog  # type: ignore[import-not-found]

        _client = Posthog(
            token,
            host=os.getenv("POSTHOG_HOST", "https://us.i.posthog.com"),
            enable_exception_autocapture=True,
        )
        atexit.register(_client.shutdown)
    except Exception:  # noqa: BLE001
        _logger.debug("PostHog analytics unavailable", exc_info=True)

    return _client


def capture(distinct_id: str, event: str, properties: dict[str, Any] | None = None) -> None:
    """Capture a single SDK usage event. No-op when PostHog is not configured."""
    client = _get_client()
    if client is None:
        return
    try:
        client.capture(distinct_id=distinct_id, event=event, properties=properties or {})
    except Exception:  # noqa: BLE001
        _logger.debug("PostHog capture failed", exc_info=True)


def set_person_props(distinct_id: str, properties: dict[str, Any]) -> None:
    """Set person-level properties on an agent/workspace. No-op when PostHog is not configured."""
    client = _get_client()
    if client is None:
        return
    try:
        client.set(distinct_id=distinct_id, properties=properties)
    except Exception:  # noqa: BLE001
        _logger.debug("PostHog set failed", exc_info=True)
