import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { NextResponse } from 'next/server';

vi.mock('@/app/api/_shared', () => ({
  proxyRustJson: vi.fn(),
}));

import { proxyRustJson } from '@/app/api/_shared';
import { GET as listApprovals } from './approvals/route';
import { GET as getApproval } from './approvals/[id]/route';
import { POST as decideApproval } from './approvals/[id]/decide/route';
import { GET as listGrants, POST as createGrant } from './grants/route';
import { POST as revokeGrant } from './grants/[id]/revoke/route';
import { GET as listReceipts } from './receipts/route';
import { GET as getReceipt } from './receipts/[id]/route';

const proxyMock = vi.mocked(proxyRustJson);
const response = NextResponse.json({ ok: true });
const context = (id: string) => ({ params: Promise.resolve({ id }) });

describe('unified authorization web proxy', () => {
  beforeEach(() => {
    proxyMock.mockReset();
    proxyMock.mockResolvedValue(response);
  });

  afterEach(() => {
    vi.clearAllMocks();
  });

  it('proxies the single approval queue and scoped approval reads', async () => {
    const listRequest = new Request('https://app.test/api/authorization/approvals');
    const getRequest = new Request('https://app.test/api/authorization/approvals/a%2Fb');

    expect(await listApprovals(listRequest)).toBe(response);
    expect(await getApproval(getRequest, context('a/b'))).toBe(response);

    expect(proxyMock).toHaveBeenNthCalledWith(1, listRequest, '/v1/authorization/approvals');
    expect(proxyMock).toHaveBeenNthCalledWith(2, getRequest, '/v1/authorization/approvals/a%2Fb');
  });

  it('forwards the exact signed decision body to Rust', async () => {
    const body = JSON.stringify({
      decision: 'approve',
      mode: 'exact_once',
      envelope_hash: 'sha256:v1:reviewed',
    });
    const request = new Request('https://app.test/api/authorization/approvals/id/decide', {
      method: 'POST',
      body,
    });

    expect(await decideApproval(request, context('id'))).toBe(response);
    expect(proxyMock).toHaveBeenCalledWith(request, '/v1/authorization/approvals/id/decide', {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body,
    });
  });

  it('proxies grant creation, listing, and revocation through one API', async () => {
    const listRequest = new Request('https://app.test/api/authorization/grants');
    const body = JSON.stringify({ principal_id: 'agent-1' });
    const createRequest = new Request('https://app.test/api/authorization/grants', {
      method: 'POST',
      body,
    });
    const revokeRequest = new Request(
      'https://app.test/api/authorization/grants/grant%2Fone/revoke',
      { method: 'POST' },
    );

    await listGrants(listRequest);
    await createGrant(createRequest);
    await revokeGrant(revokeRequest, context('grant/one'));

    expect(proxyMock).toHaveBeenNthCalledWith(1, listRequest, '/v1/authorization/grants');
    expect(proxyMock).toHaveBeenNthCalledWith(2, createRequest, '/v1/authorization/grants', {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body,
    });
    expect(proxyMock).toHaveBeenNthCalledWith(
      3,
      revokeRequest,
      '/v1/authorization/grants/grant%2Fone/revoke',
      { method: 'POST' },
    );
  });

  it('proxies authorization receipt reads with encoded ids', async () => {
    const listRequest = new Request('https://app.test/api/authorization/receipts');
    const request = new Request('https://app.test/api/authorization/receipts/r%2F1');

    expect(await listReceipts(listRequest)).toBe(response);
    expect(await getReceipt(request, context('r/1'))).toBe(response);
    expect(proxyMock).toHaveBeenNthCalledWith(1, listRequest, '/v1/authorization/receipts');
    expect(proxyMock).toHaveBeenNthCalledWith(2, request, '/v1/authorization/receipts/r%2F1');
  });
});
