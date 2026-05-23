import { NextRequest, NextResponse } from 'next/server';
import {
  DOCS_AUTH_COOKIE,
  DOCS_UNLOCK_PATH,
  createDocsAuthToken,
  safeDocsRedirectPath,
} from '@/lib/docs-auth';

const COOKIE_MAX_AGE_SECONDS = 60 * 60 * 12;

function redirectTo(path: string): NextResponse {
  return new NextResponse(null, {
    status: 303,
    headers: {
      Location: path,
    },
  });
}

export async function POST(request: NextRequest) {
  const configuredPassword = process.env['DOCS_PASSWORD'];
  const formData = await request.formData();
  const nextPath = safeDocsRedirectPath(formData.get('next'));

  if (!configuredPassword) {
    return redirectTo(nextPath);
  }

  if (formData.get('password') !== configuredPassword) {
    const unlockParams = new URLSearchParams({
      error: '1',
      next: nextPath,
    });

    return redirectTo(`${DOCS_UNLOCK_PATH}?${unlockParams.toString()}`);
  }

  const response = redirectTo(nextPath);
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
