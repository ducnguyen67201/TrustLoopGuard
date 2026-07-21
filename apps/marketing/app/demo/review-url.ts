import { APP_URL } from '@/lib/app-url';

export function refundDemoReviewUrl(
  actionId: string,
  dashboardBaseUrl = APP_URL,
): string {
  const url = new URL('/financial', dashboardBaseUrl);
  url.searchParams.set('workspace', 'default');
  url.searchParams.set('environment', 'production');
  url.searchParams.set('actionId', actionId);
  return url.toString();
}
