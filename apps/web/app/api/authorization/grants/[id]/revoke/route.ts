import { proxyRustJson } from '@/app/api/_shared';

export const runtime = 'nodejs';

type RouteContext = { params: Promise<{ id: string }> };

export async function POST(req: Request, context: RouteContext) {
  const { id } = await context.params;
  return proxyRustJson(req, `/v1/authorization/grants/${encodeURIComponent(id)}/revoke`, {
    method: 'POST',
  });
}
