import { NextRequest, NextResponse } from 'next/server';

const REALM = 'TrustLoopGuard docs';

export function proxy(request: NextRequest) {
  const password = process.env['DOCS_PASSWORD'];

  if (!password) {
    return NextResponse.next();
  }

  const authorization = request.headers.get('authorization');
  if (authorization?.startsWith('Basic ')) {
    const encoded = authorization.slice('Basic '.length);
    const decoded = atob(encoded);
    const separator = decoded.indexOf(':');
    const suppliedPassword = separator >= 0 ? decoded.slice(separator + 1) : '';

    if (suppliedPassword === password) {
      return NextResponse.next();
    }
  }

  return new NextResponse('Authentication required', {
    status: 401,
    headers: {
      'WWW-Authenticate': `Basic realm="${REALM}", charset="UTF-8"`,
    },
  });
}

export const config = {
  matcher: ['/((?!_next/static|_next/image|favicon.ico).*)'],
};
