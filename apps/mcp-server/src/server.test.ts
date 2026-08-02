import { McpServer } from '@modelcontextprotocol/sdk/server/mcp.js';
import { describe, expect, it, vi } from 'vitest';

import { registerFeatherlaneAITools } from './server';
import { type ToolHandlers, type ToolResult } from './handlers';

interface RegisteredTool {
  name: string;
  config: {
    description?: string;
    inputSchema?: object;
  };
  callback: (input: Record<string, object | string | number | boolean>) => Promise<ToolResult>;
}

function toolResult(): ToolResult {
  return { content: [{ type: 'text', text: '{}' }] };
}

function handlers(): ToolHandlers {
  return {
    submit_guard_event: vi.fn(async () => toolResult()),
    start_run: vi.fn(async () => toolResult()),
    list_runs: vi.fn(async () => toolResult()),
    get_run: vi.fn(async () => toolResult()),
    create_run_event: vi.fn(async () => toolResult()),
    finish_run: vi.fn(async () => toolResult()),
    validate_policy: vi.fn(async () => toolResult()),
    list_policies: vi.fn(async () => toolResult()),
    get_policy: vi.fn(async () => toolResult()),
    upsert_policy: vi.fn(async () => toolResult()),
    set_policy_enabled: vi.fn(async () => toolResult()),
    list_agents: vi.fn(async () => toolResult()),
    upsert_agent: vi.fn(async () => toolResult()),
    list_tool_metadata: vi.fn(async () => toolResult()),
    upsert_tool_metadata: vi.fn(async () => toolResult()),
    list_traces: vi.fn(async () => toolResult()),
    list_run_traces: vi.fn(async () => toolResult()),
  };
}

function registerTools(toolHandlers = handlers()): {
  registered: RegisteredTool[];
  handlers: ToolHandlers;
} {
  const registered: RegisteredTool[] = [];
  const server = {
    registerTool(
      name: string,
      config: RegisteredTool['config'],
      callback: RegisteredTool['callback'],
    ): void {
      registered.push({ name, config, callback });
    },
  };

  registerFeatherlaneAITools(server as McpServer, toolHandlers);

  return { registered, handlers: toolHandlers };
}

function findTool(registered: RegisteredTool[], name: string): RegisteredTool {
  const tool = registered.find((candidate) => candidate.name === name);
  if (!tool) throw new Error(`missing tool ${name}`);
  return tool;
}

describe('registerFeatherlaneAITools', () => {
  it('registers the exposed Featherlane AI tool surface', () => {
    const { registered } = registerTools();

    expect(registered.map((tool) => tool.name)).toEqual([
      'submit_guard_event',
      'start_run',
      'list_runs',
      'get_run',
      'create_run_event',
      'finish_run',
      'validate_policy',
      'list_policies',
      'get_policy',
      'upsert_policy',
      'set_policy_enabled',
      'list_agents',
      'upsert_agent',
      'list_tool_metadata',
      'upsert_tool_metadata',
      'list_traces',
      'list_run_traces',
    ]);
  });

  it('normalizes create and inspection tool inputs before calling handlers', async () => {
    const toolHandlers = handlers();
    const { registered } = registerTools(toolHandlers);

    await findTool(registered, 'start_run').callback({
      agent_id: 'support',
      kind: 'chat_session',
      metadata: { channel: 'chat' },
    });
    expect(toolHandlers.start_run).toHaveBeenCalledWith({
      agent_id: 'support',
      kind: 'chat_session',
      metadata: { channel: 'chat' },
    });

    await findTool(registered, 'list_traces').callback({
      limit: 10,
      session_id: 'sess_1',
    });
    expect(toolHandlers.list_traces).toHaveBeenCalledWith({
      limit: 10,
      session_id: 'sess_1',
    });
  });

  it('omits undefined nested optionals for agent and tool metadata inputs', async () => {
    const toolHandlers = handlers();
    const { registered } = registerTools(toolHandlers);

    await findTool(registered, 'upsert_agent').callback({
      agent_id: 'support',
      display_name: 'Support',
      knowledge_sources: [{ kb_id: 'kb', kind: 'web' }],
    });
    expect(toolHandlers.upsert_agent).toHaveBeenCalledWith({
      agent_id: 'support',
      display_name: 'Support',
      knowledge_sources: [{ kb_id: 'kb', kind: 'web' }],
    });

    await findTool(registered, 'upsert_tool_metadata').callback({
      tool: 'send_email',
      side_effect: 'external_communication',
      reversible: false,
      params: [
        {
          path: 'recipient',
          role: 'authority_bearing',
          allowed_sources: [{ origin: 'user' }],
        },
      ],
      approval: { required: true },
      sandbox_hint: null,
    });
    expect(toolHandlers.upsert_tool_metadata).toHaveBeenCalledWith({
      tool: 'send_email',
      side_effect: 'external_communication',
      reversible: false,
      params: [
        {
          path: 'recipient',
          role: 'authority_bearing',
          allowed_sources: [{ origin: 'user' }],
        },
      ],
      approval: { required: true },
      sandbox_hint: null,
    });
  });
});
