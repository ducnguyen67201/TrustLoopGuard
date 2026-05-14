import { z } from 'zod';
import { http } from './http';
import { policySummarySchema } from './policies';

export interface AgentSummary {
  agentId: string;
  displayName: string;
  hasSystemPrompt: boolean;
}

export interface AgentList {
  agents: AgentSummary[];
}

export interface CreateAgentInput {
  displayName: string;
  systemPrompt: string;
}

export interface CreatedAgent {
  agentId: string;
  displayName: string;
  hasSystemPrompt: boolean;
}

const agentWireSchema = z
  .looseObject({
    agent_id: z.string(),
    display_name: z.string(),
    system_prompt: z.string().optional(),
  })
  .transform(
    (agent): AgentSummary => ({
      agentId: agent.agent_id,
      displayName: agent.display_name,
      hasSystemPrompt: typeof agent.system_prompt === 'string' && agent.system_prompt.trim() !== '',
    }),
  );

const agentListSchema: z.ZodType<AgentList> = z.object({
  agents: z.array(agentWireSchema),
});

const generatedGuardrailsSchema = z.object({
  generated: z.array(policySummarySchema),
});

export async function listAgents(signal?: AbortSignal): Promise<AgentList> {
  return http.get('/api/agents', agentListSchema, { signal });
}

export async function createAgent(
  input: CreateAgentInput,
  signal?: AbortSignal,
): Promise<CreatedAgent> {
  return http.post(
    '/api/agents',
    { displayName: input.displayName, systemPrompt: input.systemPrompt },
    agentWireSchema,
    { signal },
  );
}

export async function generateAgentGuardrails(
  agentId: string,
  signal?: AbortSignal,
): Promise<z.infer<typeof generatedGuardrailsSchema>> {
  return http.post(
    `/api/agents/${encodeURIComponent(agentId)}/guardrails/generate`,
    {},
    generatedGuardrailsSchema,
    { signal },
  );
}
