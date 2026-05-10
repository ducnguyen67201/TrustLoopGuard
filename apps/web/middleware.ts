import { NextResponse } from 'next/server';

import { auth } from '@/auth';

export default auth((req) => {
  const { nextUrl } = req;
  const isAuthed = !!req.auth;
  const isOnDashboard = nextUrl.pathname.startsWith('/dashboard');

  if (isOnDashboard && !isAuthed) {
    const signInUrl = new URL('/signin', nextUrl);
    return NextResponse.redirect(signInUrl);
  }
  return NextResponse.next();
});

export const config = {
  matcher: ['/dashboard/:path*'],
};
