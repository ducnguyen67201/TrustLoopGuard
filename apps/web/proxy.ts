import { NextResponse, type NextRequest } from 'next/server';
import { getToken } from 'next-auth/jwt';

const PUBLIC_PATH_PREFIXES = [
  '/signin',
  '/signout',
  '/signup',
  '/docs',
  '/arena',
  '/api/auth',
  '/api/signup',
  '/_next',
  '/favicon.ico',
];
const SESSION_COOKIE_NAMES = ['authjs.session-token', '__Secure-authjs.session-token'];

export async function proxy(req: NextRequest) {
  const { pathname } = req.nextUrl;
  const hasValidSession = await isAuthenticated(req);

  if ((pathname === '/signin' || pathname === '/signup') && hasValidSession) {
    const callbackUrl = safeRedirect(req.nextUrl.searchParams.get('callbackUrl'));
    return NextResponse.redirect(new URL(callbackUrl, req.url));
  }

  if (isPublicPath(pathname) || hasValidSession) {
    return NextResponse.next();
  }

  if (pathname.startsWith('/api/')) {
    return NextResponse.json({ error: 'authentication required' }, { status: 401 });
  }

  const signInUrl = req.nextUrl.clone();
  signInUrl.pathname = '/signin';
  signInUrl.searchParams.set('callbackUrl', `${req.nextUrl.pathname}${req.nextUrl.search}`);
  return NextResponse.redirect(signInUrl);
}

export const config = {
  matcher: ['/((?!.*\\..*).*)'],
};

function isPublicPath(pathname: string): boolean {
  return PUBLIC_PATH_PREFIXES.some((prefix) => pathname === prefix || pathname.startsWith(`${prefix}/`));
}

async function isAuthenticated(req: NextRequest): Promise<boolean> {
  const secret = process.env['AUTH_SECRET'] ?? process.env['NEXTAUTH_SECRET'];
  if (!secret) {
    return SESSION_COOKIE_NAMES.some((name) => req.cookies.has(name));
  }

  for (const cookieName of SESSION_COOKIE_NAMES) {
    const token = await getToken({ req, secret, cookieName, salt: cookieName });
    if (token !== null) return true;
  }
  return false;
}

function safeRedirect(value: string | null): string {
  if (typeof value !== 'string' || value.trim() === '') return '/';
  if (!value.startsWith('/') || value.startsWith('//')) return '/';
  if (value.startsWith('/signin') || value.startsWith('/api/auth')) return '/';
  return value;
}
