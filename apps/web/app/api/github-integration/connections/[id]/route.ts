import { proxyRustNoContent } from '@/app/api/_shared';

export const runtime = 'nodejs';

export async function DELETE(
  req: Request,
  { params }: { params: Promise<{ id: string }> },
) {
  const { id } = await params;
  return proxyRustNoContent(req, `/v1/github-integration/connections/${encodeURIComponent(id)}`);
}
