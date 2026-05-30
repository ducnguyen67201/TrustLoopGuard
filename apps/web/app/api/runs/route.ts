import { NextResponse } from 'next/server';

import { errorResponse, forwardedQuery } from '../_shared';
import { rustApiForAuthorizedWorkspace } from '@/lib/server/tl-client';

export const runtime = 'nodejs';

export async function GET(req: Request) {
  try {
    const url = new URL(req.url);
    const rustQuery = forwardedQuery(url.searchParams);
    const data = await rustApiForAuthorizedWorkspace<unknown>(req, `/v1/runs${rustQuery}`);
    return NextResponse.json(data);
  } catch (err) {
    return errorResponse(err);
  }
}

export async function POST(req: Request) {
  try {
    const url = new URL(req.url);
    const body = await req.text();
    const data = await rustApiForAuthorizedWorkspace<unknown>(req, '/v1/runs', {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body,
    });
    return NextResponse.json(data, { status: 201 });
  } catch (err) {
    return errorResponse(err);
  }
}
