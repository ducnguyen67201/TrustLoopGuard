import { beforeEach, describe, expect, it, vi } from 'vitest';
import { NextResponse } from 'next/server';

vi.mock('@/app/api/_shared', () => ({
  proxyRustJson: vi.fn<typeof import('@/app/api/_shared').proxyRustJson>(),
}));

import { proxyRustJson } from '@/app/api/_shared';
import { GET } from './route';

const proxyMock = vi.mocked(proxyRustJson);

describe('/api/financial/actions/[id]/approval-envelope', () => {
  beforeEach(() => proxyMock.mockReset());

  it('proxies the encoded action id to the Rust API', async () => {
    const response = NextResponse.json({ action_fingerprint: 'sha256:v1:test' });
    proxyMock.mockResolvedValue(response);
    const req = new Request(
      'https://app.test/api/financial/actions/action%2Fone/approval-envelope',
    );

    const result = await GET(req, { params: Promise.resolve({ id: 'action/one' }) });

    expect(result).toBe(response);
    expect(proxyMock).toHaveBeenCalledWith(
      req,
      '/v1/financial/actions/action%2Fone/approval-envelope',
    );
  });
});
