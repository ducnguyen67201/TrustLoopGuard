// Boundary schemas for the playground.
//
// The SDK ships type-only definitions generated from Rust via tl-codegen,
// which gives us compile-time safety. zod adds runtime safety at the two
// untrusted boundaries that surround Client.check():
//
//   1. Form input from the user  -> formSchema -> CheckRequest
//   2. JSON response from server -> decisionResponseSchema -> Decision*
//
// Why "Decision*" instead of using the SDK's Decision directly: the
// generated `latency_ms` is typed as bigint because it's u128 in Rust,
// but tl-server serializes it as a JSON number. JSON.parse never produces
// bigint, so the SDK type is wrong at runtime. We coerce to `number`
// here at the boundary; real fix belongs in tl-codegen and is tracked
// as a follow-up.

import type { Channel, CheckRequest } from '@trustloopguard/sdk';
import { z } from 'zod';

const channelSchema = z.enum(['voice', 'chat', 'email']) satisfies z.ZodType<Channel>;

export const formSchema = z.object({
  agentId: z.string().trim().min(1, 'agent_id is required'),
  channel: channelSchema,
  policies: z
    .string()
    .trim()
    .min(1, 'at least one policy id is required')
    .transform((raw) =>
      raw
        .split(',')
        .map((p) => p.trim())
        .filter((p) => p.length > 0),
    )
    .refine((arr) => arr.length > 0, { message: 'at least one policy id is required' }),
  input: z.string().trim().min(1, 'input is required'),
  proposedOutput: z.string().trim().min(1, 'proposed_output is required'),
});

export type FormValues = z.input<typeof formSchema>;
export type ParsedForm = z.output<typeof formSchema>;

export function toCheckRequest(parsed: ParsedForm): CheckRequest {
  return {
    agent_id: parsed.agentId,
    channel: parsed.channel,
    input: parsed.input,
    proposed_output: parsed.proposedOutput,
    policies: parsed.policies,
    context: {},
    trace_id: crypto.randomUUID(),
    domain: null,
  };
}

const verdictSchema = z.enum(['allow', 'block', 'rewrite', 'escalate']);
const severitySchema = z.enum(['low', 'medium', 'high', 'critical']);

const triggeredPolicySchema = z.object({
  id: z.string(),
  severity: severitySchema,
  reason: z.string(),
});

export const decisionResponseSchema = z.object({
  trace_id: z.string(),
  verdict: verdictSchema,
  reason: z.string(),
  triggered_policies: z.array(triggeredPolicySchema),
  safe_output: z.string().nullable(),
  // tl-server returns this as a JSON number; SDK types it as bigint (wrong).
  // Parse as a number here so downstream rendering is straightforward.
  latency_ms: z.number().int().nonnegative(),
});

export type DecisionResponse = z.infer<typeof decisionResponseSchema>;
export type Verdict = z.infer<typeof verdictSchema>;
