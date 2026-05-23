import { handlers } from '@/auth';
import { authSignOutRedirectUrl, isAuthSignOutGet } from '@/lib/auth-signout-route';
import { NextResponse, type NextRequest } from 'next/server';

export function GET(request: NextRequest) {
  if (isAuthSignOutGet(request.nextUrl.pathname)) {
    return NextResponse.redirect(authSignOutRedirectUrl(request.url));
  }

  return handlers.GET(request);
}

export const { POST } = handlers;
