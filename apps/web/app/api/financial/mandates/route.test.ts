import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { NextResponse } from 'next/server';

vi.mock('@/app/api/_shared', () => ({
  proxyRustJson: vi.fn<typeof import('@/app/api/_shared').proxyRustJson>(),
}));

import { proxyRustJson } from '@/app/api/_shared';
import { GET, POST } from './route';

const proxyMock = vi.mocked(proxyRustJson);

describe('/api/financial/mandates', () => {
  beforeEach(() => {
    proxyMock.mockReset();
  });

  afterEach(() => {
    vi.clearAllMocks();
  });

  it('proxies mandate listing to the Rust financial API', async () => {
    const response = NextResponse.json({ mandates: [] });
    proxyMock.mockResolvedValue(response);
    const req = new Request('https://app.test/api/financial/mandates');

    const res = await GET(req);

    expect(res).toBe(response);
    expect(proxyMock).toHaveBeenCalledWith(req, '/v1/financial/mandates');
  });

  it('proxies mandate creation to the Rust financial API', async () => {
    const response = NextResponse.json({ id: 'mandate_1' }, { status: 201 });
    proxyMock.mockResolvedValue(response);
    const req = new Request('https://app.test/api/financial/mandates', {
      method: 'POST',
      body: JSON.stringify({ principal_id: 'refund-bot' }),
    });

    const res = await POST(req);

    expect(res).toBe(response);
    expect(proxyMock).toHaveBeenCalledWith(req, '/v1/financial/mandates', {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({ principal_id: 'refund-bot' }),
    });
  });
});
