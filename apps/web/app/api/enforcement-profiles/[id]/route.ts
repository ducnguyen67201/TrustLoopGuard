import { patchRustResource } from '@/lib/server/proxy-helpers';

export const runtime = 'nodejs';

export async function PATCH(
  req: Request,
  { params }: { params: Promise<{ id: string }> },
) {
  return patchRustResource(req, params, '/v1/enforcement-profiles');
}
