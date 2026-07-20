import { NextResponse } from 'next/server';

import { errorResponse } from '@/app/api/_shared';
import { auth } from '@/auth';
import { rustApiForUser } from '@/lib/server/tl-client';

export const runtime = 'nodejs';

interface RouteParams {
  params: Promise<{ id: string }>;
}

interface SessionUser {
  id?: string;
  email?: string | null;
  tlJwt?: string;
}

function userFromSession(
  sessionUser: SessionUser | undefined,
): { id: string; email?: string | null; tlJwt?: string } | null {
  if (sessionUser?.id === undefined || sessionUser.id === '') return null;
  const user: { id: string; email?: string | null; tlJwt?: string } = {
    id: sessionUser.id,
  };
  if (sessionUser.email !== undefined && sessionUser.email !== null) {
    user.email = sessionUser.email;
  }
  if (sessionUser.tlJwt !== undefined && sessionUser.tlJwt !== '') {
    user.tlJwt = sessionUser.tlJwt;
  }
  return user;
}

export async function DELETE(_request: Request, { params }: RouteParams) {
  try {
    const session = await auth();
    const user = userFromSession(session?.user as SessionUser | undefined);
    if (user === null) {
      return NextResponse.json({ error: 'Unauthorized' }, { status: 401 });
    }

    const { id } = await params;
    await rustApiForUser<void>(user, `/v1/team/my-workspaces/${encodeURIComponent(id)}`, {
      method: 'DELETE',
    });
    return new NextResponse(null, { status: 204 });
  } catch (error) {
    return errorResponse(error);
  }
}
