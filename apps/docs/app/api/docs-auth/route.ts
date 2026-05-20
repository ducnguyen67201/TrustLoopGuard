import { NextRequest, NextResponse } from 'next/server';
import {
  DOCS_AUTH_COOKIE,
  DOCS_UNLOCK_PATH,
  createDocsAuthToken,
  safeDocsRedirectPath,
} from '@/lib/docs-auth';

const COOKIE_MAX_AGE_SECONDS = 60 * 60 * 12;

export async function POST(request: NextRequest) {
  const configuredPassword = process.env['DOCS_PASSWORD'];
  const formData = await request.formData();
  const nextPath = safeDocsRedirectPath(formData.get('next'));

  if (!configuredPassword) {
    return NextResponse.redirect(new URL(nextPath, request.url), { status: 303 });
  }

  if (formData.get('password') !== configuredPassword) {
    const unlockUrl = new URL(DOCS_UNLOCK_PATH, request.url);
    unlockUrl.searchParams.set('error', '1');
    unlockUrl.searchParams.set('next', nextPath);

    return NextResponse.redirect(unlockUrl, { status: 303 });
  }

  const response = NextResponse.redirect(new URL(nextPath, request.url), { status: 303 });
  response.cookies.set({
    name: DOCS_AUTH_COOKIE,
    value: await createDocsAuthToken(configuredPassword),
    httpOnly: true,
    sameSite: 'lax',
    secure: process.env['NODE_ENV'] === 'production',
    maxAge: COOKIE_MAX_AGE_SECONDS,
    path: '/',
  });

  return response;
}
