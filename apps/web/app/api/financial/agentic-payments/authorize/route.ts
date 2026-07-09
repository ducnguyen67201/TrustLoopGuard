import { proxyRustJson } from '@/app/api/_shared';

export const runtime = 'nodejs';

export async function POST(req: Request) {
  return proxyRustJson(req, '/v1/financial/agentic-payments/authorize', {
    method: 'POST',
    headers: { 'Content-Type': 'application/json' },
    body: await req.text(),
  });
}
