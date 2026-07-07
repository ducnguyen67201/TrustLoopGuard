import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { NextResponse } from 'next/server';

vi.mock('@/app/api/_shared', () => ({
  proxyRustJson: vi.fn(),
}));

import { proxyRustJson } from '@/app/api/_shared';
import { PUT } from './route';

const proxyMock = vi.mocked(proxyRustJson);

function context(model: string[] = ['my-deploy', 'deepseek-v4-flash']) {
  return { params: Promise.resolve({ model }) };
}

describe('PUT /api/llm-pricing/[...model]', () => {
  beforeEach(() => {
    proxyMock.mockReset();
  });

  afterEach(() => {
    vi.clearAllMocks();
  });

  it('proxies model price updates to the encoded Rust pricing endpoint', async () => {
    const response = NextResponse.json({ model: 'my-deploy/deepseek-v4-flash' });
    proxyMock.mockResolvedValue(response);
    const body = JSON.stringify({
      input_per_million_minor: 27,
      output_per_million_minor: 110,
    });
    const req = new Request(
      'https://app.test/api/llm-pricing/my-deploy/deepseek-v4-flash?workspace=demo',
      { method: 'PUT', body },
    );

    const res = await PUT(req, context());

    expect(res).toBe(response);
    expect(proxyMock).toHaveBeenCalledWith(req, '/v1/llm-pricing/my-deploy%2Fdeepseek-v4-flash', {
      method: 'PUT',
      headers: { 'Content-Type': 'application/json' },
      body,
    });
  });
});
