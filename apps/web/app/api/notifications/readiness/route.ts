import { proxyRustCollection } from '@/lib/server/proxy-helpers';

export async function GET(req: Request) {
  return proxyRustCollection(req, '/v1/notifications/readiness', 'GET');
}
