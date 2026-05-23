import { forwardedQuery, proxyRustJson } from '../_shared';

export const runtime = 'nodejs';

export async function GET(req: Request) {
  const url = new URL(req.url);
  return proxyRustJson(req, `/v1/analytics/views${forwardedQuery(url.searchParams)}`);
}

export async function POST(req: Request) {
  const body = await req.text();
  return proxyRustJson(req, '/v1/analytics/views', {
    method: 'POST',
    headers: { 'Content-Type': 'application/json' },
    body,
  });
}
