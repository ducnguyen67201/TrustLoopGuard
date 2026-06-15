import { z } from 'zod';
import { http } from './http';

interface AgentSummary {
  agentId: string;
  displayName: string;
  hasSystemPrompt: boolean;
}

interface CreateAgentInput {
  displayName: string;
  systemPrompt: string;
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
  })
  .transform(
    (agent): AgentSummary => ({
      agentId: agent.agent_id,
      displayName: agent.display_name,
      hasSystemPrompt: typeof agent.system_prompt === 'string' && agent.system_prompt.trim() !== '',
    }),
  );

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
