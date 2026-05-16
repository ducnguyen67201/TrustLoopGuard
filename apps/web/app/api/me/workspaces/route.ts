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

export async function GET() {
  try {
    const session = await auth();
    const sessionUser = session?.user;
    if (sessionUser?.id === undefined || sessionUser.id === '') {
      return NextResponse.json({ workspaces: [] }, { status: 401 });
    }
    const data = await rustApiForUser<MyWorkspacesResponse>(
      { id: sessionUser.id, email: sessionUser.email },
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
    const sessionUser = session?.user;
    if (sessionUser?.id === undefined || sessionUser.id === '') {
      return NextResponse.json({ error: 'unauthenticated' }, { status: 401 });
    }
    const body = await req.text();
    const ws = await rustApiForUser<MyWorkspace>(
      { id: sessionUser.id, email: sessionUser.email },
      '/v1/team/my-workspaces',
      {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body,
      },
    );
    return NextResponse.json(ws, { status: 201 });
  } catch (err) {
    const message = err instanceof Error ? err.message : 'unknown error';
    return NextResponse.json({ error: message }, { status: 502 });
  }
}
