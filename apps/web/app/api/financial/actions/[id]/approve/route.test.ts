import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { NextResponse } from 'next/server';

vi.mock('@/app/api/_shared', () => ({
  proxyRustJson: vi.fn(),
}));

import { proxyRustJson } from '@/app/api/_shared';
import { POST } from './route';

const proxyMock = vi.mocked(proxyRustJson);

function context(id = 'action/one') {
  return { params: Promise.resolve({ id }) };
}

describe('POST /api/financial/actions/[id]/approve', () => {
  beforeEach(() => {
    proxyMock.mockReset();
  });

  afterEach(() => {
    vi.clearAllMocks();
  });

  it('proxies approval to the encoded Rust financial action endpoint', async () => {
    const response = NextResponse.json({ id: 'action/one', status: 'authorized' });
    proxyMock.mockResolvedValue(response);
    const req = new Request('https://app.test/api/financial/actions/action%2Fone/approve', {
      method: 'POST',
    });

    const res = await POST(req, context());

    expect(res).toBe(response);
    expect(proxyMock).toHaveBeenCalledWith(
      req,
      '/v1/financial/actions/action%2Fone/approve',
      { method: 'POST' },
    );
  });
});
