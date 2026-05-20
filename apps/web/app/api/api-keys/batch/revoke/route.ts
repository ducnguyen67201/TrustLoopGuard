import { NextResponse } from 'next/server';
import { z } from 'zod';

import { rustApiForAuthorizedWorkspace, WorkspaceAccessError } from '@/lib/server/tl-client';

export const runtime = 'nodejs';

type JsonValue = string | number | boolean | null | JsonValue[] | { [key: string]: JsonValue };

const bodySchema = z.object({
  ids: z.array(z.string().trim().min(1)).min(1),
});

export async function PATCH(req: Request) {
  let body: JsonValue;
  try {
    body = JSON.parse(await req.text()) as JsonValue;
  } catch {
    return NextResponse.json({ error: 'invalid JSON' }, { status: 400 });
  }

  const parsed = bodySchema.safeParse(body);
  if (!parsed.success) {
    return NextResponse.json({ error: 'expected { ids: string[] }' }, { status: 400 });
  }

  try {
    const result = await rustApiForAuthorizedWorkspace(req, '/v1/api-keys/batch/revoke', {
      method: 'PATCH',
      headers: { 'content-type': 'application/json' },
      body: JSON.stringify(parsed.data),
    });
    return NextResponse.json(result);
  } catch (err) {
    if (err instanceof WorkspaceAccessError) {
      return NextResponse.json({ error: err.message }, { status: err.status });
    }
    const message = err instanceof Error ? err.message : 'unknown error';
    return NextResponse.json({ error: message }, { status: 502 });
  }
}
