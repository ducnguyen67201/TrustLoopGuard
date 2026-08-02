import type { CallToolResult } from '@modelcontextprotocol/sdk/types.js';

import {
  type Client,
  SdkError,
  type AgentAuthority,
  type AgentProfile,
  type AgentScope,
  type AgentTone,
  type RunStatus,
} from '@featherlane-ai/sdk';

type JsonPrimitive = string | number | boolean | null;
export type JsonValue = JsonPrimitive | JsonValue[] | { [key: string]: JsonValue };
export type JsonObject = { [key: string]: JsonValue };

export type ToolResult = CallToolResult;

export type FeatherlaneAIClient = Pick<
  Client,
  | 'submitEvent'
  | 'startRun'
  | 'listRuns'
  | 'getRun'
  | 'createRunEvent'
  | 'finishRun'
  | 'validatePolicy'
  | 'listPolicies'
  | 'getPolicy'
  | 'upsertPolicy'
  | 'setPolicyEnabled'
  | 'listAgents'
  | 'upsertAgent'
  | 'listToolMetadata'
  | 'upsertToolMetadata'
  | 'listTraces'
  | 'listRunTraces'
>;

export interface UpsertAgentInput {
  agent_id: string;
  display_name: string;
  scope?: AgentScope | undefined;
  authority?: AgentAuthority | undefined;
  tone?: AgentTone | undefined;
  knowledge_sources?: AgentProfile['knowledge_sources'] | undefined;
  escalation_triggers?: string[] | undefined;
  workflow_requirements?: AgentProfile['workflow_requirements'] | undefined;
  system_prompt?: string | undefined;
  workflow_definition?: AgentProfile['workflow_definition'] | undefined;
  target_url?: string | undefined;
}

export interface ListTracesInput {
  limit?: number | undefined;
  session_id?: string | undefined;
}

export interface ToolHandlers {
  submit_guard_event(input: { event: Parameters<FeatherlaneAIClient['submitEvent']>[0] }): Promise<ToolResult>;
  start_run(input: Parameters<FeatherlaneAIClient['startRun']>[0]): Promise<ToolResult>;
  list_runs(): Promise<ToolResult>;
  get_run(input: { run_id: string }): Promise<ToolResult>;
  create_run_event(input: {
    run_id: string;
    event: Parameters<FeatherlaneAIClient['createRunEvent']>[1];
  }): Promise<ToolResult>;
  finish_run(input: {
    run_id: string;
    status?: Extract<RunStatus, 'completed' | 'failed' | 'canceled'> | undefined;
  }): Promise<ToolResult>;
  validate_policy(input: { source: string }): Promise<ToolResult>;
  list_policies(): Promise<ToolResult>;
  get_policy(input: { policy_id: string }): Promise<ToolResult>;
  upsert_policy(input: { source: string }): Promise<ToolResult>;
  set_policy_enabled(input: { policy_id: string; enabled: boolean }): Promise<ToolResult>;
  list_agents(): Promise<ToolResult>;
  upsert_agent(input: UpsertAgentInput): Promise<ToolResult>;
  list_tool_metadata(): Promise<ToolResult>;
  upsert_tool_metadata(input: Parameters<FeatherlaneAIClient['upsertToolMetadata']>[0]): Promise<ToolResult>;
  list_traces(input: ListTracesInput): Promise<ToolResult>;
  list_run_traces(input: { run_id: string }): Promise<ToolResult>;
}

export function createToolHandlers(client: FeatherlaneAIClient): ToolHandlers {
  return {
    submit_guard_event: (input) => runTool(() => client.submitEvent(input.event)),
    start_run: (input) => runTool(() => client.startRun(input)),
    list_runs: () => runTool(() => client.listRuns()),
    get_run: (input) => runTool(() => client.getRun(input.run_id)),
    create_run_event: (input) => runTool(() => client.createRunEvent(input.run_id, input.event)),
    finish_run: (input) =>
      runTool(() => client.finishRun(input.run_id, input.status ?? 'completed')),
    validate_policy: (input) => runTool(() => client.validatePolicy(input.source)),
    list_policies: () => runTool(() => client.listPolicies()),
    get_policy: (input) => runTool(() => client.getPolicy(input.policy_id)),
    upsert_policy: (input) => runTool(() => client.upsertPolicy(input.source)),
    set_policy_enabled: (input) =>
      runTool(() => client.setPolicyEnabled(input.policy_id, input.enabled)),
    list_agents: () => runTool(() => client.listAgents()),
    upsert_agent: (input) => runTool(() => client.upsertAgent(agentProfile(input))),
    list_tool_metadata: () => runTool(() => client.listToolMetadata()),
    upsert_tool_metadata: (input) => runTool(() => client.upsertToolMetadata(input)),
    list_traces: (input) => runTool(() => client.listTraces(traceOptions(input))),
    list_run_traces: (input) => runTool(() => client.listRunTraces(input.run_id)),
  };
}

function agentProfile(input: UpsertAgentInput): AgentProfile {
  return {
    agent_id: input.agent_id,
    display_name: input.display_name,
    scope: input.scope ?? { in_scope: [], out_of_scope: [] },
    authority: input.authority ?? { can_promise: [], cannot_promise: [] },
    tone: input.tone ?? { target: 'neutral', forbidden: [] },
    knowledge_sources: input.knowledge_sources ?? [],
    escalation_triggers: input.escalation_triggers ?? [],
    workflow_requirements: input.workflow_requirements ?? [],
    ...(input.system_prompt !== undefined ? { system_prompt: input.system_prompt } : {}),
    ...(input.workflow_definition !== undefined
      ? { workflow_definition: input.workflow_definition }
      : {}),
    ...(input.target_url !== undefined ? { target_url: input.target_url } : {}),
  };
}

async function runTool(call: () => Promise<JsonValue | object>): Promise<ToolResult> {
  try {
    return jsonToolResult(await call());
  } catch (error) {
    if (error instanceof SdkError || error instanceof Error) {
      return errorToolResult(error);
    }
    return {
      isError: true,
      content: [{ type: 'text', text: 'tool failed' }],
    };
  }
}

function traceOptions(input: ListTracesInput): Parameters<FeatherlaneAIClient['listTraces']>[0] {
  return {
    ...(input.limit !== undefined ? { limit: input.limit } : {}),
    ...(input.session_id !== undefined ? { sessionId: input.session_id } : {}),
  };
}

export function jsonToolResult(value: JsonValue | object): ToolResult {
  return {
    content: [{ type: 'text', text: JSON.stringify(value, jsonReplacer, 2) }],
  };
}

function jsonReplacer(_key: string, value: JsonValue | bigint): JsonValue | string {
  return typeof value === 'bigint' ? value.toString() : value;
}

export function errorToolResult(error: Error | SdkError): ToolResult {
  const message =
    error instanceof SdkError
      ? `${error.code}: ${error.error.message}`
      : error instanceof Error
        ? error.message
        : 'tool failed';
  return {
    isError: true,
    content: [{ type: 'text', text: message }],
  };
}
