"""Retry policy for the TrustLoopGuard Python SDK.

Mirrors `tl-sdk-rust`'s `RetryConfig`. Same defaults, same `next_delay`
contract — given the attempt number, elapsed time, the error that just
occurred, and a jitter fraction, return either the delay before the next
attempt (in seconds) or `None` to stop.

Production callers don't construct this directly; the `Client` accepts a
`RetryConfig` and feeds `random.random()` as the jitter fraction. Tests
pin the jitter fraction for determinism — see `tests/test_retry.py`.
"""

from __future__ import annotations

from dataclasses import dataclass

from trustloopguard.errors import RateLimited, SdkError


@dataclass(frozen=True)
class RetryConfig:
    """Retry policy. Defaults match `tl-sdk-rust`'s `RetryConfig::default`."""

    max_attempts: int = 4
    """Total attempts including the initial one. ``1`` means no retry."""

    total_budget_s: float = 30.0
    """Hard cap on total wall time including sleeps, in seconds."""

    base_delay_s: float = 0.2
    """Base for exponential backoff: delay = base * 2^(attempt-1)."""

    max_delay_s: float = 8.0
    """Cap on a single retry delay; prevents runaway exponential."""

    def next_delay(
        self,
        attempt: int,
        elapsed_s: float,
        err: SdkError,
        jitter_fraction: float,
    ) -> float | None:
        """Compute the delay before the next attempt, or ``None`` to stop.

        Mirrors `tl_sdk_rust::RetryConfig::next_delay` exactly.
        """
        if not err.is_retriable():
            return None
        if attempt >= self.max_attempts:
            return None
        if elapsed_s >= self.total_budget_s:
            return None

        # Exponential backoff capped at max_delay_s.
        exp = self.base_delay_s * (2 ** (attempt - 1))
        capped = min(exp, self.max_delay_s)

        # ±25% jitter: jitter_fraction in [0,1] → multiplier in [0.75, 1.25].
        frac = max(0.0, min(1.0, jitter_fraction))
        multiplier = 0.75 + (frac * 0.5)
        jittered = capped * multiplier

        # Take max of jittered and Retry-After (when present) so a
        # misconfigured server can't tighten our loop below jitter.
        server_hint: float | None = None
        if isinstance(err, RateLimited):
            server_hint = err.retry_after
        if server_hint is not None:
            delay = max(server_hint, jittered)
        else:
            delay = jittered

        # Don't sleep past the budget.
        remaining = self.total_budget_s - elapsed_s
        if remaining <= 0:
            return None
        return min(delay, remaining)
