import { proxyRustJson } from '../_shared';

export const runtime = 'nodejs';

export async function POST(req: Request) {
  const body = await req.text();
  return proxyRustJson(req, '/v1/analytics/query', {
    method: 'POST',
    headers: { 'Content-Type': 'application/json' },
    body,
  });
}
