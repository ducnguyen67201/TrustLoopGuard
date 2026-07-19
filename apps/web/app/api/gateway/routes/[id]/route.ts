import { proxyRustResource } from '@/lib/server/proxy-helpers';

export const runtime = 'nodejs';

export async function PATCH(
  req: Request,
  { params }: { params: Promise<{ id: string }> },
) {
  return proxyRustResource(req, params, '/v1/gateway/routes', 'PATCH');
}
