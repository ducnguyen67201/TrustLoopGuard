import { NextResponse } from 'next/server';
import { DOCS_AUTH_COOKIE, DOCS_UNLOCK_PATH } from '@/lib/docs-auth';

function redirectTo(path: string): NextResponse {
  return new NextResponse(null, {
    status: 303,
    headers: {
      Location: path,
    },
  });
}

export function POST() {
  const response = redirectTo(DOCS_UNLOCK_PATH);
  response.cookies.set({
    name: DOCS_AUTH_COOKIE,
    value: '',
    httpOnly: true,
    sameSite: 'lax',
    secure: process.env['NODE_ENV'] === 'production',
    maxAge: 0,
    path: '/',
  });

  return response;
}
