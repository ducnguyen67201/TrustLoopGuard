import 'server-only';
import { NextResponse } from 'next/server';

import { RustApiError, rustApiForWorkspace, workspaceIdFromSlug } from '@/lib/server/tl-client';

function handleRustError(err: unknown): Response {
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

export async function proxyRustCollection(
  req: Request,
  rustPath: string,
  method: 'GET' | 'POST',
): Promise<Response> {
  try {
    const workspaceSlug = new URL(req.url).searchParams.get('workspace')?.trim();
    const workspaceId = workspaceIdFromSlug(workspaceSlug);
    const init: RequestInit = { method };
    if (method !== 'GET') {
      init.headers = { 'Content-Type': 'application/json' };
      init.body = await req.text();
    }
    const data = await rustApiForWorkspace<unknown>(workspaceId, rustPath, init);
    return NextResponse.json(data, { status: method === 'POST' ? 201 : 200 });
  } catch (err) {
    return handleRustError(err);
  }
}

export async function patchRustResource(
  req: Request,
  params: Promise<{ id: string }>,
  rustPathPrefix: string,
): Promise<Response> {
  try {
    const { id } = await params;
    const workspaceSlug = new URL(req.url).searchParams.get('workspace')?.trim();
    const workspaceId = workspaceIdFromSlug(workspaceSlug);
    const data = await rustApiForWorkspace<unknown>(
      workspaceId,
      `${rustPathPrefix}/${encodeURIComponent(id)}`,
      {
        method: 'PATCH',
        headers: { 'Content-Type': 'application/json' },
        body: await req.text(),
      },
    );
    return NextResponse.json(data);
  } catch (err) {
    return handleRustError(err);
  }
}
