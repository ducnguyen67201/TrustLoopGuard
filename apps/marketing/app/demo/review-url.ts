const LOCAL_DASHBOARD_URL = 'http://localhost:3000';
const PRODUCTION_DASHBOARD_URL = 'https://app.gettrustloop.app';

export function refundDemoReviewUrl(
  actionId: string,
  dashboardBaseUrl = process.env['NEXT_PUBLIC_TRUSTLOOPGUARD_APP_URL'] ?? defaultDashboardUrl(),
): string {
  const url = new URL('/financial', dashboardBaseUrl);
  url.searchParams.set('workspace', 'default');
  url.searchParams.set('environment', 'production');
  url.searchParams.set('actionId', actionId);
  return url.toString();
}

function defaultDashboardUrl(): string {
  return process.env.NODE_ENV === 'production' ? PRODUCTION_DASHBOARD_URL : LOCAL_DASHBOARD_URL;
}
