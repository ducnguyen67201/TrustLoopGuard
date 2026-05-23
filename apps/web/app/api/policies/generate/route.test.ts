import { beforeEach, describe, expect, it, vi } from 'vitest';

interface DraftResponse {
  draft: {
    id: string;
    description: string;
    match_type: string;
    match_value: string;
    action: string;
    severity: string;
    rewrite?: string | null;
  };
}

interface DraftingClient {
  draftPolicy: (prompt: string) => Promise<DraftResponse>;
}

const mockState = vi.hoisted(() => {
  class MockWorkspaceAccessError extends Error {
    readonly status: 401 | 403;

    constructor(status: 401 | 403, message: string) {
      super(message);
      this.status = status;
    }
  }

  return {
    draftPolicy: vi.fn<(prompt: string) => Promise<DraftResponse>>(),
    tlClientForRequest: vi.fn<(req: Request) => Promise<DraftingClient>>(),
    WorkspaceAccessError: MockWorkspaceAccessError,
  };
});

vi.mock('@/lib/server/tl-client', () => ({
  tlClientForRequest: mockState.tlClientForRequest,
  WorkspaceAccessError: mockState.WorkspaceAccessError,
}));

import { POST } from './route';

describe('/api/policies/generate', () => {
  beforeEach(() => {
    mockState.draftPolicy.mockReset();
    mockState.tlClientForRequest.mockReset();
  });

  it('uses the request-scoped Rust client when drafting a policy', async () => {
    const req = new Request('https://app.test/api/policies/generate?workspace=demo', {
      method: 'POST',
      body: JSON.stringify({ prompt: 'block customers from sharing passwords' }),
    });
    mockState.draftPolicy.mockResolvedValue({
      draft: {
        id: 'no-passwords',
        description: 'Block password sharing',
        match_type: 'regex',
        match_value: 'password',
        action: 'block',
        severity: 'high',
      },
    });
    mockState.tlClientForRequest.mockResolvedValue({ draftPolicy: mockState.draftPolicy });

    const res = await POST(req);

    expect(mockState.tlClientForRequest).toHaveBeenCalledWith(req);
    expect(mockState.draftPolicy).toHaveBeenCalledWith('block customers from sharing passwords');
    await expect(res.json()).resolves.toEqual({
      draft: {
        id: 'no-passwords',
        description: 'Block password sharing',
        matchType: 'regex',
        matchValue: 'password',
        action: 'block',
        severity: 'high',
      },
    });
  });

  it('returns workspace access failures without masking them as bad gateway', async () => {
    mockState.tlClientForRequest.mockRejectedValue(
      new mockState.WorkspaceAccessError(401, 'authentication required'),
    );

    const res = await POST(
      new Request('https://app.test/api/policies/generate', {
        method: 'POST',
        body: JSON.stringify({ prompt: 'block password sharing' }),
      }),
    );

    expect(res.status).toBe(401);
    await expect(res.json()).resolves.toEqual({ error: 'authentication required' });
  });
});
