import { mkdir, mkdtemp, readdir, rm, symlink, writeFile } from 'node:fs/promises';
import { tmpdir } from 'node:os';
import { join } from 'node:path';

import { afterEach, describe, expect, test, vi } from 'vitest';

import type { JsonValue } from './runtime-types.js';
import {
  authorizeToolCall,
  buildGuardEvent,
  completeToolCall,
  type BridgeOptions,
} from './bridge.js';
import type { HostToolCall, RuntimeRegistration } from './wire.js';

const directories: string[] = [];

async function fixture(): Promise<{
  root: string;
  project: string;
  registration: RuntimeRegistration;
}> {
  const root = await mkdtemp(join(tmpdir(), 'featherlane-ai-bridge-'));
  directories.push(root);
  const project = join(root, 'project');
  await mkdir(project);
  const registration: RuntimeRegistration = {
    root: project,
    url: 'https://api.example.test',
    agentId: 'coding-agent',
    targets: ['claude', 'codex', 'opencode'],
    cliVersion: '0.0.1',
    runtimeVersion: 'v1',
    createdAt: '2026-01-01T00:00:00Z',
    updatedAt: '2026-01-01T00:00:00Z',
  };
  await writeFile(
    join(root, 'registry.json'),
    JSON.stringify({ version: 1, projects: [registration] }),
  );
  return { root, project, registration };
}

function call(project: string, overrides: Partial<HostToolCall> = {}): HostToolCall {
  return {
    host: 'claude',
    event: 'pre',
    toolName: 'Bash',
    callId: 'call-1',
    sessionId: 'session-1',
    cwd: join(project, 'src'),
    projectRoot: project,
    input: { command: 'rm -rf ./build', timeout: 5_000 },
    hostVersion: '2.1.133',
    ...overrides,
  };
}

function response(body: JsonValue, status = 200): Response {
  return new Response(JSON.stringify(body), {
    status,
    headers: { 'content-type': 'application/json' },
  });
}

function options(root: string, fetchImpl: typeof fetch): BridgeOptions {
  return {
    configRoot: root,
    env: {
      FEATHERLANE_AI_API_KEY: 'tl_live_test_only',
      FEATHERLANE_AI_APPROVAL_POLL_MS: '1',
      FEATHERLANE_AI_APPROVAL_TIMEOUT_MS: '20',
    },
    fetchImpl,
    sleep: () => Promise.resolve(),
  };
}

afterEach(async () => {
  await Promise.all(directories.splice(0).map((directory) => rm(directory, { recursive: true })));
});

describe('runtime bridge', () => {
  test('normalizes shell, read, file, network, and unknown tools conservatively', async () => {
    const { project, registration } = await fixture();
    expect(buildGuardEvent(call(project), registration)).toMatchObject({
      kind: 'shell.action.proposed',
      principal: { agent_id: 'coding-agent' },
      action: {
        side_effect: 'shell_exec',
        tool_identity: { server_id: 'claude-code', tool_name: 'Bash' },
        parameters: { command: 'rm -rf ./build', shell: 'bash', timeout_ms: 5_000 },
      },
    });
    expect(
      buildGuardEvent(call(project, { host: 'codex', toolName: 'apply_patch' }), registration),
    ).toMatchObject({
      kind: 'file.action.proposed',
      action: { side_effect: 'file_write', tool_identity: { server_id: 'codex' } },
    });
    expect(
      buildGuardEvent(call(project, { toolName: 'Read' }), registration).action.side_effect,
    ).toBe('read');
    expect(
      buildGuardEvent(call(project, { host: 'opencode', toolName: 'webfetch' }), registration),
    ).toMatchObject({
      kind: 'network.request.proposed',
      action: { side_effect: 'network_call', tool_identity: { server_id: 'opencode' } },
    });
    expect(
      buildGuardEvent(call(project, { toolName: 'mcp__github__create_issue' }), registration),
    ).toMatchObject({
      kind: 'tool.call.proposed',
      action: { side_effect: 'api_mutation' },
    });
  });

  test('passes unmanaged projects through without making a request', async () => {
    const { root, project } = await fixture();
    const fetchMock = vi.fn<typeof fetch>();
    const result = await authorizeToolCall(
      call(project, { cwd: join(root, 'other') }),
      options(root, fetchMock),
    );
    expect(result.managed).toBe(false);
    expect(fetchMock).not.toHaveBeenCalled();
  });

  test('maps permits and denials and sends exactly one typed event', async () => {
    const { root, project } = await fixture();
    const fetchMock = vi
      .fn<typeof fetch>()
      .mockResolvedValue(
        response({ effect: 'deny', reason: 'policy matched', trace_id: 'trace-1' }),
      );
    const denied = await authorizeToolCall(call(project), options(root, fetchMock));
    expect(denied).toMatchObject({ managed: true, allowed: false });
    expect(fetchMock).toHaveBeenCalledTimes(1);
    const init = fetchMock.mock.calls[0]?.[1];
    const event = JSON.parse(String(init?.body)) as {
      action: { invocation_id: string };
      principal: { agent_id: string };
    };
    expect(event).toMatchObject({
      action: { invocation_id: 'call-1' },
      principal: { agent_id: 'coding-agent' },
    });
  });

  test.each(['Read', 'Glob', 'Grep'])('fails closed for %s transport outages', async (toolName) => {
    const { root, project } = await fixture();
    const fetchMock = vi.fn<typeof fetch>().mockResolvedValue(response({ message: 'down' }, 500));
    const result = await authorizeToolCall(call(project, { toolName }), options(root, fetchMock));
    expect(result).toMatchObject({ managed: true, allowed: false });
    expect(result.reason).toContain('HTTP 500');
  });

  test('fails closed for a missing key, missing call id, and malformed response', async () => {
    const { root, project } = await fixture();
    const fetchMock = vi.fn<typeof fetch>().mockResolvedValue(response({ bad: true }));
    const noKey = await authorizeToolCall(call(project), {
      configRoot: root,
      env: {},
      fetchImpl: fetchMock,
    });
    expect(noKey.reason).toContain('FEATHERLANE_AI_API_KEY');
    const noId = await authorizeToolCall(call(project, { callId: '' }), options(root, fetchMock));
    expect(noId.reason).toContain('tool-use id');
    const malformed = await authorizeToolCall(call(project), options(root, fetchMock));
    expect(malformed.reason).toContain('unexpected decision');
  });

  test('stores and consumes the exact lease', async () => {
    const { root, project } = await fixture();
    const fetchMock = vi
      .fn<typeof fetch>()
      .mockResolvedValueOnce(
        response({
          effect: 'permit',
          reason: 'allowed',
          trace_id: 'trace-2',
          lease: { id: 'lease-1' },
        }),
      )
      .mockResolvedValueOnce(response({ id: 'lease-1', status: 'consumed' }));
    const bridgeOptions = options(root, fetchMock);
    const authorized = await authorizeToolCall(call(project), bridgeOptions);
    expect(authorized.allowed).toBe(true);
    expect(await readdir(join(root, 'state'))).toHaveLength(1);
    const completed = await completeToolCall(
      { ...call(project), event: 'post-success' },
      'consumed',
      bridgeOptions,
    );
    expect(completed).toEqual({ completed: 1, retained: 0, errors: [] });
    expect(await readdir(join(root, 'state'))).toHaveLength(0);
    const completion = JSON.parse(String(fetchMock.mock.calls[1]?.[1]?.body)) as {
      status: string;
    };
    expect(completion.status).toBe('consumed');
  });

  test('polls approval, resumes the same invocation, and requires a lease', async () => {
    const { root, project } = await fixture();
    const fetchMock = vi
      .fn<typeof fetch>()
      .mockResolvedValueOnce(
        response({
          effect: 'require_approval',
          reason: 'review',
          trace_id: 'trace-a',
          approval: { id: 'approval-1', poll_after_ms: 1 },
        }),
      )
      .mockResolvedValueOnce(response({ status: 'approved', grant_id: 'grant-1' }))
      .mockResolvedValueOnce(
        response({
          effect: 'permit',
          reason: 'approved',
          trace_id: 'trace-b',
          lease: { id: 'lease-approved' },
        }),
      );
    const result = await authorizeToolCall(call(project), options(root, fetchMock));
    expect(result.allowed).toBe(true);
    const first = JSON.parse(String(fetchMock.mock.calls[0]?.[1]?.body)) as {
      action: { invocation_id: string };
    };
    const resumed = JSON.parse(String(fetchMock.mock.calls[2]?.[1]?.body)) as {
      action: { invocation_id: string; authorization: { grant_id: string } };
    };
    expect(resumed.action.invocation_id).toBe(first.action.invocation_id);
    expect(resumed.action.authorization.grant_id).toBe('grant-1');
  });

  test('retains lease state after three completion failures', async () => {
    const { root, project } = await fixture();
    const fetchMock = vi
      .fn<typeof fetch>()
      .mockResolvedValueOnce(
        response({
          effect: 'permit',
          reason: 'allowed',
          trace_id: 'trace-3',
          lease: { id: 'lease-retained' },
        }),
      )
      .mockResolvedValue(response({ message: 'down' }, 500));
    const bridgeOptions = options(root, fetchMock);
    await authorizeToolCall(call(project), bridgeOptions);
    const completed = await completeToolCall(
      { ...call(project), event: 'post-failure' },
      'canceled',
      bridgeOptions,
    );
    expect(completed.retained).toBe(1);
    expect(fetchMock).toHaveBeenCalledTimes(4);
    expect(await readdir(join(root, 'state'))).toHaveLength(1);
  });

  test('fails closed instead of writing state through a symlink', async () => {
    const { root, project } = await fixture();
    const target = join(root, 'external-state');
    await mkdir(target);
    await symlink(target, join(root, 'state'));
    const fetchMock = vi.fn<typeof fetch>().mockResolvedValue(
      response({
        effect: 'permit',
        reason: 'allowed',
        trace_id: 'trace-4',
        lease: { id: 'lease-symlink' },
      }),
    );
    const result = await authorizeToolCall(call(project), options(root, fetchMock));
    expect(result.allowed).toBe(false);
    expect(await readdir(target)).toHaveLength(0);
  });
});
