import { forwardedQuery, proxyRustJson } from '@/app/api/_shared';

export const runtime = 'nodejs';

export async function GET(req: Request) {
  const query = forwardedQuery(new URL(req.url).searchParams);
  return proxyRustJson(req, `/v1/financial/observations/summary${query}`);
}
