import { forwardedQuery, proxyRustJson } from '@/app/api/_shared';

export const runtime = 'nodejs';

type RouteContext = {
  params: Promise<{ id: string }>;
};

export async function GET(req: Request, context: RouteContext) {
  const { id } = await context.params;
  const url = new URL(req.url);
  return proxyRustJson(
    req,
    `/v1/traces/${encodeURIComponent(id)}${forwardedQuery(url.searchParams)}`,
  );
}
