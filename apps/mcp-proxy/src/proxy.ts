import { Client as McpClient } from '@modelcontextprotocol/sdk/client/index.js';
import { StdioClientTransport } from '@modelcontextprotocol/sdk/client/stdio.js';
import { Server } from '@modelcontextprotocol/sdk/server/index.js';
import {
  CallToolRequestSchema,
  ListToolsRequestSchema,
  type CallToolResult,
} from '@modelcontextprotocol/sdk/types.js';
import {
  Client as TrustLoopClient,
  type AuthorizedActionOptions,
  type AuthorizedActionResult,
} from '@trustloopguard/sdk';

import { schemaHash } from './canonical-json';
import type { ProxyConfig } from './config';

const blocked = (message: string): CallToolResult => ({
  content: [{ type: 'text', text: message }],
  isError: true,
});

type DownstreamClient = Pick<McpClient, 'callTool' | 'close' | 'listTools'>;

export interface AuthorizationGuard {
  withAuthorizedAction<T>(
    options: AuthorizedActionOptions,
    execute: (parameters: Readonly<Record<string, unknown>>) => Promise<T>,
  ): Promise<AuthorizedActionResult<T>>;
}

export function mcpOperation(serverId: string, toolName: string): string {
  return `mcp:${encodeURIComponent(serverId)}:${encodeURIComponent(toolName)}`;
}

export function createProxyServer(
  config: ProxyConfig,
  downstream: DownstreamClient,
  guard: AuthorizationGuard,
): Server {
  const server = new Server(
    { name: 'trustloopguard-mcp-proxy', version: '0.0.0' },
    { capabilities: { tools: {} } },
  );

  server.setRequestHandler(ListToolsRequestSchema, async () => downstream.listTools());
  server.setRequestHandler(CallToolRequestSchema, async (request, extra) => {
    try {
      const listed = await downstream.listTools();
      const tool = listed.tools.find((candidate) => candidate.name === request.params.name);
      if (!tool) return blocked(`unknown downstream tool: ${request.params.name}`);

      let downstreamStarted = false;
      const result = await guard.withAuthorizedAction(
        {
          agentId: config.agentId,
          operation: mcpOperation(config.serverId, tool.name),
          parameters: request.params.arguments ?? {},
          toolIdentity: {
            server_id: config.serverId,
            tool_name: tool.name,
            schema_hash: schemaHash(tool.inputSchema),
          },
          sideEffect: 'api_mutation',
          signal: extra.signal,
        },
        async (parameters) => {
          if (extra.signal.aborted) throw new Error('tool call canceled before execution');
          if (downstreamStarted) throw new Error('downstream tool execution already started');
          downstreamStarted = true;
          return downstream.callTool({ ...request.params, arguments: parameters });
        },
      );
      if (result.executed && result.value) return result.value;
      const requestId = result.decision.approval?.id;
      return blocked(
        `TrustLoopGuard decision: ${result.decision.effect}${requestId ? ` (${requestId})` : ''}`,
      );
    } catch {
      return blocked('TrustLoopGuard could not authorize this tool call');
    }
  });

  return server;
}

export async function createProxy(config: ProxyConfig): Promise<{
  server: Server;
  close: () => Promise<void>;
}> {
  const downstream = new McpClient(
    { name: 'trustloopguard-mcp-proxy', version: '0.0.0' },
    { capabilities: {} },
  );
  const transport = new StdioClientTransport({
    command: config.command,
    args: config.args,
    stderr: 'pipe',
  });
  await downstream.connect(transport);

  const guard = new TrustLoopClient({ baseUrl: config.baseUrl, apiKey: config.apiKey });
  const server = createProxyServer(config, downstream, guard);

  return { server, close: () => downstream.close() };
}
