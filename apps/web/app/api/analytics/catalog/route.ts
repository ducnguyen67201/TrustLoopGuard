import { forwardedQuery, proxyRustJson } from '../_shared';

export const runtime = 'nodejs';

export async function GET(req: Request) {
  const url = new URL(req.url);
  return proxyRustJson(req, `/v1/analytics/catalog${forwardedQuery(url.searchParams)}`);
}
