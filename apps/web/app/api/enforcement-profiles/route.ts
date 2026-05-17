import { NextResponse } from 'next/server';

import { rustApiForWorkspace, workspaceIdFromSlug } from '@/lib/server/tl-client';

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
    const message = err instanceof Error ? err.message : 'unknown error';
    return NextResponse.json({ error: message }, { status: 502 });
  }
}
