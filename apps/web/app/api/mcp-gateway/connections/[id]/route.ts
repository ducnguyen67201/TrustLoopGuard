import { deleteRustResource, patchRustResource } from '@/lib/server/proxy-helpers';
export const runtime = 'nodejs';
const prefix = '/v1/mcp-gateway/connections';
export async function PATCH(req: Request, { params }: { params: Promise<{ id: string }> }) { return patchRustResource(req, params, prefix); }
export async function DELETE(req: Request, { params }: { params: Promise<{ id: string }> }) { return deleteRustResource(req, params, prefix); }
