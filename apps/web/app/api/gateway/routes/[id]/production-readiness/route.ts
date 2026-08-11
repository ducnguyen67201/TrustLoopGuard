import { proxyRustCollection } from '@/lib/server/proxy-helpers';
import { forwardedQuery } from '@/app/api/_shared';

export const runtime = 'nodejs';

export async function GET(req: Request, { params }: { params: Promise<{ id: string }> }) {
  const { id } = await params;
  const rustQuery = forwardedQuery(new URL(req.url).searchParams);
  return proxyRustCollection(
    req,
    `/v1/gateway/routes/${encodeURIComponent(id)}/production-readiness${rustQuery}`,
    'GET',
  );
}
