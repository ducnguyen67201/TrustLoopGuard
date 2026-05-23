process.env['SKIP_ENV_VALIDATION'] = 'true';

const { checkEnv, getAppUrl, getDocsUrl, getEnv } = await import('./env');

clearEnv();
process.env['NEXT_PUBLIC_APP_ENV'] = 'staging';
assertEqual(getEnv(), 'staging', 'staging app env is canonicalized');
assertEqual(checkEnv(), 'stage', 'legacy checkEnv stage value is preserved');
assertEqual(getAppUrl(), 'https://staging3.gettrustloop.app', 'staging app URL default');
assertEqual(
  getDocsUrl(),
  'https://staging3.gettrustloop.app/apps/doc',
  'staging docs URL default',
);

clearEnv();
process.env['NEXT_PUBLIC_APP_ENV'] = 'prod';
assertEqual(getEnv(), 'prod', 'prod app env is canonicalized');
assertEqual(getAppUrl(), 'https://app.gettrustloop.app', 'prod app URL default');
assertEqual(getDocsUrl(), 'https://app.gettrustloop.app/apps/doc', 'prod docs URL default');

clearEnv();
process.env['NEXT_PUBLIC_APP_ENV'] = 'dev';
assertEqual(getEnv(), 'dev', 'dev app env is canonicalized');
assertEqual(getAppUrl(), 'http://localhost:3000', 'dev app URL default');
assertEqual(getDocsUrl(), 'http://localhost:3001/docs', 'dev docs URL default');

clearEnv();
process.env['NEXT_PUBLIC_APP_ENV'] = 'staging';
process.env['AUTH_URL'] = 'https://custom-staging.example';
assertEqual(getAppUrl(), 'https://custom-staging.example', 'AUTH_URL overrides app URL default');

function clearEnv() {
  delete process.env['APP_ENV'];
  delete process.env['AUTH_URL'];
  delete process.env['NEXTAUTH_URL'];
  delete process.env['NEXT_PUBLIC_APP_ENV'];
  delete process.env['VERCEL_ENV'];
}

function assertEqual(actual: string, expected: string, message: string) {
  if (actual !== expected) {
    throw new Error(`${message}: expected ${expected}, got ${actual}`);
  }
}
