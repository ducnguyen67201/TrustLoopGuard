import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { NextResponse } from 'next/server';

vi.mock('@/app/api/_shared', () => ({
  proxyRustJson: vi.fn(),
}));

import { proxyRustJson } from '@/app/api/_shared';
import { GET } from './route';

const proxyMock = vi.mocked(proxyRustJson);

function context(id = 'receipt/one') {
  return { params: Promise.resolve({ id }) };
}

describe('GET /api/financial/receipts/[id]', () => {
  beforeEach(() => {
    proxyMock.mockReset();
  });

  afterEach(() => {
    vi.clearAllMocks();
  });

  it('proxies receipt reads to the Rust financial API with encoded ids', async () => {
    const response = NextResponse.json({ id: 'receipt/one' });
    proxyMock.mockResolvedValue(response);
    const req = new Request('https://app.test/api/financial/receipts/receipt%2Fone');

    const res = await GET(req, context());

    expect(res).toBe(response);
    expect(proxyMock).toHaveBeenCalledWith(
      req,
      '/v1/financial/receipts/receipt%2Fone',
    );
  });
});
