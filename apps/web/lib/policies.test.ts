import { afterEach, describe, expect, it, vi } from 'vitest';

import { generatePolicyDraft, validatePolicy } from './policies';

describe('generatePolicyDraft', () => {
  afterEach(() => {
    vi.unstubAllGlobals();
    window.history.replaceState({}, '', '/');
  });

  it('preserves the selected workspace on the policy draft request', async () => {
    window.history.replaceState({}, '', '/policies?workspace=featherlane-ai-demo');
    const fetchMock = vi.fn<typeof fetch>(async () => {
      return new Response(
        JSON.stringify({
          draft: {
            id: 'no-passwords',
            description: 'Block password sharing',
            matchType: 'regex',
            matchValue: 'password',
            action: 'deny',
            severity: 'high',
          },
        }),
        { status: 200, headers: { 'content-type': 'application/json' } },
      );
    });
    vi.stubGlobal('fetch', fetchMock);

    await generatePolicyDraft('block password sharing');

    expect(fetchMock).toHaveBeenCalledWith(
      '/api/policies/generate?workspace=featherlane-ai-demo',
      expect.objectContaining({ method: 'POST' }),
    );
  });

  it('preserves the selected workspace on the policy validation request', async () => {
    window.history.replaceState({}, '', '/policies?workspace=featherlane-ai-demo');
    const fetchMock = vi.fn<typeof fetch>(async () => {
      return new Response(
        JSON.stringify({
          valid: true,
          errors: [],
        }),
        { status: 200, headers: { 'content-type': 'application/json' } },
      );
    });
    vi.stubGlobal('fetch', fetchMock);

    await validatePolicy('id: no-passwords');

    expect(fetchMock).toHaveBeenCalledWith(
      '/api/policies/validate?workspace=featherlane-ai-demo',
      expect.objectContaining({ method: 'POST' }),
    );
  });
});
