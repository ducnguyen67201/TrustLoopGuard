import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { NextResponse } from 'next/server';

vi.mock('@/app/api/_shared', () => ({
  proxyRustJson: vi.fn(),
}));

import { proxyRustJson } from '@/app/api/_shared';
import { GET, POST } from './route';

const proxyMock = vi.mocked(proxyRustJson);

describe('/api/financial/policies', () => {
  beforeEach(() => {
    proxyMock.mockReset();
  });

  afterEach(() => {
    vi.clearAllMocks();
  });

  it('proxies financial policy listing to the Rust API', async () => {
    const response = NextResponse.json({ policies: [] });
    proxyMock.mockResolvedValue(response);
    const req = new Request('https://app.test/api/financial/policies');

    const res = await GET(req);

    expect(res).toBe(response);
    expect(proxyMock).toHaveBeenCalledWith(req, '/v1/financial/policies');
  });

  it('proxies financial policy creation to the Rust API', async () => {
    const response = NextResponse.json({ id: 'refund-controls' }, { status: 201 });
    proxyMock.mockResolvedValue(response);
    const body = JSON.stringify({ id: 'refund-controls' });
    const req = new Request('https://app.test/api/financial/policies', {
      method: 'POST',
      body,
    });

    const res = await POST(req);

    expect(res).toBe(response);
    expect(proxyMock).toHaveBeenCalledWith(req, '/v1/financial/policies', {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body,
    });
  });
});
