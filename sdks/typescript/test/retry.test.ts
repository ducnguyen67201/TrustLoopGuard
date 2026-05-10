// Retry-policy tests for the TypeScript SDK.
// Mirrors tl-sdk-rust's tests in src/retry.rs and the Python SDK's
// test_retry.py. These three suites must agree on every number — a
// divergence breaks the parity claim in docs/SDK_DRIVEN.md.

import { describe, expect, it } from 'vitest';

import {
  DEFAULT_RETRY,
  Invalid,
  RateLimited,
  Unavailable,
  type RetryConfig,
  nextDelay,
} from '../src';

const rateLimited = (retryAfter?: number) =>
  new RateLimited(
    {
      code: 'rate_limited',
      message: 'slow down',
      retriable: true,
      details: null,
    },
    retryAfter,
  );

const unavailable = () =>
  new Unavailable({
    code: 'unavailable',
    message: 'upstream down',
    retriable: true,
    details: null,
  });

const invalid = () =>
  new Invalid({
    code: 'invalid',
    message: 'bad input',
    retriable: false,
    details: null,
  });

const cfg: RetryConfig = {
  maxAttempts: 4,
  baseDelayS: 0.2,
  maxDelayS: 8.0,
  totalBudgetS: 30.0,
};

describe('nextDelay', () => {
  it('non-retriable errors stop immediately', () => {
    expect(nextDelay(cfg, 1, 0, invalid(), 0.5)).toBeUndefined();
  });

  it('retries Unavailable with exponential backoff', () => {
    expect(nextDelay(cfg, 1, 0, unavailable(), 0.5)).toBeCloseTo(0.2);
    expect(nextDelay(cfg, 2, 0.2, unavailable(), 0.5)).toBeCloseTo(0.4);
    expect(nextDelay(cfg, 3, 0.6, unavailable(), 0.5)).toBeCloseTo(0.8);
  });

  it('caps per-retry delay at maxDelayS', () => {
    const c: RetryConfig = {
      maxAttempts: 10,
      baseDelayS: 1.0,
      maxDelayS: 4.0,
      totalBudgetS: 60.0,
    };
    expect(nextDelay(c, 5, 0, unavailable(), 0.5)).toBeCloseTo(4.0);
  });

  it('honors Retry-After when longer than jittered', () => {
    const d = nextDelay(cfg, 1, 0, rateLimited(10), 0.5);
    expect(d).toBeDefined();
    expect(d!).toBeGreaterThanOrEqual(10);
  });

  it('ignores Retry-After when jitter is already longer', () => {
    const d = nextDelay(cfg, 3, 0, rateLimited(0), 0.5);
    expect(d).toBeDefined();
    expect(d!).toBeGreaterThanOrEqual(0.6);
  });

  it('stops after max_attempts', () => {
    const c: RetryConfig = { ...cfg, maxAttempts: 2 };
    expect(nextDelay(c, 2, 0, unavailable(), 0.5)).toBeUndefined();
  });

  it('stops when budget exhausted', () => {
    const c: RetryConfig = { ...cfg, totalBudgetS: 1.0 };
    expect(nextDelay(c, 1, 1.0, unavailable(), 0.5)).toBeUndefined();
  });

  it('shrinks last delay to remaining budget', () => {
    const c: RetryConfig = {
      maxAttempts: 5,
      baseDelayS: 2.0,
      maxDelayS: 10.0,
      totalBudgetS: 3.0,
    };
    const d = nextDelay(c, 1, 2.5, unavailable(), 0.5);
    expect(d).toBeCloseTo(0.5, 6);
  });

  it('jitter fraction clamps to unit interval', () => {
    expect(nextDelay(cfg, 1, 0, unavailable(), -1)).toBeCloseTo(0.15);
    expect(nextDelay(cfg, 1, 0, unavailable(), 2)).toBeCloseTo(0.25);
  });

  it('default config matches the cross-SDK contract', () => {
    expect(DEFAULT_RETRY.maxAttempts).toBe(4);
    expect(DEFAULT_RETRY.totalBudgetS).toBe(30.0);
    expect(DEFAULT_RETRY.baseDelayS).toBe(0.2);
    expect(DEFAULT_RETRY.maxDelayS).toBe(8.0);
  });
});
