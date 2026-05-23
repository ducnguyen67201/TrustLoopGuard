import { describe, expect, it } from 'vitest';

import { authSignOutRedirectUrl, isAuthSignOutGet } from './auth-signout-route';

describe('auth signout route helpers', () => {
  it('matches only the Auth.js signout endpoint', () => {
    expect(isAuthSignOutGet('/api/auth/signout')).toBe(true);
    expect(isAuthSignOutGet('/api/auth/session')).toBe(false);
    expect(isAuthSignOutGet('/signout')).toBe(false);
  });

  it('redirects direct Auth.js signout visits to the branded signout page', () => {
    expect(
      authSignOutRedirectUrl(
        'https://app.gettrustloop.app/api/auth/signout?callbackUrl=%2Fsettings#confirm',
      ).toString(),
    ).toBe('https://app.gettrustloop.app/signout');
  });
});
