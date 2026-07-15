import { proxyRustJson } from '@/app/api/_shared';

export const runtime = 'nodejs';

type RouteContext = { params: Promise<{ id: string }> };

export async function GET(req: Request, context: RouteContext) {
  const { id } = await context.params;
  return proxyRustJson(req, `/v1/environments/${encodeURIComponent(id)}/checker-modes`);
}

export async function PUT(req: Request, context: RouteContext) {
  const { id } = await context.params;
  return proxyRustJson(req, `/v1/environments/${encodeURIComponent(id)}/checker-modes`, {
    method: 'PUT',
    body: await req.text(),
  });
}
