import { proxyRustResource } from '@/lib/server/proxy-helpers';
export const runtime = 'nodejs';
const prefix = '/v1/mcp-gateway/connections';
export async function PATCH(req: Request, { params }: { params: Promise<{ id: string }> }) { return proxyRustResource(req, params, prefix, 'PATCH'); }
export async function DELETE(req: Request, { params }: { params: Promise<{ id: string }> }) { return proxyRustResource(req, params, prefix, 'DELETE'); }
