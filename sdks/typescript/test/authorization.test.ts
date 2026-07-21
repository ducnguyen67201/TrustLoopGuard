import { describe, expect, it, vi } from 'vitest';

import { Client, type GuardEvent } from '../src';
import { mockFetch } from './test-utils';

const approvalId = '018f1111-1111-7111-8111-111111111111';
const grantId = '018f2222-2222-7222-8222-222222222222';
const leaseId = '018f3333-3333-7333-8333-333333333333';

function decision(effect: 'permit' | 'require_approval', approval = false, lease = false) {
  return {
    trace_id: `trace-${effect}`,
    domain: 'tool',
    effect,
    reason: effect,
    findings: [],
    latency_ms: 1,
    ...(approval
      ? {
          approval: {
            id: approvalId,
            status: 'pending',
            envelope_hash: 'sha256:v1:reviewed',
            expires_at: '2026-07-15T00:00:00Z',
            poll_after_ms: 1,
          },
        }
      : {}),
    ...(lease
      ? {
          lease: {
            id: leaseId,
            intent_id: '018f4444-4444-7444-8444-444444444444',
            grant_id: grantId,
            attempt_id: 'attempt-1',
            fingerprint: 'sha256:v1:subject',
            status: 'claimed',
            claimed_at: '2026-07-14T00:00:00Z',
            expires_at: '2026-07-14T00:05:00Z',
          },
        }
      : {}),
  };
}

describe('withAuthorizedAction', () => {
  it('polls a common approval, resumes with its grant, and completes one lease', async () => {
    const submitted: GuardEvent[] = [];
    const completions: object[] = [];
    const fetchImpl = mockFetch(async (input, init) => {
      const url = typeof input === 'string' ? input : input instanceof URL ? input.href : input.url;
      if (url.includes(`/authorization/approvals/${approvalId}`)) {
        return Response.json({
          id: approvalId,
          status: 'approved',
          grant_id: grantId,
          envelope: { principal_id: 'agent-1' },
        });
      }
      if (url.includes(`/authorization/leases/${leaseId}/complete`)) {
        completions.push(JSON.parse(String(init?.body)) as object);
        return Response.json({ ...decision('permit', false, true).lease, status: 'consumed' });
      }
      submitted.push(JSON.parse(String(init?.body)) as GuardEvent);
      return Response.json(
        submitted.length === 1
          ? decision('require_approval', true)
          : decision('permit', false, true),
      );
    });
    const client = new Client({ baseUrl: 'http://x', fetchImpl });
    const execute = vi.fn(async (parameters: Readonly<Record<string, unknown>>) => parameters);

    const result = await client.withAuthorizedAction(
      {
        agentId: 'agent-1',
        operation: 'mail/send_email',
        parameters: { to: 'a@example.com', nested: { subject: 'hello' } },
        toolIdentity: {
          server_id: 'mail',
          tool_name: 'send_email',
          schema_hash: 'sha256:v1:schema',
        },
      },
      execute,
    );

    expect(result.executed).toBe(true);
    expect(execute).toHaveBeenCalledTimes(1);
    expect(Object.isFrozen(execute.mock.calls[0]?.[0])).toBe(true);
    expect(Object.isFrozen(execute.mock.calls[0]?.[0].nested as object)).toBe(true);
    expect(submitted).toHaveLength(2);
    expect(submitted[1]?.action.invocation_id).toBe(submitted[0]?.action.invocation_id);
    expect(submitted[0]?.action.authorization).toBeUndefined();
    expect(submitted[1]?.action.authorization?.grant_id).toBe(grantId);
    expect(completions).toEqual([{ status: 'consumed', outcome: { success: true } }]);
  });

  it('keeps one shell invocation through approval resume and lease completion', async () => {
    const submitted: GuardEvent[] = [];
    const fetchImpl = mockFetch(async (input, init) => {
      const url = typeof input === 'string' ? input : input instanceof URL ? input.href : input.url;
      if (url.includes(`/authorization/approvals/${approvalId}`)) {
        return Response.json({ id: approvalId, status: 'approved', grant_id: grantId });
      }
      if (url.includes(`/authorization/leases/${leaseId}/complete`)) {
        return Response.json({ ...decision('permit', false, true).lease, status: 'consumed' });
      }
      submitted.push(JSON.parse(String(init?.body)) as GuardEvent);
      return Response.json(
        submitted.length === 1
          ? decision('require_approval', true)
          : decision('permit', false, true),
      );
    });
    const client = new Client({ baseUrl: 'http://x', fetchImpl });

    const result = await client.withAuthorizedShellAction(
      {
        agentId: 'agent-1',
        command: 'rm -rf ./build',
        invocationId: 'tool-use-shell',
        toolIdentity: {
          server_id: 'claude-code',
          tool_name: 'Bash',
          schema_hash: 'sha256:v1:bash',
        },
        pollIntervalMs: 1,
      },
      async (parameters) => parameters.command,
    );

    expect(result).toMatchObject({ executed: true, value: 'rm -rf ./build' });
    expect(submitted).toHaveLength(2);
    expect(submitted[0]).toMatchObject({
      kind: 'shell.action.proposed',
      action: {
        operation: 'Bash',
        invocation_id: 'tool-use-shell',
        side_effect: 'shell_exec',
      },
    });
    expect(submitted[1]?.action.invocation_id).toBe('tool-use-shell');
    expect(submitted[1]?.action.authorization?.grant_id).toBe(grantId);
  });
});

describe('authorization activity', () => {
  it('lists common receipts from the environment-scoped control-plane endpoint', async () => {
    const fetchImpl = mockFetch(async (input, init) => {
      const url = typeof input === 'string' ? input : input instanceof URL ? input.href : input.url;
      expect(url).toContain('/v1/authorization/receipts');
      expect(init?.method).toBe('GET');
      return Response.json({ receipts: [] });
    });
    const client = new Client({ baseUrl: 'http://x', fetchImpl });

    await expect(client.listAuthorizationReceipts()).resolves.toEqual({ receipts: [] });
  });
});
