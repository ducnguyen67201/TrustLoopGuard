import { NextResponse } from 'next/server';

import { rustApiForAuthorizedWorkspace, WorkspaceAccessError } from '@/lib/server/tl-client';

export const runtime = 'nodejs';

interface CreateApiKeyResponse {
  api_key: unknown;
  plaintext_key: string;
}

export async function POST(req: Request) {
  try {
    const body = await req.text();
    const data = await rustApiForAuthorizedWorkspace<CreateApiKeyResponse>(req, '/v1/api-keys', {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body,
    });
    return NextResponse.json(data, { status: 201 });
  } catch (err) {
    if (err instanceof WorkspaceAccessError) {
      return NextResponse.json({ error: err.message }, { status: err.status });
    }
    const message = err instanceof Error ? err.message : 'unknown error';
    return NextResponse.json({ error: message }, { status: 502 });
  }
}
