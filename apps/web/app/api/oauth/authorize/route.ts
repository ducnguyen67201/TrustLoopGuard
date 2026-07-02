import { NextResponse } from 'next/server';
import { z } from 'zod';

import { auth } from '@/auth';
import { rustApiForUserWorkspace, WorkspaceAccessError } from '@/lib/server/tl-client';

export const runtime = 'nodejs';

// The consent form POSTs here on Approve. We re-check the session, then ask
// tl-server to mint a PKCE-bound authorization code — authenticated with the
// internal key + forwarded user/workspace identity (the standard proxy
// pattern), so tl-server verifies the user owns the workspace.
const requestSchema = z.object({
  clientId: z.string().trim().min(1),
  redirectUri: z.string().url(),
  state: z.string().trim().min(1),
  codeChallenge: z.string().trim().min(1),
  workspaceId: z.string().trim().min(1),
});

export async function POST(req: Request) {
  const session = await auth();
  const user = session?.user as { id?: string; email?: string | null } | undefined;
  if (!user?.id) {
    return NextResponse.json({ error: 'authentication required' }, { status: 401 });
  }

  let body: unknown;
  try {
    body = await req.json();
  } catch {
    return NextResponse.json({ error: 'invalid JSON body' }, { status: 400 });
  }

  const parsed = requestSchema.safeParse(body);
  if (!parsed.success) {
    return NextResponse.json(
      { error: parsed.error.issues[0]?.message ?? 'invalid request' },
      { status: 400 },
    );
  }

  const { clientId, redirectUri, codeChallenge, workspaceId } = parsed.data;

  try {
    const { code } = await rustApiForUserWorkspace<{ code: string }>(
      { id: user.id, email: user.email ?? undefined },
      workspaceId,
      '/v1/oauth/authorize',
      {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({
          client_id: clientId,
          redirect_uri: redirectUri,
          code_challenge: codeChallenge,
        }),
      },
    );
    return NextResponse.json({ code });
  } catch (err) {
    if (err instanceof WorkspaceAccessError) {
      return NextResponse.json({ error: err.message }, { status: err.status });
    }
    return NextResponse.json({ error: 'authorization failed' }, { status: 500 });
  }
}
