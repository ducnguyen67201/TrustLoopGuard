import { NextResponse } from 'next/server';

import { auth } from '@/auth';

// Paths that render without (or *before*) a workspace context.
// Everything else falls through to `getDashboardShell`, which
// redirects to `/welcome` when the user has no memberships.
const PUBLIC_PREFIXES = [
  '/signin',
  '/signup',
  '/welcome',
  '/invite/accept',
];

const PUBLIC_EXACT = new Set<string>(['/favicon.ico', '/robots.txt']);

export default auth((req) => {
  const { pathname } = req.nextUrl;

  if (PUBLIC_EXACT.has(pathname)) return;
  if (PUBLIC_PREFIXES.some((prefix) => pathname.startsWith(prefix))) return;

  if (!req.auth) {
    const url = req.nextUrl.clone();
    url.pathname = '/signin';
    url.searchParams.set('callbackUrl', pathname + req.nextUrl.search);
    return NextResponse.redirect(url);
  }

  // Authenticated → continue. The workspace-membership check happens
  // in `getDashboardShell`, which redirects to `/welcome` server-side
  // when memberships = 0. Doing it there (rather than here) keeps
  // middleware free of a per-request Rust round-trip.
  return;
});

// Skip Next internals, the NextAuth handler, and the same-origin proxy
// routes (they're called *by* middleware-protected pages, not the user).
export const config = {
  matcher: ['/((?!_next/static|_next/image|api/|.*\\.[^/]+$).*)'],
};
