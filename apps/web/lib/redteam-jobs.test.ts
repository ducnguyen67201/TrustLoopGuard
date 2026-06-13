import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';

import {
  cancelJob,
  dispatchJob,
  getJob,
  isTerminalStatus,
  listJobs,
  redteamJobDetailSchema,
  redteamJobSummarySchema,
} from './redteam-jobs';

const fetchMock = vi.fn<typeof fetch>();

const SUMMARY = {
  id: 'job_1',
  workspace_id: 'ws',
  environment_id: 'env',
  status: 'queued',
  target: 'http://127.0.0.1:9102',
  profile: 'fast',
  generator: 'deterministic',
  agent_id: null,
  attacks: 0,
  landed: 0,
  blocked: 0,
  error: null,
  created_at: '2026-06-13T00:00:00Z',
  updated_at: '2026-06-13T00:00:00Z',
};

function jsonResponse(body: unknown, status = 200): Response {
  return new Response(JSON.stringify(body), {
    status,
    headers: { 'content-type': 'application/json' },
  });
}

describe('schemas', () => {
  it('accepts a well-formed summary', () => {
    expect(redteamJobSummarySchema.safeParse(SUMMARY).success).toBe(true);
  });

  it('rejects an unknown status', () => {
    expect(redteamJobSummarySchema.safeParse({ ...SUMMARY, status: 'paused' }).success).toBe(false);
  });

  it('parses a detail with results', () => {
    const detail = {
      job: { ...SUMMARY, status: 'complete', attacks: 1, landed: 1 },
      results: [
        {
          seq: 0,
          attack: 'prompt_injection',
          goal: 'exfiltrate',
          outcome: 'landed',
          landed: true,
          prompt: 'ignore previous',
          reply: 'the secret is...',
          trace_id: null,
        },
      ],
    };
    expect(redteamJobDetailSchema.safeParse(detail).success).toBe(true);
  });
});

describe('isTerminalStatus', () => {
  it('treats complete/error/cancelled as terminal', () => {
    expect(isTerminalStatus('complete')).toBe(true);
    expect(isTerminalStatus('error')).toBe(true);
    expect(isTerminalStatus('cancelled')).toBe(true);
    expect(isTerminalStatus('queued')).toBe(false);
    expect(isTerminalStatus('running')).toBe(false);
  });
});

describe('client functions', () => {
  beforeEach(() => {
    fetchMock.mockReset();
    vi.stubGlobal('fetch', fetchMock);
  });

  afterEach(() => {
    vi.unstubAllGlobals();
  });

  it('dispatchJob posts a snake_case body and parses the summary', async () => {
    fetchMock.mockResolvedValue(jsonResponse(SUMMARY, 201));

    const summary = await dispatchJob({ targetUrl: 'http://127.0.0.1:9102', profile: 'fast' });

    expect(summary.id).toBe('job_1');
    const [url, init] = fetchMock.mock.calls[0] ?? [];
    expect(url).toBe('/api/redteam/dispatch');
    expect(init?.method).toBe('POST');
    expect(JSON.parse(String(init?.body))).toEqual({
      target_url: 'http://127.0.0.1:9102',
      profile: 'fast',
    });
  });

  it('dispatchJob forwards generator + agent_id when provided', async () => {
    fetchMock.mockResolvedValue(jsonResponse(SUMMARY, 201));

    await dispatchJob({
      targetUrl: 'http://127.0.0.1:9102',
      profile: 'max',
      generator: 'hackagent',
      agentId: 'agent-9',
    });

    const body = JSON.parse(String(fetchMock.mock.calls[0]?.[1]?.body));
    expect(body).toEqual({
      target_url: 'http://127.0.0.1:9102',
      profile: 'max',
      generator: 'hackagent',
      agent_id: 'agent-9',
    });
  });

  it('dispatchJob throws the server error message on failure', async () => {
    fetchMock.mockResolvedValue(jsonResponse({ error: 'dispatch queue is full' }, 503));

    await expect(
      dispatchJob({ targetUrl: 'http://127.0.0.1:9102', profile: 'fast' }),
    ).rejects.toThrow('dispatch queue is full');
  });

  it('getJob parses the detail', async () => {
    fetchMock.mockResolvedValue(
      jsonResponse({ job: { ...SUMMARY, status: 'complete' }, results: [] }),
    );

    const detail = await getJob('job_1');

    expect(fetchMock.mock.calls[0]?.[0]).toBe('/api/redteam/jobs/job_1');
    expect(detail.job.status).toBe('complete');
  });

  it('listJobs forwards filters and returns the jobs array', async () => {
    fetchMock.mockResolvedValue(jsonResponse({ jobs: [SUMMARY] }));

    const jobs = await listJobs({ agentId: 'agent-9', limit: 5 });

    expect(jobs).toHaveLength(1);
    const url = String(fetchMock.mock.calls[0]?.[0]);
    expect(url).toContain('/api/redteam/jobs?');
    expect(url).toContain('agent_id=agent-9');
    expect(url).toContain('limit=5');
  });

  it('cancelJob posts to the cancel route and parses the summary', async () => {
    fetchMock.mockResolvedValue(jsonResponse({ ...SUMMARY, status: 'cancelled' }));

    const summary = await cancelJob('job_1');

    expect(fetchMock.mock.calls[0]?.[0]).toBe('/api/redteam/jobs/job_1/cancel');
    expect(fetchMock.mock.calls[0]?.[1]?.method).toBe('POST');
    expect(summary.status).toBe('cancelled');
  });
});
