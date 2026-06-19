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

interface WorkflowDefinitionInput {
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

interface CreatedAgent {
  agentId: string;
  displayName: string;
  hasSystemPrompt: boolean;
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

export async function listAgents(signal?: AbortSignal): Promise<AgentSummary[]> {
  return http.get('/api/agents', agentListSchema, { signal });
}

export async function createAgent(
  input: CreateAgentInput,
  signal?: AbortSignal,
): Promise<CreatedAgent> {
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
