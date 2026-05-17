import { NextResponse } from 'next/server';

import { RustApiError, rustApiForWorkspace, workspaceIdFromSlug } from '@/lib/server/tl-client';

export const runtime = 'nodejs';

export async function GET(req: Request) {
  try {
    const url = new URL(req.url);
    const workspaceId = workspaceIdFromSlug(url.searchParams.get('workspace'));
    const rustQuery = forwardedQuery(url.searchParams);
    const data = await rustApiForWorkspace<unknown>(workspaceId, `/v1/runs${rustQuery}`);
    return NextResponse.json(data);
  } catch (err) {
    return errorResponse(err);
  }
}

export async function POST(req: Request) {
  try {
    const url = new URL(req.url);
    const workspaceId = workspaceIdFromSlug(url.searchParams.get('workspace'));
    const body = await req.text();
    const data = await rustApiForWorkspace<unknown>(workspaceId, '/v1/runs', {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body,
    });
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
