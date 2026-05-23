import { beforeEach, describe, expect, it, vi } from 'vitest';

interface ValidationResponse {
  valid: boolean;
  errors: Array<{ path: string; message: string }>;
  policy_id?: string;
}

interface ValidationClient {
  validatePolicy: (yaml: string) => Promise<ValidationResponse>;
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
    tlClientForRequest: vi.fn<(req: Request) => Promise<ValidationClient>>(),
    validatePolicy: vi.fn<(yaml: string) => Promise<ValidationResponse>>(),
    WorkspaceAccessError: MockWorkspaceAccessError,
  };
});

vi.mock('@/lib/server/tl-client', () => ({
  tlClientForRequest: mockState.tlClientForRequest,
  WorkspaceAccessError: mockState.WorkspaceAccessError,
}));

import { POST } from './route';

describe('/api/policies/validate', () => {
  beforeEach(() => {
    mockState.tlClientForRequest.mockReset();
    mockState.validatePolicy.mockReset();
  });

  it('uses the request-scoped Rust client when validating a policy', async () => {
    const req = new Request('https://app.test/api/policies/validate?workspace=demo', {
      method: 'POST',
      body: 'id: no-passwords',
    });
    mockState.validatePolicy.mockResolvedValue({ valid: true, errors: [] });
    mockState.tlClientForRequest.mockResolvedValue({ validatePolicy: mockState.validatePolicy });

    const res = await POST(req);

    expect(mockState.tlClientForRequest).toHaveBeenCalledWith(req);
    expect(mockState.validatePolicy).toHaveBeenCalledWith('id: no-passwords');
    await expect(res.json()).resolves.toEqual({ valid: true, errors: [] });
  });

  it('returns workspace access failures without masking them as bad gateway', async () => {
    mockState.tlClientForRequest.mockRejectedValue(
      new mockState.WorkspaceAccessError(401, 'authentication required'),
    );

    const res = await POST(
      new Request('https://app.test/api/policies/validate', {
        method: 'POST',
        body: 'id: no-passwords',
      }),
    );

    expect(res.status).toBe(401);
    await expect(res.json()).resolves.toEqual({ error: 'authentication required' });
  });
});
