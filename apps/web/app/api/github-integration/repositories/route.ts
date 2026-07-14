import { forwardedQuery, proxyRustJson } from '@/app/api/_shared';

export const runtime = 'nodejs';

export async function GET(req: Request) {
  const url = new URL(req.url);
  return proxyRustJson(req, `/v1/github-integration/repositories${forwardedQuery(url.searchParams)}`);
}
