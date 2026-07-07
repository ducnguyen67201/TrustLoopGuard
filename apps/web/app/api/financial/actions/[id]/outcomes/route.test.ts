import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { NextResponse } from 'next/server';

vi.mock('@/app/api/_shared', () => ({
  proxyRustJson: vi.fn<typeof import('@/app/api/_shared').proxyRustJson>(),
}));

import { proxyRustJson } from '@/app/api/_shared';
import { GET, POST } from './route';

const proxyMock = vi.mocked(proxyRustJson);

type RouteContext = Parameters<typeof GET>[1];

function context(id = 'action/one'): RouteContext {
  return { params: Promise.resolve({ id }) };
}

describe('/api/financial/actions/[id]/outcomes', () => {
  beforeEach(() => {
    proxyMock.mockReset();
  });

  afterEach(() => {
    vi.clearAllMocks();
  });

  it('proxies outcome reads to the Rust financial API with encoded ids', async () => {
    const response = NextResponse.json({ outcomes: [] });
    proxyMock.mockResolvedValue(response);
    const req = new Request('https://app.test/api/financial/actions/action%2Fone/outcomes');

    const res = await GET(req, context());

    expect(res).toBe(response);
    expect(proxyMock).toHaveBeenCalledWith(req, '/v1/financial/actions/action%2Fone/outcomes');
  });

  it('proxies outcome writes to the Rust financial API with encoded ids', async () => {
    const response = NextResponse.json({ status: 'succeeded' }, { status: 201 });
    proxyMock.mockResolvedValue(response);
    const req = new Request('https://app.test/api/financial/actions/action%2Fone/outcomes', {
      method: 'POST',
      body: JSON.stringify({ status: 'succeeded' }),
    });

    const res = await POST(req, context());

    expect(res).toBe(response);
    expect(proxyMock).toHaveBeenCalledWith(req, '/v1/financial/actions/action%2Fone/outcomes');
  });
});
