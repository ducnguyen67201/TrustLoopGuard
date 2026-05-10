//! Retry policy for the TrustLoopGuard Rust SDK.
//!
//! Decisions live in [`RetryConfig::next_delay`], a pure function: given
//! the attempt number, elapsed time, and the error that just occurred,
//! return either the delay before the next attempt or `None` to stop.
//!
//! Keeping the policy pure means the integration tests in `tests/retry.rs`
//! exercise the HTTP wiring while these unit tests pin the policy itself.
//! The two layers can fail independently and the diagnostic is sharper.
//!
//! Mirrored to Python and TypeScript in PR 6.

use std::time::Duration;

use crate::error::SdkError;

#[derive(Debug, Clone)]
pub struct RetryConfig {
    /// Total attempts including the initial one. `1` means "no retry".
    pub max_attempts: u32,
    /// Hard cap on total wall time including sleeps.
    pub total_budget: Duration,
    /// Base for exponential backoff: delay = base * 2^(attempt-1).
    pub base_delay: Duration,
    /// Cap on a single retry delay; prevents runaway exponential.
    pub max_delay: Duration,
}

impl Default for RetryConfig {
    /// Defaults tuned for chat-channel callers (latency-tolerant).
    /// Voice callers should override `max_attempts = 1` and
    /// `total_budget` to a tight value (≈100ms) before the call.
    fn default() -> Self {
        Self {
            max_attempts: 4, // 1 initial + 3 retries
            total_budget: Duration::from_secs(30),
            base_delay: Duration::from_millis(200),
            max_delay: Duration::from_secs(8),
        }
    }
}

impl RetryConfig {
    /// Compute the delay before the next attempt, or `None` if the loop
    /// should give up.
    ///
    /// Inputs:
    /// - `attempt` — 1-indexed; the attempt number that *just* failed.
    /// - `elapsed` — total wall time spent in the loop so far, including
    ///   the most recent failed attempt.
    /// - `err` — the error that triggered the retry decision.
    /// - `jitter_fraction` — 0.0..1.0; the multiplier component of the
    ///   jittered backoff. Production callers pass a random value; tests
    ///   pin it for determinism.
    pub fn next_delay(
        &self,
        attempt: u32,
        elapsed: Duration,
        err: &SdkError,
        jitter_fraction: f64,
    ) -> Option<Duration> {
        if !err.is_retriable() {
            return None;
        }
        if attempt >= self.max_attempts {
            return None;
        }
        if elapsed >= self.total_budget {
            return None;
        }

        let exp = self
            .base_delay
            .saturating_mul(2u32.saturating_pow(attempt.saturating_sub(1)));
        let capped = exp.min(self.max_delay);

        // ±25% jitter centered on `capped` so retries are diffuse instead
        // of synchronized. jitter_fraction in [0,1] maps to [0.75, 1.25].
        let frac = jitter_fraction.clamp(0.0, 1.0);
        let multiplier = 0.75 + (frac * 0.5);
        let jittered = capped.mul_f64(multiplier);

        // Honor Retry-After when the server gave us one. Take the max so
        // we never retry sooner than the server asked, but jitter still
        // breaks up the herd.
        let server_hint = match err {
            SdkError::RateLimited { retry_after, .. } => *retry_after,
            _ => None,
        };
        let delay = match server_hint {
            Some(hint) => hint.max(jittered),
            None => jittered,
        };

        // Don't sleep past the budget — return the remaining budget if it
        // covers any positive duration so the caller still gets one last
        // attempt instead of bailing immediately.
        let remaining = self.total_budget.saturating_sub(elapsed);
        if remaining.is_zero() {
            return None;
        }
        Some(delay.min(remaining))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tl_core::{ApiError, ApiErrorCode};

    fn rate_limited(retry_after_secs: Option<u64>) -> SdkError {
        SdkError::RateLimited {
            error: ApiError {
                code: ApiErrorCode::RateLimited,
                message: "slow down".into(),
                retriable: true,
                details: serde_json::Value::Null,
            },
            retry_after: retry_after_secs.map(Duration::from_secs),
        }
    }

    fn unavailable() -> SdkError {
        SdkError::Unavailable(ApiError {
            code: ApiErrorCode::Unavailable,
            message: "upstream down".into(),
            retriable: true,
            details: serde_json::Value::Null,
        })
    }

    fn invalid() -> SdkError {
        SdkError::Invalid(ApiError {
            code: ApiErrorCode::Invalid,
            message: "bad input".into(),
            retriable: false,
            details: serde_json::Value::Null,
        })
    }

    #[test]
    fn non_retriable_errors_stop_immediately() {
        let cfg = RetryConfig::default();
        assert!(cfg.next_delay(1, Duration::ZERO, &invalid(), 0.5).is_none());
    }

    #[test]
    fn retries_unavailable_with_exponential_backoff() {
        let cfg = RetryConfig {
            max_attempts: 4,
            base_delay: Duration::from_millis(200),
            max_delay: Duration::from_secs(8),
            total_budget: Duration::from_secs(30),
        };
        // jitter=0.5 → multiplier 1.0, so delay equals capped exp.
        let d1 = cfg.next_delay(1, Duration::ZERO, &unavailable(), 0.5);
        let d2 = cfg.next_delay(2, Duration::from_millis(200), &unavailable(), 0.5);
        let d3 = cfg.next_delay(3, Duration::from_millis(600), &unavailable(), 0.5);
        assert_eq!(d1, Some(Duration::from_millis(200)));
        assert_eq!(d2, Some(Duration::from_millis(400)));
        assert_eq!(d3, Some(Duration::from_millis(800)));
    }

    #[test]
    fn caps_per_retry_delay_at_max_delay() {
        let cfg = RetryConfig {
            max_attempts: 10,
            base_delay: Duration::from_secs(1),
            max_delay: Duration::from_secs(4),
            total_budget: Duration::from_secs(60),
        };
        // attempt=5 → 1s * 2^4 = 16s, capped to 4s.
        let d = cfg
            .next_delay(5, Duration::ZERO, &unavailable(), 0.5)
            .unwrap();
        assert_eq!(d, Duration::from_secs(4));
    }

    #[test]
    fn honors_retry_after_when_longer_than_jittered() {
        let cfg = RetryConfig::default();
        // retry_after=10s ≫ jittered ~200ms; server hint wins.
        let d = cfg
            .next_delay(1, Duration::ZERO, &rate_limited(Some(10)), 0.5)
            .unwrap();
        assert!(d >= Duration::from_secs(10));
    }

    #[test]
    fn ignores_retry_after_when_jitter_already_longer() {
        // Retry-After of 0 with normal exponential — exp wins so callers
        // don't get hammered into a tight loop by a misconfigured server.
        let cfg = RetryConfig::default();
        let d = cfg
            .next_delay(3, Duration::ZERO, &rate_limited(Some(0)), 0.5)
            .unwrap();
        assert!(d >= Duration::from_millis(600));
    }

    #[test]
    fn stops_after_max_attempts() {
        let cfg = RetryConfig {
            max_attempts: 2,
            ..Default::default()
        };
        // attempt=2 means we already used 1 initial + 1 retry; no more.
        assert!(cfg
            .next_delay(2, Duration::ZERO, &unavailable(), 0.5)
            .is_none());
    }

    #[test]
    fn stops_when_budget_exhausted() {
        let cfg = RetryConfig {
            total_budget: Duration::from_secs(1),
            ..Default::default()
        };
        assert!(cfg
            .next_delay(1, Duration::from_secs(1), &unavailable(), 0.5)
            .is_none());
    }

    #[test]
    fn shrinks_last_delay_to_remaining_budget() {
        let cfg = RetryConfig {
            max_attempts: 5,
            base_delay: Duration::from_secs(2),
            max_delay: Duration::from_secs(10),
            total_budget: Duration::from_secs(3),
        };
        // Elapsed 2.5s, exp delay would be 2s — remaining is 0.5s.
        let d = cfg
            .next_delay(1, Duration::from_millis(2500), &unavailable(), 0.5)
            .unwrap();
        assert_eq!(d, Duration::from_millis(500));
    }

    #[test]
    fn jitter_fraction_clamps_to_unit_interval() {
        let cfg = RetryConfig::default();
        // Out-of-range fractions should not panic and should produce
        // delays inside [0.75x, 1.25x] of capped.
        let d_low = cfg
            .next_delay(1, Duration::ZERO, &unavailable(), -1.0)
            .unwrap();
        let d_hi = cfg
            .next_delay(1, Duration::ZERO, &unavailable(), 2.0)
            .unwrap();
        assert_eq!(d_low, Duration::from_millis(150));
        assert_eq!(d_hi, Duration::from_millis(250));
    }
}
