import { proxyRustJson } from '@/app/api/_shared';

export const runtime = 'nodejs';

interface Ctx {
  params: Promise<{ id: string }>;
}

export async function POST(req: Request, ctx: Ctx) {
  const { id } = await ctx.params;
  return proxyRustJson(req, `/v1/runs/${encodeURIComponent(id)}/finalize`, {
    method: 'POST',
    headers: { 'content-type': 'application/json' },
    body: await req.text(),
  });
}
