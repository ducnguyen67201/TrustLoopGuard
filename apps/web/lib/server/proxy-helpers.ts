import 'server-only';
import { NextResponse } from 'next/server';

import {
  RustApiError,
  WorkspaceAccessError,
  rustApiForAuthorizedWorkspace,
} from '@/lib/server/tl-client';

type JsonObject = { [key: string]: JsonValue };
type JsonValue = string | number | boolean | null | JsonObject | JsonValue[];

function handleRustError(err: unknown): Response {
  if (err instanceof WorkspaceAccessError) {
    return NextResponse.json({ error: err.message }, { status: err.status });
  }
  if (err instanceof RustApiError) {
    const status = err.status >= 500 ? 502 : err.status;
    try {
      return NextResponse.json(JSON.parse(err.body), { status });
    } catch {
      return NextResponse.json({ error: 'upstream error' }, { status });
    }
  }
  return NextResponse.json({ error: 'upstream error' }, { status: 502 });
}

export async function proxyRustCollection(
  req: Request,
  rustPath: string,
  method: 'GET' | 'POST',
): Promise<Response> {
  try {
    const init: RequestInit = { method };
    if (method !== 'GET') {
      init.headers = { 'Content-Type': 'application/json' };
      init.body = await req.text();
    }
    const data = await rustApiForAuthorizedWorkspace<JsonValue>(req, rustPath, init);
    return NextResponse.json(data, { status: method === 'POST' ? 201 : 200 });
  } catch (err) {
    return handleRustError(err);
  }
}

export async function patchRustResource(
  req: Request,
  params: Promise<{ id: string }>,
  rustPathPrefix: string,
): Promise<Response> {
  try {
    const { id } = await params;
    const data = await rustApiForAuthorizedWorkspace<JsonValue>(
      req,
      `${rustPathPrefix}/${encodeURIComponent(id)}`,
      {
        method: 'PATCH',
        headers: { 'Content-Type': 'application/json' },
        body: await req.text(),
      },
    );
    return NextResponse.json(data);
  } catch (err) {
    return handleRustError(err);
  }
}

export async function deleteRustResource(
  req: Request,
  params: Promise<{ id: string }>,
  rustPathPrefix: string,
): Promise<Response> {
  try {
    const { id } = await params;
    await rustApiForAuthorizedWorkspace<JsonValue>(
      req,
      `${rustPathPrefix}/${encodeURIComponent(id)}`,
      { method: 'DELETE' },
    );
    return new Response(null, { status: 204 });
  } catch (err) {
    return handleRustError(err);
  }
}

export async function proxyRustResourceAction(
  req: Request,
  params: Promise<{ id: string }>,
  rustPathPrefix: string,
  action: string,
): Promise<Response> {
  try {
    const { id } = await params;
    const data = await rustApiForAuthorizedWorkspace<JsonValue>(
      req,
      `${rustPathPrefix}/${encodeURIComponent(id)}/${action}`,
      { method: 'POST' },
    );
    return NextResponse.json(data);
  } catch (err) {
    return handleRustError(err);
  }
}

export async function putRustResource(
  req: Request,
  params: Promise<{ id: string }>,
  rustPathPrefix: string,
  suffix?: string,
): Promise<Response> {
  try {
    const { id } = await params;
    const path = `${rustPathPrefix}/${encodeURIComponent(id)}${suffix ? `/${suffix}` : ''}`;
    const data = await rustApiForAuthorizedWorkspace<JsonValue>(req, path, {
      method: 'PUT',
      headers: { 'Content-Type': 'application/json' },
      body: await req.text(),
    });
    return NextResponse.json(data);
  } catch (err) {
    return handleRustError(err);
  }
}
