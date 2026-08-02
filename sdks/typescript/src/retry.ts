// Retry policy for the Featherlane AI TypeScript SDK.
//
// Mirrors `tl-sdk-rust`'s `RetryConfig` exactly. Same defaults
// (4 attempts, 30s budget, 200ms base, 8s cap), same `nextDelay`
// contract: given the attempt number, elapsed time, the error that just
// occurred, and a jitter fraction, return the delay before the next
// attempt (in seconds) or `undefined` to stop.
//
// The function is pure so it can be unit-tested without spinning up an
// HTTP server. Production callers feed `Math.random()`; tests pin the
// value for determinism.

import { RateLimited, SdkError } from './errors.js';

export interface RetryConfig {
  /** Total attempts including the initial one. `1` = no retry. */
  maxAttempts: number;
  /** Hard cap on total wall time including sleeps, in seconds. */
  totalBudgetS: number;
  /** Base for exponential backoff: delay = base * 2^(attempt-1). */
  baseDelayS: number;
  /** Cap on a single retry delay; prevents runaway exponential. */
  maxDelayS: number;
}

export const DEFAULT_RETRY: Readonly<RetryConfig> = Object.freeze({
  maxAttempts: 4,
  totalBudgetS: 30.0,
  baseDelayS: 0.2,
  maxDelayS: 8.0,
});

/**
 * Compute the delay before the next attempt, or `undefined` to stop.
 * Pure mirror of `tl_sdk_rust::RetryConfig::next_delay` and
 * `featherlane_ai.retry.RetryConfig.next_delay`.
 */
export function nextDelay(
  cfg: RetryConfig,
  attempt: number,
  elapsedS: number,
  err: SdkError,
  jitterFraction: number,
): number | undefined {
  if (!err.isRetriable()) return undefined;
  if (attempt >= cfg.maxAttempts) return undefined;
  if (elapsedS >= cfg.totalBudgetS) return undefined;

  const exp = cfg.baseDelayS * 2 ** (attempt - 1);
  const capped = Math.min(exp, cfg.maxDelayS);

  // ±25% jitter: jitterFraction in [0,1] maps to multiplier [0.75, 1.25].
  const frac = Math.max(0, Math.min(1, jitterFraction));
  const multiplier = 0.75 + frac * 0.5;
  const jittered = capped * multiplier;

  let delay: number;
  if (err instanceof RateLimited && err.retryAfter !== undefined) {
    delay = Math.max(err.retryAfter, jittered);
  } else {
    delay = jittered;
  }

  const remaining = cfg.totalBudgetS - elapsedS;
  if (remaining <= 0) return undefined;
  return Math.min(delay, remaining);
}
