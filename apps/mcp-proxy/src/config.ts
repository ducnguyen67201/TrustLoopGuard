import { z } from 'zod';

const configSchema = z.object({
  TLG_URL: z.string().url(),
  TLG_API_KEY: z.string().min(1),
  TLG_AGENT_ID: z.string().min(1),
  TLG_MCP_COMMAND: z.string().min(1),
  TLG_MCP_ARGS_JSON: z
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
  TLG_MCP_SERVER_ID: z.string().min(1),
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
    baseUrl: parsed.TLG_URL,
    apiKey: parsed.TLG_API_KEY,
    agentId: parsed.TLG_AGENT_ID,
    command: parsed.TLG_MCP_COMMAND,
    args: parsed.TLG_MCP_ARGS_JSON,
    serverId: parsed.TLG_MCP_SERVER_ID,
  };
}
