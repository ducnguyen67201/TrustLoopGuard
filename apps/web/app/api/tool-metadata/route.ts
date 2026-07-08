import { proxyRustJson } from '@/app/api/_shared';

export const runtime = 'nodejs';

export async function POST(req: Request) {
  return proxyRustJson(req, '/v1/tool-metadata', {
    method: 'POST',
    headers: { 'content-type': 'application/json' },
    body: await req.text(),
  });
}
