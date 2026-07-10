import { deleteRustResource, patchRustResource } from '@/lib/server/proxy-helpers';

export const runtime = 'nodejs';

export async function PATCH(req: Request, { params }: { params: Promise<{ id: string }> }) {
  return patchRustResource(req, params, '/v1/gateway/provider-connections');
}

export async function DELETE(req: Request, { params }: { params: Promise<{ id: string }> }) {
  return deleteRustResource(req, params, '/v1/gateway/provider-connections');
}
