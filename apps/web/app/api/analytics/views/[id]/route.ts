import { proxyRustJson, proxyRustNoContent } from '../../_shared';

export const runtime = 'nodejs';

export async function PATCH(req: Request, { params }: { params: Promise<{ id: string }> }) {
  const { id } = await params;
  const body = await req.text();
  return proxyRustJson(req, `/v1/analytics/views/${encodeURIComponent(id)}`, {
    method: 'PATCH',
    headers: { 'Content-Type': 'application/json' },
    body,
  });
}

export async function DELETE(req: Request, { params }: { params: Promise<{ id: string }> }) {
  const { id } = await params;
  return proxyRustNoContent(req, `/v1/analytics/views/${encodeURIComponent(id)}`);
}
