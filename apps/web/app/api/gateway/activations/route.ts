import { proxyRustCollection } from '@/lib/server/proxy-helpers';

export const runtime = 'nodejs';

export async function POST(req: Request) {
  return proxyRustCollection(req, '/v1/gateway/activations', 'POST');
}
