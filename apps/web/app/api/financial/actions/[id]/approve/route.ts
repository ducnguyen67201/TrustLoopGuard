import { proxyRustJson } from '@/app/api/_shared';

export const runtime = 'nodejs';

type RouteContext = {
  params: Promise<{ id: string }>;
};

export async function POST(req: Request, context: RouteContext) {
  const { id } = await context.params;
  return proxyRustJson(
    req,
    `/v1/financial/actions/${encodeURIComponent(id)}/approve`,
    { method: 'POST' },
  );
}
