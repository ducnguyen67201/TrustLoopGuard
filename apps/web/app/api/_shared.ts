import { NextResponse } from 'next/server';

import {
  RustApiError,
  rustApiResponseForAuthorizedWorkspace,
  rustApiForAuthorizedWorkspace,
  WorkspaceAccessError,
} from '@/lib/server/tl-client';

export function forwardedQuery(searchParams: URLSearchParams): string {
  const next = new URLSearchParams(searchParams);
  next.delete('workspace');
  next.delete('environment');
  const serialized = next.toString();
  return serialized === '' ? '' : `?${serialized}`;
}

export async function proxyRustJson(req: Request, path: string, init?: RequestInit) {
  try {
    const { data, status } = await rustApiResponseForAuthorizedWorkspace<unknown>(
      req,
      path,
      init,
    );
    if (status === 204) {
      return new NextResponse(null, { status });
    }
    return NextResponse.json(data, { status });
  } catch (err) {
    return errorResponse(err);
  }
}

export async function proxyRustNoContent(req: Request, path: string) {
  try {
    await rustApiForAuthorizedWorkspace<unknown>(req, path, { method: 'DELETE' });
    return new NextResponse(null, { status: 204 });
  } catch (err) {
    return errorResponse(err);
  }
}

export function errorResponse(err: unknown) {
  if (err instanceof WorkspaceAccessError) {
    return NextResponse.json({ error: err.message }, { status: err.status });
  }
  if (err instanceof RustApiError) {
    return upstreamErrorResponse(err);
  }
  const message = err instanceof Error ? err.message : 'unknown error';
  return NextResponse.json({ error: message }, { status: 502 });
}

function upstreamErrorResponse(err: RustApiError) {
  if (err.body.trim() !== '') {
    try {
      const body: unknown = JSON.parse(err.body);
      return NextResponse.json(body, { status: err.status });
    } catch {
      return new NextResponse(err.body, { status: err.status });
    }
  }
  return NextResponse.json({ error: err.message }, { status: err.status });
}
