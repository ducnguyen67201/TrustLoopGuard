import { NextRequest, NextResponse } from 'next/server';
import {
  DOCS_AUTH_COOKIE,
  DOCS_UNLOCK_PATH,
  createDocsAuthToken,
} from './lib/docs-auth';

const UNPROTECTED_PATHS = [DOCS_UNLOCK_PATH, '/api/docs-auth'];

export async function proxy(request: NextRequest) {
  const password = process.env['DOCS_PASSWORD'];

  if (!password) {
    return NextResponse.next();
  }

  if (UNPROTECTED_PATHS.some((path) => request.nextUrl.pathname.startsWith(path))) {
    return NextResponse.next();
  }

  const authCookie = request.cookies.get(DOCS_AUTH_COOKIE)?.value;
  const expectedCookie = await createDocsAuthToken(password);

  if (authCookie === expectedCookie) {
    return NextResponse.next();
  }

  const unlockUrl = request.nextUrl.clone();
  unlockUrl.pathname = DOCS_UNLOCK_PATH;
  unlockUrl.search = '';
  unlockUrl.searchParams.set('next', `${request.nextUrl.pathname}${request.nextUrl.search}`);

  return NextResponse.redirect(unlockUrl);
}

export const config = {
  matcher: ['/((?!_next/static|_next/image|favicon.ico).*)'],
};
