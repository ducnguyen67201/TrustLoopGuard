import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';

import { attackRunRequestSchema, pollAttackRun, startAttackRun } from './attacks';

const fetchMock = vi.fn<typeof fetch>();

describe('attackRunRequestSchema', () => {
  it('accepts a profile + target url', () => {
    expect(
      attackRunRequestSchema.safeParse({ profile: 'fast', targetUrl: 'http://127.0.0.1:9102' })
        .success,
    ).toBe(true);
  });

  it('rejects a missing or invalid target url', () => {
    expect(attackRunRequestSchema.safeParse({ profile: 'fast' }).success).toBe(false);
    expect(
      attackRunRequestSchema.safeParse({ profile: 'fast', targetUrl: 'not a url' }).success,
    ).toBe(false);
  });

  it('rejects an invalid profile', () => {
    expect(
      attackRunRequestSchema.safeParse({ profile: 'nope', targetUrl: 'http://127.0.0.1:9102' })
        .success,
    ).toBe(false);
  });
});

describe('startAttackRun / pollAttackRun', () => {
  beforeEach(() => {
    fetchMock.mockReset();
    vi.stubGlobal('fetch', fetchMock);
  });

  afterEach(() => {
    vi.unstubAllGlobals();
  });

  it('posts the request to /api/attacks and parses the handle', async () => {
    fetchMock.mockResolvedValue(
      new Response(JSON.stringify({ runId: 'run_1', status: 'running' }), { status: 200 }),
    );

    const handle = await startAttackRun({ profile: 'fast', targetUrl: 'http://127.0.0.1:9102' });

    const [url, init] = fetchMock.mock.calls[0] ?? [];
    expect(url).toBe('/api/attacks');
    expect(init?.method).toBe('POST');
    expect(handle).toEqual({ runId: 'run_1', status: 'running' });
  });

  it('surfaces a server error message', async () => {
    fetchMock.mockResolvedValue(
      new Response(JSON.stringify({ error: 'targetUrl must target a loopback agent' }), {
        status: 400,
      }),
    );

    await expect(
      startAttackRun({ profile: 'fast', targetUrl: 'http://127.0.0.1:9102' }),
    ).rejects.toThrow('targetUrl must target a loopback agent');
  });

  it('polls a run by id', async () => {
    fetchMock.mockResolvedValue(
      new Response(JSON.stringify({ runId: 'run_1', status: 'complete', report: null }), {
        status: 200,
      }),
    );

    const poll = await pollAttackRun('run_1');

    const [url] = fetchMock.mock.calls[0] ?? [];
    expect(url).toBe('/api/attacks?runId=run_1');
    expect(poll.status).toBe('complete');
  });
});
