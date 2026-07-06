import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { NextResponse } from 'next/server';

vi.mock('@/app/api/_shared', () => ({
  proxyRustJson: vi.fn(),
}));

import { proxyRustJson } from '@/app/api/_shared';
import { GET, POST } from './route';

const proxyMock = vi.mocked(proxyRustJson);

describe('/api/financial/actions', () => {
  beforeEach(() => {
    proxyMock.mockReset();
  });

  afterEach(() => {
    vi.clearAllMocks();
  });

  it('proxies action listing to the Rust financial API', async () => {
    const response = NextResponse.json({ actions: [] });
    proxyMock.mockResolvedValue(response);
    const req = new Request('https://app.test/api/financial/actions');

    const res = await GET(req);

    expect(res).toBe(response);
    expect(proxyMock).toHaveBeenCalledWith(req, '/v1/financial/actions');
  });

  it('proxies action creation to the Rust financial API', async () => {
    const response = NextResponse.json({ id: 'act_1' }, { status: 201 });
    proxyMock.mockResolvedValue(response);
    const req = new Request('https://app.test/api/financial/actions', {
      method: 'POST',
      body: JSON.stringify({ idempotency_key: 'idem_1' }),
    });

    const res = await POST(req);

    expect(res).toBe(response);
    expect(proxyMock).toHaveBeenCalledWith(req, '/v1/financial/actions', {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({ idempotency_key: 'idem_1' }),
    });
  });
});
