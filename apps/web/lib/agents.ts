import { z } from 'zod';
import { http } from './http';

export interface AgentSummary {
  agentId: string;
  displayName: string;
  hasSystemPrompt: boolean;
  hasWorkflow: boolean;
  /** Loopback endpoint the agent is reachable at (the arena adapter contract). */
  targetUrl?: string;
}

export interface AgentProfile {
  agentId: string;
  displayName: string;
  systemPrompt?: string;
  workflowDefinition?: WorkflowDefinitionInput;
  targetUrl?: string;
  scope: {
    inScope: string[];
    outOfScope: string[];
  };
  authority: {
    canPromise: string[];
    cannotPromise: string[];
  };
  tone: {
    target: string;
    forbidden: string[];
  };
  escalationTriggers: string[];
}

export interface WorkflowDefinitionInput {
  source: string;
  definition: Record<string, unknown>;
}

interface CreateAgentInput {
  displayName: string;
  /** Optional when a workflow definition is supplied instead. */
  systemPrompt?: string;
  /** Optional machine-readable agent definition (e.g. an n8n workflow export). */
  workflowDefinition?: WorkflowDefinitionInput;
  /** Loopback endpoint the agent is reachable at, captured at import. */
  targetUrl?: string;
}

export interface UpdateAgentInput {
  displayName: string;
  systemPrompt?: string;
  workflowDefinition?: WorkflowDefinitionInput;
  targetUrl?: string;
  scope: AgentProfile['scope'];
  authority: AgentProfile['authority'];
  tone: AgentProfile['tone'];
  escalationTriggers: string[];
}

const agentWireSchema = z
  .looseObject({
    agent_id: z.string(),
    display_name: z.string(),
    system_prompt: z.string().optional(),
    workflow_definition: z.unknown().optional(),
    target_url: z.string().optional(),
  })
  .transform(
    (agent): AgentSummary => ({
      agentId: agent.agent_id,
      displayName: agent.display_name,
      hasSystemPrompt: typeof agent.system_prompt === 'string' && agent.system_prompt.trim() !== '',
      hasWorkflow: agent.workflow_definition !== undefined && agent.workflow_definition !== null,
      ...(typeof agent.target_url === 'string' && agent.target_url.trim() !== ''
        ? { targetUrl: agent.target_url }
        : {}),
    }),
  );

const agentListSchema = z
  .object({ agents: z.array(agentWireSchema) })
  .transform((value): AgentSummary[] => value.agents);

const stringArraySchema = z.array(z.string()).catch([]);

const workflowDefinitionSchema = z
  .object({
    source: z.string(),
    definition: z.record(z.string(), z.unknown()),
  })
  .optional();

const agentProfileWireSchema = z
  .looseObject({
    agent_id: z.string(),
    display_name: z.string(),
    system_prompt: z.string().optional(),
    workflow_definition: workflowDefinitionSchema,
    target_url: z.string().optional(),
    scope: z
      .looseObject({
        in_scope: stringArraySchema.optional(),
        out_of_scope: stringArraySchema.optional(),
      })
      .optional(),
    authority: z
      .looseObject({
        can_promise: stringArraySchema.optional(),
        cannot_promise: stringArraySchema.optional(),
      })
      .optional(),
    tone: z
      .looseObject({
        target: z.string().catch('clear-professional'),
        forbidden: stringArraySchema.optional(),
      })
      .optional(),
    escalation_triggers: stringArraySchema.optional(),
  })
  .transform((agent): AgentProfile => {
    const profile: AgentProfile = {
      agentId: agent.agent_id,
      displayName: agent.display_name,
      scope: {
        inScope: agent.scope?.in_scope ?? [],
        outOfScope: agent.scope?.out_of_scope ?? [],
      },
      authority: {
        canPromise: agent.authority?.can_promise ?? [],
        cannotPromise: agent.authority?.cannot_promise ?? [],
      },
      tone: {
        target: agent.tone?.target ?? 'clear-professional',
        forbidden: agent.tone?.forbidden ?? [],
      },
      escalationTriggers: agent.escalation_triggers ?? [],
    };
    if (typeof agent.system_prompt === 'string') profile.systemPrompt = agent.system_prompt;
    if (agent.workflow_definition !== undefined) {
      profile.workflowDefinition = agent.workflow_definition;
    }
    if (typeof agent.target_url === 'string' && agent.target_url.trim() !== '') {
      profile.targetUrl = agent.target_url;
    }
    return profile;
  });

export async function listAgents(signal?: AbortSignal): Promise<AgentSummary[]> {
  return http.get('/api/agents', agentListSchema, { signal });
}

export async function getAgent(agentId: string, signal?: AbortSignal): Promise<AgentProfile> {
  return http.get(`/api/agents/${encodeURIComponent(agentId)}`, agentProfileWireSchema, { signal });
}

export async function createAgent(
  input: CreateAgentInput,
  signal?: AbortSignal,
): Promise<AgentSummary> {
  return http.post(
    '/api/agents',
    {
      displayName: input.displayName,
      ...(input.systemPrompt !== undefined ? { systemPrompt: input.systemPrompt } : {}),
      ...(input.workflowDefinition !== undefined
        ? { workflowDefinition: input.workflowDefinition }
        : {}),
      ...(input.targetUrl !== undefined ? { targetUrl: input.targetUrl } : {}),
    },
    agentWireSchema,
    { signal },
  );
}

export async function updateAgent(
  agentId: string,
  input: UpdateAgentInput,
  signal?: AbortSignal,
): Promise<AgentProfile> {
  return http.put(
    `/api/agents/${encodeURIComponent(agentId)}`,
    {
      displayName: input.displayName,
      ...(input.systemPrompt !== undefined ? { systemPrompt: input.systemPrompt } : {}),
      ...(input.workflowDefinition !== undefined
        ? { workflowDefinition: input.workflowDefinition }
        : {}),
      ...(input.targetUrl !== undefined ? { targetUrl: input.targetUrl } : {}),
      scope: input.scope,
      authority: input.authority,
      tone: input.tone,
      escalationTriggers: input.escalationTriggers,
    },
    agentProfileWireSchema,
    { signal },
  );
}
