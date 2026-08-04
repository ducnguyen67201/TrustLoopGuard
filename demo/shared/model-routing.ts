import routingManifest from '../../config/llm-routing.json';
import { z } from 'zod';

const reasoningEffortSchema = z.enum(['none', 'low', 'medium', 'high', 'xhigh', 'max']);

const providerTargetSchema = z
  .object({
    provider: z.string().min(1),
    model: z.string().trim().min(1),
    deadline_ms: z.number().int().positive(),
    reasoning_effort: reasoningEffortSchema.optional(),
  })
  .strict();

const routeSchema = z
  .object({
    description: z.string().min(1),
    primary: providerTargetSchema,
    fallback: providerTargetSchema.optional(),
    cache_ttl_seconds: z.number().int().nonnegative().optional(),
  })
  .strict();

const providerSchema = z
  .object({
    kind: z.enum(['openai', 'openrouter']),
    api_key_env: z.string().min(1),
    base_url: z.string().url().optional(),
  })
  .strict();

const manifestSchema = z
  .object({
    schema_version: z.literal(1),
    providers: z.record(z.string(), providerSchema),
    routes: z.record(z.string(), routeSchema),
    budgets: z
      .object({
        default_monthly_tokens: z.number().int().nonnegative(),
        tenants: z.record(z.string(), z.number().int().nonnegative()),
      })
      .strict(),
  })
  .strict();

export type DemoModelRouteName = 'demo_default' | 'demo_dispute';
export type DemoReasoningEffort = z.infer<typeof reasoningEffortSchema>;

export interface DemoModelRoute {
  model: string;
  reasoningEffort?: DemoReasoningEffort;
}

export function parseDemoModelRoute(
  manifestInput: object,
  routeName: DemoModelRouteName,
): DemoModelRoute {
  const parsed = manifestSchema.safeParse(manifestInput);
  if (!parsed.success) {
    throw new Error(`Invalid LLM routing manifest: ${parsed.error.message}`);
  }

  const route = parsed.data.routes[routeName];
  if (route === undefined) {
    throw new Error(`LLM routing manifest is missing route "${routeName}"`);
  }
  const provider = parsed.data.providers[route.primary.provider];
  if (provider === undefined) {
    throw new Error(
      `Demo route "${routeName}" references missing provider "${route.primary.provider}"`,
    );
  }
  if (provider.kind !== 'openai') {
    throw new Error(`Demo route "${routeName}" must use an openai provider`);
  }

  return {
    model: route.primary.model,
    ...(route.primary.reasoning_effort === undefined
      ? {}
      : { reasoningEffort: route.primary.reasoning_effort }),
  };
}

export function demoModelRoute(routeName: DemoModelRouteName): DemoModelRoute {
  return parseDemoModelRoute(routingManifest, routeName);
}
