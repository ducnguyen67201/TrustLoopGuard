import { proxyRustJson } from '@/app/api/_shared';

export const runtime = 'nodejs';

interface Ctx {
  params: Promise<{ id: string }>;
}

export async function GET(req: Request, ctx: Ctx) {
  const { id } = await ctx.params;
  return proxyRustJson(req, `/v1/agents/${encodeURIComponent(id)}/evaluation-profile`);
}

export async function PUT(req: Request, ctx: Ctx) {
  const { id } = await ctx.params;
  return proxyRustJson(req, `/v1/agents/${encodeURIComponent(id)}/evaluation-profile`, {
    method: 'PUT',
    headers: { 'content-type': 'application/json' },
    body: await req.text(),
  });
}
