import { putRustResource } from '@/lib/server/proxy-helpers';
export const runtime = 'nodejs';
export async function PUT(req: Request, { params }: { params: Promise<{ id: string }> }) { return putRustResource(req, params, '/v1/mcp-gateway/tools', 'assignments'); }
