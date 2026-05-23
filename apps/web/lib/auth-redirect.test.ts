import { safeAuthRedirect, type AuthRedirectConfig } from './auth-redirect';
import { describe, expect, it } from 'vitest';

const config: AuthRedirectConfig = {
  appUrl: 'https://app.gettrustloop.app',
  serverUrl: 'http://server:8080',
  publicServerUrl: 'https://api.gettrustloop.app',
};

describe('safeAuthRedirect', () => {
  it('preserves trusted dashboard redirects', () => {
    expect(safeAuthRedirect('/dashboard?workspace=trustloop', config)).toBe(
      'https://app.gettrustloop.app/dashboard?workspace=trustloop',
    );

    expect(safeAuthRedirect('https://app.gettrustloop.app/policies', config)).toBe(
      'https://app.gettrustloop.app/policies',
    );
  });

  it('rewrites Rust API callbacks onto the dashboard origin', () => {
    expect(safeAuthRedirect('http://0.0.0.0:8080/welcome?from=signin', config)).toBe(
      'https://app.gettrustloop.app/welcome?from=signin',
    );

    expect(safeAuthRedirect('http://localhost:8080/runs/abc', config)).toBe(
      'https://app.gettrustloop.app/runs/abc',
    );

    expect(safeAuthRedirect('https://api.gettrustloop.app/account', config)).toBe(
      'https://app.gettrustloop.app/account',
    );
  });

  it('rejects unknown external redirects', () => {
    expect(safeAuthRedirect('https://evil.example/phish', config)).toBe(
      'https://app.gettrustloop.app/',
    );
    expect(safeAuthRedirect('//evil.example/phish', config)).toBe(
      'https://app.gettrustloop.app/',
    );
  });
});
