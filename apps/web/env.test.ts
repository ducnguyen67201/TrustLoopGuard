import { beforeEach, describe, expect, it } from 'vitest';

process.env['SKIP_ENV_VALIDATION'] = 'true';

const { checkEnv, getAppUrl, getDocsUrl, getEnv, isHostedApprovalGateEnabled } =
  await import('./env');

describe('web environment helpers', () => {
  beforeEach(() => {
    clearEnv();
  });

  it('canonicalizes staging defaults', () => {
    process.env['NEXT_PUBLIC_APP_ENV'] = 'staging';

    expect(getEnv()).toBe('staging');
    expect(checkEnv()).toBe('stage');
    expect(getAppUrl()).toBe('https://staging3.gettrustloop.app');
    expect(getDocsUrl()).toBe('https://staging3.gettrustloop.app/apps/doc');
  });

  it('canonicalizes production defaults', () => {
    process.env['NEXT_PUBLIC_APP_ENV'] = 'prod';

    expect(getEnv()).toBe('prod');
    expect(getAppUrl()).toBe('https://app.gettrustloop.app');
    expect(getDocsUrl()).toBe('https://app.gettrustloop.app/apps/doc');
  });

  it('canonicalizes local development defaults', () => {
    process.env['NEXT_PUBLIC_APP_ENV'] = 'dev';

    expect(getEnv()).toBe('dev');
    expect(getAppUrl()).toBe('http://localhost:3000');
    expect(getDocsUrl()).toBe('http://localhost:3001/docs');
  });

  it('lets AUTH_URL override the app URL default', () => {
    process.env['NEXT_PUBLIC_APP_ENV'] = 'staging';
    process.env['AUTH_URL'] = 'https://custom-staging.example';

    expect(getAppUrl()).toBe('https://custom-staging.example');
  });

  it('arms the approval gate on the hosted flag alone — dev has no bypass', () => {
    expect(isHostedApprovalGateEnabled('true')).toBe(true);
    expect(isHostedApprovalGateEnabled('hosted')).toBe(true);
    expect(isHostedApprovalGateEnabled('1')).toBe(true);
    expect(isHostedApprovalGateEnabled(undefined)).toBe(false);
    expect(isHostedApprovalGateEnabled('false')).toBe(false);
    expect(isHostedApprovalGateEnabled('')).toBe(false);
  });
});

function clearEnv() {
  delete process.env['APP_ENV'];
  delete process.env['AUTH_URL'];
  delete process.env['NEXTAUTH_URL'];
  delete process.env['NEXT_PUBLIC_APP_ENV'];
  delete process.env['TL_HOSTED_DEPLOYMENT'];
  delete process.env['VERCEL_ENV'];
}
