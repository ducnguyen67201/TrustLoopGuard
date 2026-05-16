import { NextResponse } from 'next/server';

import { rustApiForWorkspace, workspaceIdFromSlug } from '@/lib/server/tl-client';

export const runtime = 'nodejs';

interface CreateApiKeyResponse {
  api_key: unknown;
  plaintext_key: string;
}

export async function POST(req: Request) {
  try {
    const workspaceSlug = new URL(req.url).searchParams.get('workspace')?.trim();
    const workspaceId = workspaceIdFromSlug(workspaceSlug);
    const body = await req.text();
    const data = await rustApiForWorkspace<CreateApiKeyResponse>(workspaceId, '/v1/api-keys', {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body,
    });
    return NextResponse.json(data, { status: 201 });
  } catch (err) {
    const message = err instanceof Error ? err.message : 'unknown error';
    return NextResponse.json({ error: message }, { status: 502 });
  }
}
