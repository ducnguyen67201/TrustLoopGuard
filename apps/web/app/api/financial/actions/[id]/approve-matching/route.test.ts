import { beforeEach, describe, expect, it, vi } from 'vitest';
import { NextResponse } from 'next/server';

vi.mock('@/app/api/_shared', () => ({
  proxyRustJson: vi.fn<typeof import('@/app/api/_shared').proxyRustJson>(),
}));

import { proxyRustJson } from '@/app/api/_shared';
import { POST } from './route';

const proxyMock = vi.mocked(proxyRustJson);

describe('/api/financial/actions/[id]/approve-matching', () => {
  beforeEach(() => proxyMock.mockReset());

  it('forwards the approval bounds to the Rust API', async () => {
    const response = NextResponse.json({ mandate: { id: 'mandate_1' } });
    proxyMock.mockResolvedValue(response);
    const body = JSON.stringify({
      action_fingerprint: 'sha256:v1:test',
      max_amount_minor: 10000,
      expires_at: '2026-07-14T12:00:00Z',
    });
    const req = new Request(
      'https://app.test/api/financial/actions/action%2Fone/approve-matching',
      {
        method: 'POST',
        body,
      },
    );

    const result = await POST(req, { params: Promise.resolve({ id: 'action/one' }) });

    expect(result).toBe(response);
    expect(proxyMock).toHaveBeenCalledWith(
      req,
      '/v1/financial/actions/action%2Fone/approve-matching',
      { method: 'POST', headers: { 'Content-Type': 'application/json' }, body },
    );
  });
});
