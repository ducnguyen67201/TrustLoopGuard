import { NextRequest, NextResponse } from 'next/server';

const COUNTRY_HEADERS = [
  'x-vercel-ip-country',
  'cf-ipcountry',
  'cloudfront-viewer-country',
] as const;

export function proxy(request: NextRequest) {
  if (!isVietnameseCountry(request)) return NextResponse.next();

  const destination = request.nextUrl.clone();
  destination.pathname = '/vi';

  const response = NextResponse.redirect(destination);
  response.headers.set('Cache-Control', 'private, no-store');
  return response;
}

function isVietnameseCountry(request: NextRequest): boolean {
  return COUNTRY_HEADERS.some(
    (header) => request.headers.get(header)?.trim().toUpperCase() === 'VN',
  );
}

export const config = {
  matcher: ['/'],
};
