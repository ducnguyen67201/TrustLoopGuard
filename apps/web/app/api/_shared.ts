import { NextResponse } from 'next/server';
import { Decode, SdkError, Transport, type ApiErrorCode } from '@featherlane-ai/sdk';

import {
  RustApiError,
  rustApiResponseForAuthorizedWorkspace,
  rustApiForAuthorizedWorkspace,
  WorkspaceAccessError,
} from '@/lib/server/tl-client';

const SDK_CODE_TO_STATUS: Record<ApiErrorCode, number> = {
  invalid: 400,
  unauthorized: 401,
  forbidden: 403,
  not_found: 404,
  conflict: 409,
  gone: 410,
  unprocessable: 422,
  rate_limited: 429,
  internal: 500,
  unavailable: 503,
};

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
  if (err instanceof SdkError) {
    return sdkErrorResponse(err);
  }
  // Never surface raw internal error text (network/runtime details) to clients.
  return NextResponse.json({ error: 'upstream request failed' }, { status: 502 });
}

// SDK-client faults: the agent/redteam routes call the typed SDK, not the raw
// Rust proxy. Preserve the upstream status so user-fixable states (404/422/503)
// aren't masked as a gateway error, but never forward transport/decode text —
// it carries internal host/parse detail. The structured `error.message` from a
// real upstream `ApiError` is the public, client-safe envelope and is forwarded.
function sdkErrorResponse(err: SdkError) {
  if (err instanceof Transport || err instanceof Decode) {
    return NextResponse.json({ error: 'upstream request failed' }, { status: 502 });
  }
  const status = SDK_CODE_TO_STATUS[err.code] ?? 502;
  return NextResponse.json({ error: err.error.message }, { status });
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
