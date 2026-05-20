import { NextResponse } from 'next/server';

import {
  RustApiError,
  rustApiForAuthorizedWorkspace,
  WorkspaceAccessError,
} from '@/lib/server/tl-client';

export const runtime = 'nodejs';

type RouteContext = {
  params: Promise<{ id: string }>;
};

export async function GET(req: Request, context: RouteContext) {
  try {
    const { id } = await context.params;
    const url = new URL(req.url);
    const rustQuery = forwardedQuery(url.searchParams);
    const data = await rustApiForAuthorizedWorkspace<unknown>(
      req,
      `/v1/runs/${encodeURIComponent(id)}/events${rustQuery}`,
    );
    return NextResponse.json(data);
  } catch (err) {
    return errorResponse(err);
  }
}

export async function POST(req: Request, context: RouteContext) {
  try {
    const { id } = await context.params;
    const body = await req.text();
    const data = await rustApiForAuthorizedWorkspace<unknown>(
      req,
      `/v1/runs/${encodeURIComponent(id)}/events`,
      {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body,
      },
    );
    return NextResponse.json(data, { status: 201 });
  } catch (err) {
    return errorResponse(err);
  }
}

function forwardedQuery(searchParams: URLSearchParams): string {
  const next = new URLSearchParams(searchParams);
  next.delete('workspace');
  const serialized = next.toString();
  return serialized === '' ? '' : `?${serialized}`;
}

function errorResponse(err: unknown) {
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
