import { safeAuthRedirect, type AuthRedirectConfig } from './auth-redirect';
import { describe, expect, it } from 'vitest';

const config: AuthRedirectConfig = {
  appUrl: 'https://app.featherlane.ai',
  serverUrl: 'http://server:8080',
  publicServerUrl: 'https://api.featherlane.ai',
};

describe('safeAuthRedirect', () => {
  it('preserves trusted dashboard redirects', () => {
    expect(safeAuthRedirect('/dashboard?workspace=featherlane_ai', config)).toBe(
      'https://app.featherlane.ai/dashboard?workspace=featherlane_ai',
    );

    expect(safeAuthRedirect('https://app.featherlane.ai/policies', config)).toBe(
      'https://app.featherlane.ai/policies',
    );
  });

  it('rewrites Rust API callbacks onto the dashboard origin', () => {
    expect(safeAuthRedirect('http://0.0.0.0:8080/welcome?from=signin', config)).toBe(
      'https://app.featherlane.ai/welcome?from=signin',
    );

    expect(safeAuthRedirect('http://localhost:8080/runs/abc', config)).toBe(
      'https://app.featherlane.ai/runs/abc',
    );

    expect(safeAuthRedirect('https://api.featherlane.ai/account', config)).toBe(
      'https://app.featherlane.ai/account',
    );
  });

  it('rejects unknown external redirects', () => {
    expect(safeAuthRedirect('https://evil.example/phish', config)).toBe(
      'https://app.featherlane.ai/',
    );
    expect(safeAuthRedirect('//evil.example/phish', config)).toBe(
      'https://app.featherlane.ai/',
    );
  });
});
