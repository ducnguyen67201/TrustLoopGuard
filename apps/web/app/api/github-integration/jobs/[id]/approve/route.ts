import { proxyRustJson } from '@/app/api/_shared';

export const runtime = 'nodejs';

export async function POST(
  req: Request,
  { params }: { params: Promise<{ id: string }> },
) {
  const { id } = await params;
  return proxyRustJson(req, `/v1/github-integration/jobs/${encodeURIComponent(id)}/approve`, {
    method: 'POST',
  });
}
