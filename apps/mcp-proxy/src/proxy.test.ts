import { Client as McpClient } from '@modelcontextprotocol/sdk/client/index.js';
import { InMemoryTransport } from '@modelcontextprotocol/sdk/inMemory.js';
import { Server } from '@modelcontextprotocol/sdk/server/index.js';
import {
  CallToolRequestSchema,
  ListToolsRequestSchema,
  type CallToolRequest,
} from '@modelcontextprotocol/sdk/types.js';
import type {
  AuthorizedActionOptions,
  AuthorizedActionResult,
  AuthorizationDecision,
  AuthorizationEffect,
} from '@featherlane-ai/sdk';
import { afterEach, describe, expect, it, vi } from 'vitest';

import { schemaHash } from './canonical-json';
import type { ProxyConfig } from './config';
import { type AuthorizationGuard, createProxyServer, mcpOperation } from './proxy';

const config: ProxyConfig = {
  baseUrl: 'http://127.0.0.1:8080',
  apiKey: 'tl_test',
  agentId: 'agent-1',
  command: 'unused-in-memory',
  args: [],
  serverId: 'mail:prod',
};

const inputSchema = {
  type: 'object' as const,
  properties: {
    body: { type: 'string' },
    to: { type: 'string' },
  },
  required: ['to'],
};

function guardDecision(effect: AuthorizationEffect): AuthorizationDecision {
  return {
    trace_id: `trace-${effect}`,
    domain: 'tool',
    effect,
    reason: effect,
    findings: [],
    latency_ms: 1n,
  };
}

function allowingGuard(captured: AuthorizedActionOptions[]): AuthorizationGuard {
  return {
    async withAuthorizedAction<T>(
      options: AuthorizedActionOptions,
      execute: (parameters: Readonly<Record<string, unknown>>) => Promise<T>,
    ): Promise<AuthorizedActionResult<T>> {
      captured.push(options);
      return {
        decision: guardDecision('permit'),
        executed: true,
        value: await execute(options.parameters ?? {}),
      };
    },
  };
}

function rejectingGuard(effect: AuthorizationEffect): AuthorizationGuard {
  return {
    async withAuthorizedAction<T>(): Promise<AuthorizedActionResult<T>> {
      return { decision: guardDecision(effect), executed: false };
    },
  };
}

type Harness = {
  upstream: McpClient;
  downstreamCalls: CallToolRequest['params'][];
  close: () => Promise<void>;
};

const openHarnesses: Harness[] = [];

async function createHarness(guard: AuthorizationGuard): Promise<Harness> {
  const downstreamCalls: CallToolRequest['params'][] = [];
  const downstreamServer = new Server(
    { name: 'fake-downstream', version: '1.0.0' },
    { capabilities: { tools: {} } },
  );
  downstreamServer.setRequestHandler(ListToolsRequestSchema, async () => ({
    tools: [
      {
        name: 'send/email',
        description: 'Send an email',
        inputSchema,
      },
    ],
  }));
  downstreamServer.setRequestHandler(CallToolRequestSchema, async (request) => {
    downstreamCalls.push(request.params);
    return { content: [{ type: 'text', text: JSON.stringify(request.params.arguments) }] };
  });

  const downstream = new McpClient(
    { name: 'proxy-downstream-client', version: '1.0.0' },
    { capabilities: {} },
  );
  const [downstreamServerTransport, downstreamClientTransport] =
    InMemoryTransport.createLinkedPair();
  await downstreamServer.connect(downstreamServerTransport);
  await downstream.connect(downstreamClientTransport);

  const proxyServer = createProxyServer(config, downstream, guard);
  const upstream = new McpClient({ name: 'fake-host', version: '1.0.0' }, { capabilities: {} });
  const [proxyTransport, hostTransport] = InMemoryTransport.createLinkedPair();
  await proxyServer.connect(proxyTransport);
  await upstream.connect(hostTransport);

  const harness = {
    upstream,
    downstreamCalls,
    close: async () => {
      await Promise.allSettled([
        upstream.close(),
        proxyServer.close(),
        downstream.close(),
        downstreamServer.close(),
      ]);
    },
  };
  openHarnesses.push(harness);
  return harness;
}

afterEach(async () => {
  await Promise.allSettled(openHarnesses.splice(0).map((harness) => harness.close()));
});

describe('transparent MCP proxy', () => {
  it('mirrors tools and forwards the exact approved arguments once', async () => {
    const captured: AuthorizedActionOptions[] = [];
    const harness = await createHarness(allowingGuard(captured));

    const listed = await harness.upstream.listTools();
    expect(listed.tools).toEqual([
      {
        name: 'send/email',
        description: 'Send an email',
        inputSchema,
      },
    ]);

    const args = { to: 'a@example.com', body: 'hello' };
    const result = await harness.upstream.callTool({ name: 'send/email', arguments: args });

    expect(result.isError).not.toBe(true);
    expect(harness.downstreamCalls).toEqual([{ name: 'send/email', arguments: args }]);
    expect(captured).toHaveLength(1);
    expect(captured[0]).toMatchObject({
      agentId: 'agent-1',
      operation: 'mcp:mail%3Aprod:send%2Femail',
      parameters: args,
      toolIdentity: {
        server_id: 'mail:prod',
        tool_name: 'send/email',
        schema_hash: schemaHash(inputSchema),
      },
    });
  });

  it.each(['deny', 'transform', 'require_approval', 'defer'] as const)(
    'returns a safe MCP error and does not call downstream on %s',
    async (effect) => {
      const harness = await createHarness(rejectingGuard(effect));

      const result = await harness.upstream.callTool({
        name: 'send/email',
        arguments: { to: 'a@example.com' },
      });

      expect(result.isError).toBe(true);
      expect(harness.downstreamCalls).toHaveLength(0);
    },
  );

  it('fails closed when Featherlane AI is unavailable', async () => {
    const guard: AuthorizationGuard = {
      async withAuthorizedAction<T>(): Promise<AuthorizedActionResult<T>> {
        throw new Error('server unavailable');
      },
    };
    const harness = await createHarness(guard);

    const result = await harness.upstream.callTool({
      name: 'send/email',
      arguments: { to: 'a@example.com' },
    });

    expect(result.isError).toBe(true);
    expect(harness.downstreamCalls).toHaveLength(0);
  });

  it('honors host cancellation and never invokes downstream later', async () => {
    const guard: AuthorizationGuard = {
      async withAuthorizedAction<T>(
        options: AuthorizedActionOptions,
      ): Promise<AuthorizedActionResult<T>> {
        await new Promise<void>((_resolve, reject) => {
          options.signal?.addEventListener('abort', () => reject(new Error('aborted')), {
            once: true,
          });
        });
        return { decision: guardDecision('permit'), executed: false };
      },
    };
    const harness = await createHarness(guard);
    const controller = new AbortController();

    const pending = harness.upstream.callTool(
      { name: 'send/email', arguments: { to: 'a@example.com' } },
      undefined,
      { signal: controller.signal },
    );
    await vi.waitFor(() => expect(controller.signal.aborted).toBe(false));
    controller.abort();

    await expect(pending).rejects.toThrow();
    await new Promise((resolve) => setTimeout(resolve, 0));
    expect(harness.downstreamCalls).toHaveLength(0);
  });

  it('guards against a buggy approval helper invoking the executor twice', async () => {
    const guard: AuthorizationGuard = {
      async withAuthorizedAction<T>(
        options: AuthorizedActionOptions,
        execute: (parameters: Readonly<Record<string, unknown>>) => Promise<T>,
      ): Promise<AuthorizedActionResult<T>> {
        await execute(options.parameters ?? {});
        await execute(options.parameters ?? {});
        return { decision: guardDecision('permit'), executed: false };
      },
    };
    const harness = await createHarness(guard);

    const result = await harness.upstream.callTool({
      name: 'send/email',
      arguments: { to: 'a@example.com' },
    });

    expect(result.isError).toBe(true);
    expect(harness.downstreamCalls).toHaveLength(1);
  });
});

describe('mcpOperation', () => {
  it('percent-encodes both identity components to prevent collisions', () => {
    expect(mcpOperation('a:b', 'c/d')).toBe('mcp:a%3Ab:c%2Fd');
    expect(mcpOperation('a', 'b:c/d')).not.toBe(mcpOperation('a:b', 'c/d'));
  });
});
