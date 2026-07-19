import { proxyRustResourceAction } from '@/lib/server/proxy-helpers';
export const runtime = 'nodejs';
export async function POST(req: Request, { params }: { params: Promise<{ id: string }> }) { return proxyRustResourceAction(req, params, '/v1/mcp-gateway/connections', 'sync'); }
