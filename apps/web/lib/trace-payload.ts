export interface AgentTracePayload {
  agent_id?: string;
  event?: {
    principal?: {
      agent_id?: string;
    };
  };
}

export function readTraceAgent(payload: AgentTracePayload): string {
  return payload.agent_id?.trim() || payload.event?.principal?.agent_id?.trim() || 'Runtime agent';
}
