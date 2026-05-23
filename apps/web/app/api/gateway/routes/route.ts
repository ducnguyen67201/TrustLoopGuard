import { proxyRustCollection } from '@/lib/server/proxy-helpers';

export const runtime = 'nodejs';

export async function GET(req: Request) {
  return proxyRustCollection(req, '/v1/gateway/routes', 'GET');
}

export async function POST(req: Request) {
  return proxyRustCollection(req, '/v1/gateway/routes', 'POST');
}
