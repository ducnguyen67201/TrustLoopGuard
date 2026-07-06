import { proxyRustJson } from '@/app/api/_shared';

export const runtime = 'nodejs';

export async function GET(req: Request) {
  return proxyRustJson(req, '/v1/financial/approval-requests');
}
