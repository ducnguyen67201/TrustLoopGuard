import { beforeEach, describe, expect, it } from 'vitest';

process.env['SKIP_ENV_VALIDATION'] = 'true';

const { isCredentialsAuthEnabled } = await import('./auth-capabilities');

describe('auth capabilities', () => {
  beforeEach(() => {
    clearEnv();
  });

  it('enables credentials auth in local development', () => {
    process.env['NEXT_PUBLIC_APP_ENV'] = 'dev';

    expect(isCredentialsAuthEnabled()).toBe(true);
  });

  it('disables credentials auth in staging', () => {
    process.env['NEXT_PUBLIC_APP_ENV'] = 'staging';

    expect(isCredentialsAuthEnabled()).toBe(false);
  });

  it('disables credentials auth in production', () => {
    process.env['NEXT_PUBLIC_APP_ENV'] = 'prod';

    expect(isCredentialsAuthEnabled()).toBe(false);
  });
});

function clearEnv() {
  delete process.env['APP_ENV'];
  delete process.env['NEXT_PUBLIC_APP_ENV'];
  delete process.env['VERCEL_ENV'];
}
