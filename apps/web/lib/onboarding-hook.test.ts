import { spawn } from 'node:child_process';
import { mkdir, mkdtemp, readdir, rm, stat, symlink } from 'node:fs/promises';
import { createServer, type IncomingMessage, type ServerResponse } from 'node:http';
import { tmpdir } from 'node:os';
import { join } from 'node:path';

import { afterEach, describe, expect, test } from 'vitest';

import { CLAUDE_HOOK_SCRIPT } from './onboarding';

type JsonBody = Record<string, object | string | boolean | number | null>;
type RecordedRequest = { method: string; path: string; body: JsonBody | null };
type Handler = (request: RecordedRequest, index: number) => { status?: number; body: JsonBody };

const stateDirectories: string[] = [];

afterEach(async () => {
  await Promise.all(
    stateDirectories.splice(0).map((directory) => rm(directory, { recursive: true })),
  );
});

async function readBody(request: IncomingMessage): Promise<JsonBody | null> {
  const chunks: Buffer[] = [];
  for await (const chunk of request) chunks.push(Buffer.from(chunk));
  if (chunks.length === 0) return null;
  return JSON.parse(Buffer.concat(chunks).toString('utf8')) as JsonBody;
}

async function withServer<T>(
  handler: Handler,
  run: (baseUrl: string, requests: RecordedRequest[]) => Promise<T>,
): Promise<T> {
  const requests: RecordedRequest[] = [];
  const server = createServer(async (request: IncomingMessage, response: ServerResponse) => {
    const recorded = {
      method: request.method ?? 'GET',
      path: request.url ?? '/',
      body: await readBody(request),
    };
    requests.push(recorded);
    const result = handler(recorded, requests.length - 1);
    response.writeHead(result.status ?? 200, { 'content-type': 'application/json' });
    response.end(JSON.stringify(result.body));
  });
  await new Promise<void>((resolve) => server.listen(0, '127.0.0.1', resolve));
  const address = server.address();
  if (!address || typeof address === 'string') throw new Error('mock server did not bind');
  try {
    return await run(`http://127.0.0.1:${address.port}`, requests);
  } finally {
    await new Promise<void>((resolve, reject) =>
      server.close((error) => (error ? reject(error) : resolve())),
    );
  }
}

async function runHook(
  hook: JsonBody,
  baseUrl: string,
  stateDirectory: string,
): Promise<{ stdout: string; stderr: string }> {
  const script = CLAUDE_HOOK_SCRIPT.replace(/^#!.*\n/, '');
  const child = spawn(process.execPath, ['--input-type=module', '--eval', script], {
    env: {
      ...process.env,
      TLG_URL: baseUrl,
      TLG_AGENT_ID: 'coding-agent',
      TLG_HOOK_STATE_DIR: stateDirectory,
      TLG_APPROVAL_TIMEOUT_MS: '100',
      TLG_APPROVAL_POLL_MS: '1',
      CLAUDE_PROJECT_DIR: '/workspace/project',
    },
    stdio: ['pipe', 'pipe', 'pipe'],
  });
  const stdout: Buffer[] = [];
  const stderr: Buffer[] = [];
  child.stdout.on('data', (chunk: Buffer) => stdout.push(chunk));
  child.stderr.on('data', (chunk: Buffer) => stderr.push(chunk));
  child.stdin.end(JSON.stringify(hook));
  const exitCode = await new Promise<number | null>((resolve, reject) => {
    child.once('error', reject);
    child.once('close', resolve);
  });
  expect(exitCode).toBe(0);
  return {
    stdout: Buffer.concat(stdout).toString('utf8'),
    stderr: Buffer.concat(stderr).toString('utf8'),
  };
}

async function newStateDirectory(): Promise<string> {
  const directory = await mkdtemp(join(tmpdir(), 'tlg-hook-test-'));
  stateDirectories.push(directory);
  return directory;
}

function bashHook(overrides: JsonBody = {}): JsonBody {
  return {
    hook_event_name: 'PreToolUse',
    tool_name: 'Bash',
    tool_use_id: 'tool-use-1',
    session_id: 'session-1',
    cwd: '/workspace/project/subdir',
    tool_input: { command: 'rm -rf ./build', timeout: 5000 },
    ...overrides,
  };
}

function permission(output: string): JsonBody {
  return JSON.parse(output) as JsonBody;
}

describe('Claude Code command hook bridge', () => {
  test('sends a typed Bash event and maps deny to a structured denial', async () => {
    const stateDirectory = await newStateDirectory();
    await withServer(
      () => ({ body: { effect: 'deny', reason: 'policy matched', trace_id: 'trace-1' } }),
      async (baseUrl, requests) => {
        const result = await runHook(bashHook(), baseUrl, stateDirectory);
        expect(permission(result.stdout)).toMatchObject({
          hookSpecificOutput: { permissionDecision: 'deny', hookEventName: 'PreToolUse' },
        });
        expect(result.stderr).toBe('');
        expect(requests).toHaveLength(1);
        expect(requests[0]?.body).toMatchObject({
          kind: 'shell.action.proposed',
          principal: { agent_id: 'coding-agent', session_id: 'session-1' },
          action: {
            operation: 'Bash',
            invocation_id: 'tool-use-1',
            side_effect: 'shell_exec',
            parameters: {
              command: 'rm -rf ./build',
              shell: 'bash',
              cwd: '/workspace/project/subdir',
              workspace_root: '/workspace/project',
              timeout_ms: 5000,
              run_in_background: false,
            },
            tool_identity: { server_id: 'claude-code', tool_name: 'Bash' },
          },
        });
      },
    );
  });

  test.each(['defer', 'transform'])('maps %s to deny instead of local ask', async (effect) => {
    const stateDirectory = await newStateDirectory();
    await withServer(
      () => ({ body: { effect, reason: 'cannot authorize', trace_id: 'trace-2' } }),
      async (baseUrl) => {
        const result = await runHook(bashHook(), baseUrl, stateDirectory);
        expect(permission(result.stdout)).toMatchObject({
          hookSpecificOutput: { permissionDecision: 'deny' },
        });
      },
    );
  });

  test.each([422, 500])('fails closed for Bash on HTTP %s', async (status) => {
    const stateDirectory = await newStateDirectory();
    await withServer(
      () => ({ status, body: { message: 'unavailable' } }),
      async (baseUrl) => {
        const result = await runHook(bashHook(), baseUrl, stateDirectory);
        expect(permission(result.stdout)).toMatchObject({
          hookSpecificOutput: { permissionDecision: 'deny' },
        });
      },
    );
  });

  test.each(['denied', 'expired'])('fails closed when approval becomes %s', async (status) => {
    const stateDirectory = await newStateDirectory();
    await withServer(
      (request) =>
        request.path === '/v1/events'
          ? {
              body: {
                effect: 'require_approval',
                reason: 'review required',
                trace_id: 'trace-terminal-approval',
                approval: { id: 'approval-terminal' },
              },
            }
          : { body: { status } },
      async (baseUrl) => {
        const result = await runHook(bashHook(), baseUrl, stateDirectory);
        expect(permission(result.stdout)).toMatchObject({
          hookSpecificOutput: { permissionDecision: 'deny' },
        });
        expect(await readdir(stateDirectory)).toHaveLength(0);
      },
    );
  });

  test('fails closed when approval polling times out', async () => {
    const stateDirectory = await newStateDirectory();
    await withServer(
      (request) =>
        request.path === '/v1/events'
          ? {
              body: {
                effect: 'require_approval',
                reason: 'review required',
                trace_id: 'trace-timeout',
                approval: { id: 'approval-timeout' },
              },
            }
          : { body: { status: 'pending' } },
      async (baseUrl) => {
        const result = await runHook(bashHook(), baseUrl, stateDirectory);
        expect(permission(result.stdout)).toMatchObject({
          hookSpecificOutput: { permissionDecision: 'deny' },
        });
      },
    );
  });

  test('does not override native permissions when a read-only guard request fails', async () => {
    const stateDirectory = await newStateDirectory();
    await withServer(
      () => ({ status: 500, body: { message: 'unavailable' } }),
      async (baseUrl) => {
        const result = await runHook(
          bashHook({
            tool_name: 'Read',
            tool_input: { file_path: '/workspace/project/README.md' },
          }),
          baseUrl,
          stateDirectory,
        );
        expect(result.stdout).toBe('');
      },
    );
  });

  test('polls approval, resumes the exact event, stores the lease, then consumes it', async () => {
    const stateDirectory = await newStateDirectory();
    let eventCount = 0;
    let approvalCount = 0;
    await withServer(
      (request) => {
        if (request.path === '/v1/events') {
          eventCount += 1;
          if (eventCount === 1) {
            return {
              body: {
                effect: 'require_approval',
                reason: 'review required',
                trace_id: 'trace-approval',
                approval: { id: 'approval-1' },
              },
            };
          }
          return {
            body: {
              effect: 'permit',
              reason: 'approved',
              trace_id: 'trace-resume',
              lease: { id: 'lease-1' },
            },
          };
        }
        if (request.path === '/v1/authorization/approvals/approval-1') {
          approvalCount += 1;
          return {
            body:
              approvalCount === 1
                ? { status: 'pending' }
                : { status: 'approved', grant_id: 'grant-1' },
          };
        }
        if (request.path === '/v1/authorization/leases/lease-1/complete') {
          return { body: { id: 'lease-1', status: 'consumed' } };
        }
        return { status: 404, body: { message: 'not found' } };
      },
      async (baseUrl, requests) => {
        const pre = await runHook(bashHook(), baseUrl, stateDirectory);
        expect(permission(pre.stdout)).toMatchObject({
          hookSpecificOutput: { permissionDecision: 'allow' },
        });
        const eventRequests = requests.filter((request) => request.path === '/v1/events');
        expect(eventRequests).toHaveLength(2);
        expect(eventRequests[1]?.body).toMatchObject({
          action: {
            invocation_id: 'tool-use-1',
            authorization: { grant_id: 'grant-1' },
          },
        });
        const [stateFile] = await readdir(stateDirectory);
        expect(stateFile).toBeDefined();
        expect((await stat(stateDirectory)).mode & 0o777).toBe(0o700);
        expect((await stat(join(stateDirectory, stateFile!))).mode & 0o777).toBe(0o600);

        const post = await runHook(
          {
            hook_event_name: 'PostToolUse',
            tool_name: 'Bash',
            tool_use_id: 'tool-use-1',
            session_id: 'session-1',
          },
          baseUrl,
          stateDirectory,
        );
        expect(post.stdout).toBe('');
        expect(await readdir(stateDirectory)).toHaveLength(0);
        expect(requests.at(-1)).toMatchObject({
          path: '/v1/authorization/leases/lease-1/complete',
          body: { status: 'consumed' },
        });
      },
    );
  });

  test('cancels a stored lease after tool failure', async () => {
    const stateDirectory = await newStateDirectory();
    await withServer(
      (request) =>
        request.path === '/v1/events'
          ? {
              body: {
                effect: 'permit',
                reason: 'allowed',
                trace_id: 'trace-3',
                lease: { id: 'lease-failed' },
              },
            }
          : { body: { id: 'lease-failed', status: 'canceled' } },
      async (baseUrl, requests) => {
        await runHook(bashHook(), baseUrl, stateDirectory);
        await runHook(
          {
            hook_event_name: 'PostToolUseFailure',
            tool_name: 'Bash',
            tool_use_id: 'tool-use-1',
            session_id: 'session-1',
          },
          baseUrl,
          stateDirectory,
        );
        expect(requests.at(-1)).toMatchObject({ body: { status: 'canceled' } });
        expect(await readdir(stateDirectory)).toHaveLength(0);
      },
    );
  });

  test('keeps lease state after three completion failures', async () => {
    const stateDirectory = await newStateDirectory();
    await withServer(
      (request) =>
        request.path === '/v1/events'
          ? {
              body: {
                effect: 'permit',
                reason: 'allowed',
                trace_id: 'trace-retained',
                lease: { id: 'lease-retained' },
              },
            }
          : { status: 500, body: { message: 'unavailable' } },
      async (baseUrl, requests) => {
        await runHook(bashHook(), baseUrl, stateDirectory);
        const post = await runHook(
          {
            hook_event_name: 'PostToolUse',
            tool_name: 'Bash',
            tool_use_id: 'tool-use-1',
            session_id: 'session-1',
          },
          baseUrl,
          stateDirectory,
        );
        expect(post.stderr).toContain('state retained');
        expect(await readdir(stateDirectory)).toHaveLength(1);
        expect(requests.filter((request) => request.path.endsWith('/complete'))).toHaveLength(3);
      },
    );
  });

  test('stores parallel tool leases under distinct state keys', async () => {
    const stateDirectory = await newStateDirectory();
    await withServer(
      (request) => {
        const invocationId = String(
          (request.body?.['action'] as Record<string, object | string>)?.['invocation_id'],
        );
        return {
          body: {
            effect: 'permit',
            reason: 'allowed',
            trace_id: 'trace-' + invocationId,
            lease: { id: 'lease-' + invocationId },
          },
        };
      },
      async (baseUrl) => {
        const results = await Promise.all([
          runHook(bashHook({ tool_use_id: 'tool-use-1' }), baseUrl, stateDirectory),
          runHook(bashHook({ tool_use_id: 'tool-use-2' }), baseUrl, stateDirectory),
        ]);
        for (const result of results) {
          expect(permission(result.stdout)).toMatchObject({
            hookSpecificOutput: { permissionDecision: 'allow' },
          });
        }
        expect(await readdir(stateDirectory)).toHaveLength(2);
      },
    );
  });

  test('fails closed instead of writing lease state through a symlinked directory', async () => {
    const parent = await newStateDirectory();
    const targetDirectory = join(parent, 'target');
    const stateDirectory = join(parent, 'state-link');
    await mkdir(targetDirectory);
    await symlink(targetDirectory, stateDirectory);

    await withServer(
      () => ({
        body: {
          effect: 'permit',
          reason: 'allowed',
          trace_id: 'trace-symlink',
          lease: { id: 'lease-symlink' },
        },
      }),
      async (baseUrl) => {
        const result = await runHook(bashHook(), baseUrl, stateDirectory);
        expect(permission(result.stdout)).toMatchObject({
          hookSpecificOutput: { permissionDecision: 'deny' },
        });
        expect(await readdir(targetDirectory)).toHaveLength(0);
      },
    );
  });
});
