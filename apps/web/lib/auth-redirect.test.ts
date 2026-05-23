import { safeAuthRedirect, type AuthRedirectConfig } from './auth-redirect';

const config: AuthRedirectConfig = {
  appUrl: 'https://app.gettrustloop.app',
  serverUrl: 'http://server:8080',
  publicServerUrl: 'https://api.gettrustloop.app',
};

assertEqual(
  safeAuthRedirect('/dashboard?workspace=trustloop', config),
  'https://app.gettrustloop.app/dashboard?workspace=trustloop',
  'relative callback redirects to the dashboard origin',
);

assertEqual(
  safeAuthRedirect('https://app.gettrustloop.app/policies', config),
  'https://app.gettrustloop.app/policies',
  'same-origin dashboard redirect is preserved',
);

assertEqual(
  safeAuthRedirect('http://0.0.0.0:8080/welcome?from=signin', config),
  'https://app.gettrustloop.app/welcome?from=signin',
  '0.0.0.0:8080 callback is rewritten to the dashboard origin',
);

assertEqual(
  safeAuthRedirect('http://localhost:8080/runs/abc', config),
  'https://app.gettrustloop.app/runs/abc',
  'localhost Rust callback is rewritten to the dashboard origin',
);

assertEqual(
  safeAuthRedirect('https://api.gettrustloop.app/account', config),
  'https://app.gettrustloop.app/account',
  'public Rust API callback is rewritten to the dashboard origin',
);

assertEqual(
  safeAuthRedirect('https://evil.example/phish', config),
  'https://app.gettrustloop.app/',
  'unknown external origins fall back to the dashboard root',
);

assertEqual(
  safeAuthRedirect('//evil.example/phish', config),
  'https://app.gettrustloop.app/',
  'protocol-relative redirects are rejected',
);

function assertEqual(actual: string, expected: string, message: string) {
  if (actual !== expected) {
    throw new Error(`${message}: expected ${expected}, got ${actual}`);
  }
}
