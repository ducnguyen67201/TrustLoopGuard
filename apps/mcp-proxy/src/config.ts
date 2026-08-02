import { z } from 'zod';

const configSchema = z.object({
  FEATHERLANE_AI_URL: z.string().url(),
  FEATHERLANE_AI_API_KEY: z.string().min(1),
  FEATHERLANE_AI_AGENT_ID: z.string().min(1),
  FEATHERLANE_AI_MCP_COMMAND: z.string().min(1),
  FEATHERLANE_AI_MCP_ARGS_JSON: z
    .string()
    .default('[]')
    .transform((raw, ctx) => {
      try {
        return z.array(z.string()).parse(JSON.parse(raw));
      } catch {
        ctx.addIssue({ code: 'custom', message: 'must be a JSON string array' });
        return z.NEVER;
      }
    }),
  FEATHERLANE_AI_MCP_SERVER_ID: z.string().min(1),
});

export type ProxyConfig = {
  baseUrl: string;
  apiKey: string;
  agentId: string;
  command: string;
  args: string[];
  serverId: string;
};

export function loadConfig(env: NodeJS.ProcessEnv = process.env): ProxyConfig {
  const parsed = configSchema.parse(env);
  return {
    baseUrl: parsed.FEATHERLANE_AI_URL,
    apiKey: parsed.FEATHERLANE_AI_API_KEY,
    agentId: parsed.FEATHERLANE_AI_AGENT_ID,
    command: parsed.FEATHERLANE_AI_MCP_COMMAND,
    args: parsed.FEATHERLANE_AI_MCP_ARGS_JSON,
    serverId: parsed.FEATHERLANE_AI_MCP_SERVER_ID,
  };
}
