import { NextResponse } from 'next/server';

import { RustApiError, rustApiForWorkspace, workspaceIdFromSlug } from '@/lib/server/tl-client';

export const runtime = 'nodejs';

export async function GET(req: Request) {
  return proxy(req, '/v1/enforcement-profiles', 'GET');
}

export async function POST(req: Request) {
  return proxy(req, '/v1/enforcement-profiles', 'POST');
}

async function proxy(req: Request, path: string, method: 'GET' | 'POST') {
  try {
    const workspaceSlug = new URL(req.url).searchParams.get('workspace')?.trim();
    const workspaceId = workspaceIdFromSlug(workspaceSlug);
    const init: RequestInit = { method };
    if (method !== 'GET') {
      init.headers = { 'Content-Type': 'application/json' };
      init.body = await req.text();
    }
    const data = await rustApiForWorkspace<unknown>(workspaceId, path, init);
    return NextResponse.json(data, { status: method === 'POST' ? 201 : 200 });
  } catch (err) {
    if (err instanceof RustApiError) {
      const status = err.status >= 500 ? 502 : err.status;
      try {
        return NextResponse.json(JSON.parse(err.body), { status });
      } catch {
        return NextResponse.json({ error: 'upstream error' }, { status });
      }
    }
    return NextResponse.json({ error: 'upstream error' }, { status: 502 });
  }
}
