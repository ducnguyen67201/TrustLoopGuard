import { McpServer } from '@modelcontextprotocol/sdk/server/mcp.js';
import { z } from 'zod';

import {
  createToolHandlers,
  type ListTracesInput,
  type ToolHandlers,
  type FeatherlaneAIClient,
  type UpsertAgentInput,
} from './handlers';

const jsonValue = z.json();
const jsonObject = z.record(z.string(), jsonValue);
const guardEvent = jsonObject.transform(
  (event) => event as Parameters<FeatherlaneAIClient['submitEvent']>[0],
);
const sideEffect = z.enum([
  'none',
  'read',
  'external_communication',
  'file_write',
  'shell_exec',
  'network_call',
  'db_mutation',
  'api_mutation',
  'memory_write',
  'publish',
]);
const runStatus = z.enum(['completed', 'failed', 'canceled']);
const runKind = z.enum(['chat_session', 'live_call', 'workflow', 'job', 'other']);
const runEventKind = z.enum([
  'user_turn',
  'assistant_turn',
  'tool_call',
  'workflow_step',
  'interruption',
  'retry',
  'system_event',
  'other',
]);

const runInput = z.object({
  agent_id: z.string().min(1),
  kind: runKind,
  status: z.enum(['warming', 'running', 'completed', 'failed', 'canceled']).optional(),
  external_id: z.string().optional(),
  metadata: jsonObject.optional(),
});

const runEventInput = z.object({
  kind: runEventKind,
  sequence: z.number().int().optional(),
  label: z.string().optional(),
  input_summary: z.string().optional(),
  output_summary: z.string().optional(),
  metadata: jsonObject.optional(),
  occurred_at: z.string().optional(),
});

const knowledgeSource = z.object({
  kb_id: z.string().min(1),
  kind: z.enum(['local', 'web']).optional(),
  url: z.string().optional(),
  description: z.string().optional(),
});

const workflowDefinition = z.object({
  source: z.string().min(1),
  definition: jsonObject,
});

const agentSchema = z.object({
  agent_id: z.string().min(1),
  display_name: z.string().min(1),
  scope: z.object({ in_scope: z.array(z.string()), out_of_scope: z.array(z.string()) }).optional(),
  authority: z
    .object({ can_promise: z.array(z.string()), cannot_promise: z.array(z.string()) })
    .optional(),
  tone: z.object({ target: z.string(), forbidden: z.array(z.string()) }).optional(),
  knowledge_sources: z.array(knowledgeSource).optional(),
  escalation_triggers: z.array(z.string()).optional(),
  system_prompt: z.string().optional(),
  workflow_definition: workflowDefinition.optional(),
  target_url: z.string().optional(),
});

const approval = z.object({
  required: z.boolean(),
  approver_roles: z.array(z.string()).optional(),
  reason: z.string().optional(),
});

const toolParam = z.object({
  path: z.string(),
  role: z.enum(['authority_bearing', 'content_bearing']),
  allowed_sources: z
    .array(
      z.object({
        origin: z.enum(['user', 'system', 'tool', 'memory', 'file', 'web', 'email', 'api', 'unknown']),
        source_id: z.string().optional(),
        kind: z.string().optional(),
      }),
    )
    .optional(),
});

const toolMetadataInput = z.object({
  tool: z.string().min(1),
  side_effect: sideEffect,
  reversible: z.boolean().default(false),
  params: z.array(toolParam).default([]),
  enabled: z.boolean().optional(),
  approval: approval.optional(),
  sandbox_hint: jsonObject.nullable().optional(),
});

const traceSchema = z.object({
  limit: z.number().int().positive().max(100).optional(),
  session_id: z.string().optional(),
});

export function createFeatherlaneAIMcpServer(client: FeatherlaneAIClient): McpServer {
  const server = new McpServer({ name: 'featherlane-ai', version: '0.0.0' });
  registerFeatherlaneAITools(server, createToolHandlers(client));
  return server;
}

export function registerFeatherlaneAITools(server: McpServer, handlers: ToolHandlers): void {
  server.registerTool(
    'submit_guard_event',
    {
      description: 'Submit a GuardEvent and return a Decision.',
      inputSchema: { event: guardEvent },
    },
    handlers.submit_guard_event,
  );
  server.registerTool(
    'start_run',
    { description: 'Create a Featherlane AI run.', inputSchema: runInput.shape },
    (input) => handlers.start_run(runRequest(input)),
  );
  server.registerTool(
    'list_runs',
    { description: 'List Featherlane AI runs.', inputSchema: {} },
    () => handlers.list_runs(),
  );
  server.registerTool(
    'get_run',
    {
      description: 'Fetch a run with its events and recent traces.',
      inputSchema: { run_id: z.string().min(1) },
    },
    handlers.get_run,
  );
  server.registerTool(
    'create_run_event',
    {
      description: 'Append a timeline event to a run.',
      inputSchema: { run_id: z.string().min(1), event: runEventInput },
    },
    (input) => handlers.create_run_event({ run_id: input.run_id, event: runEventRequest(input.event) }),
  );
  server.registerTool(
    'finish_run',
    {
      description: 'Mark a run completed, failed, or canceled.',
      inputSchema: { run_id: z.string().min(1), status: runStatus.optional() },
    },
    handlers.finish_run,
  );
  server.registerTool(
    'validate_policy',
    {
      description: 'Validate policy YAML without saving it.',
      inputSchema: { source: z.string().min(1) },
    },
    handlers.validate_policy,
  );
  server.registerTool(
    'list_policies',
    { description: 'List policy summaries.', inputSchema: {} },
    () => handlers.list_policies(),
  );
  server.registerTool(
    'get_policy',
    { description: 'Fetch a policy document.', inputSchema: { policy_id: z.string().min(1) } },
    handlers.get_policy,
  );
  server.registerTool(
    'upsert_policy',
    {
      description: 'Create or update a policy from YAML.',
      inputSchema: { source: z.string().min(1) },
    },
    handlers.upsert_policy,
  );
  server.registerTool(
    'set_policy_enabled',
    {
      description: 'Enable or disable a policy.',
      inputSchema: { policy_id: z.string().min(1), enabled: z.boolean() },
    },
    handlers.set_policy_enabled,
  );
  server.registerTool(
    'list_agents',
    { description: 'List agent profiles.', inputSchema: {} },
    () => handlers.list_agents(),
  );
  server.registerTool(
    'upsert_agent',
    {
      description: 'Create or update an agent profile.',
      inputSchema: agentSchema.shape,
    },
    (input) => handlers.upsert_agent(agentInput(input)),
  );
  server.registerTool(
    'list_tool_metadata',
    { description: 'List registered tool metadata.', inputSchema: {} },
    () => handlers.list_tool_metadata(),
  );
  server.registerTool(
    'upsert_tool_metadata',
    {
      description: 'Create or update tool metadata.',
      inputSchema: toolMetadataInput.shape,
    },
    (input) => handlers.upsert_tool_metadata(toolMetadataRequest(input)),
  );
  server.registerTool(
    'list_traces',
    {
      description: 'List recent traces.',
      inputSchema: traceSchema.shape,
    },
    (input) => handlers.list_traces(traceInput(input)),
  );
  server.registerTool(
    'list_run_traces',
    { description: 'List traces for a run.', inputSchema: { run_id: z.string().min(1) } },
    handlers.list_run_traces,
  );
}

function runRequest(input: z.infer<typeof runInput>): Parameters<FeatherlaneAIClient['startRun']>[0] {
  return {
    agent_id: input.agent_id,
    kind: input.kind,
    ...(input.status !== undefined ? { status: input.status } : {}),
    ...(input.external_id !== undefined ? { external_id: input.external_id } : {}),
    ...(input.metadata !== undefined ? { metadata: input.metadata } : {}),
  };
}

function runEventRequest(
  input: z.infer<typeof runEventInput>,
): Parameters<FeatherlaneAIClient['createRunEvent']>[1] {
  return {
    kind: input.kind,
    ...(input.sequence !== undefined ? { sequence: input.sequence } : {}),
    ...(input.label !== undefined ? { label: input.label } : {}),
    ...(input.input_summary !== undefined ? { input_summary: input.input_summary } : {}),
    ...(input.output_summary !== undefined ? { output_summary: input.output_summary } : {}),
    ...(input.metadata !== undefined ? { metadata: input.metadata } : {}),
    ...(input.occurred_at !== undefined ? { occurred_at: input.occurred_at } : {}),
  };
}

function agentInput(input: z.infer<typeof agentSchema>): UpsertAgentInput {
  return {
    agent_id: input.agent_id,
    display_name: input.display_name,
    ...(input.scope !== undefined ? { scope: input.scope } : {}),
    ...(input.authority !== undefined ? { authority: input.authority } : {}),
    ...(input.tone !== undefined ? { tone: input.tone } : {}),
    ...(input.knowledge_sources !== undefined
      ? { knowledge_sources: input.knowledge_sources.map(knowledgeSourceInput) }
      : {}),
    ...(input.escalation_triggers !== undefined
      ? { escalation_triggers: input.escalation_triggers }
      : {}),
    ...(input.system_prompt !== undefined ? { system_prompt: input.system_prompt } : {}),
    ...(input.workflow_definition !== undefined
      ? { workflow_definition: input.workflow_definition }
      : {}),
    ...(input.target_url !== undefined ? { target_url: input.target_url } : {}),
  };
}

function knowledgeSourceInput(
  input: z.infer<typeof knowledgeSource>,
): Exclude<UpsertAgentInput['knowledge_sources'], undefined>[number] {
  return {
    kb_id: input.kb_id,
    ...(input.kind !== undefined ? { kind: input.kind } : {}),
    ...(input.url !== undefined ? { url: input.url } : {}),
    ...(input.description !== undefined ? { description: input.description } : {}),
  };
}

function toolMetadataRequest(
  input: z.infer<typeof toolMetadataInput>,
): Parameters<FeatherlaneAIClient['upsertToolMetadata']>[0] {
  return {
    tool: input.tool,
    side_effect: input.side_effect,
    reversible: input.reversible,
    params: input.params.map((param) => ({
      path: param.path,
      role: param.role,
      ...(param.allowed_sources !== undefined
        ? { allowed_sources: param.allowed_sources.map(allowedSourceInput) }
        : {}),
    })),
    ...(input.enabled !== undefined ? { enabled: input.enabled } : {}),
    ...(input.approval !== undefined ? { approval: approvalInput(input.approval) } : {}),
    ...(input.sandbox_hint !== undefined ? { sandbox_hint: input.sandbox_hint } : {}),
  };
}

function allowedSourceInput(
  input: Exclude<z.infer<typeof toolParam>['allowed_sources'], undefined>[number],
): Exclude<
  Parameters<FeatherlaneAIClient['upsertToolMetadata']>[0]['params'][number]['allowed_sources'],
  undefined
>[number] {
  return {
    origin: input.origin,
    ...(input.source_id !== undefined ? { source_id: input.source_id } : {}),
    ...(input.kind !== undefined ? { kind: input.kind } : {}),
  };
}

function approvalInput(
  input: z.infer<typeof approval>,
): Exclude<Parameters<FeatherlaneAIClient['upsertToolMetadata']>[0]['approval'], undefined> {
  return {
    required: input.required,
    ...(input.approver_roles !== undefined ? { approver_roles: input.approver_roles } : {}),
    ...(input.reason !== undefined ? { reason: input.reason } : {}),
  };
}

function traceInput(input: z.infer<typeof traceSchema>): ListTracesInput {
  return {
    ...(input.limit !== undefined ? { limit: input.limit } : {}),
    ...(input.session_id !== undefined ? { session_id: input.session_id } : {}),
  };
}
