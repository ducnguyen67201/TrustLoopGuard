import { NextResponse } from 'next/server';

import { auth } from '@/auth';
import { rustApiForUser } from '@/lib/server/tl-client';

export const runtime = 'nodejs';

interface MyWorkspace {
  id: string;
  slug: string;
  name: string;
  role: string;
  organization_id: string;
}

interface MyWorkspacesResponse {
  workspaces: MyWorkspace[];
}

function userFromSession(
  sessionUser:
    | { id?: string; email?: string | null; tlJwt?: string }
    | undefined,
): { id: string; email?: string | null; tlJwt?: string } | null {
  if (sessionUser?.id === undefined || sessionUser.id === '') return null;
  const out: { id: string; email?: string | null; tlJwt?: string } = {
    id: sessionUser.id,
  };
  if (sessionUser.email !== undefined && sessionUser.email !== null) {
    out.email = sessionUser.email;
  }
  if (sessionUser.tlJwt !== undefined && sessionUser.tlJwt !== '') {
    out.tlJwt = sessionUser.tlJwt;
  }
  return out;
}

export async function GET() {
  try {
    const session = await auth();
    const user = userFromSession(session?.user as never);
    if (user === null) {
      return NextResponse.json({ workspaces: [] }, { status: 401 });
    }
    const data = await rustApiForUser<MyWorkspacesResponse>(
      user,
      '/v1/team/my-workspaces',
    );
    return NextResponse.json(data);
  } catch (err) {
    const message = err instanceof Error ? err.message : 'unknown error';
    return NextResponse.json({ error: message, workspaces: [] }, { status: 502 });
  }
}

export async function POST(req: Request) {
  try {
    const session = await auth();
    const user = userFromSession(session?.user as never);
    if (user === null) {
      return NextResponse.json({ error: 'unauthenticated' }, { status: 401 });
    }
    const body = await req.text();
    const ws = await rustApiForUser<MyWorkspace>(user, '/v1/team/my-workspaces', {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body,
    });
    return NextResponse.json(ws, { status: 201 });
  } catch (err) {
    const message = err instanceof Error ? err.message : 'unknown error';
    return NextResponse.json({ error: message }, { status: 502 });
  }
}
