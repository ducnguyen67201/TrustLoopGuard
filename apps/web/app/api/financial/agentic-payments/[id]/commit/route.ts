import { proxyRustJson } from '@/app/api/_shared';

export const runtime = 'nodejs';

type RouteContext = {
  params: Promise<{ id: string }>;
};

export async function POST(req: Request, context: RouteContext) {
  const { id } = await context.params;
  return proxyRustJson(req, `/v1/financial/agentic-payments/${encodeURIComponent(id)}/commit`, {
    method: 'POST',
    headers: { 'Content-Type': 'application/json' },
    body: await req.text(),
  });
}
