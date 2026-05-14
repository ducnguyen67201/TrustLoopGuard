import { z } from 'zod';

export const POLICY_ACTIONS = ['block', 'rewrite', 'escalate'] as const;
export const POLICY_SEVERITIES = ['low', 'medium', 'high', 'critical'] as const;
export const POLICY_MATCH_TYPES = ['literal', 'regex'] as const;

export const policyDraftSchema = z.object({
  id: z
    .string()
    .trim()
    .min(1, 'id is required')
    .regex(/^[a-z0-9-]+$/, 'lowercase letters, digits, and hyphens only'),
  description: z.string().trim().min(1, 'description is required'),
  matchType: z.enum(POLICY_MATCH_TYPES),
  matchValue: z.string().trim().min(1, 'match value is required'),
  action: z.enum(POLICY_ACTIONS),
  severity: z.enum(POLICY_SEVERITIES),
  rewrite: z.string().trim().optional(),
});

export type PolicyDraft = z.infer<typeof policyDraftSchema>;

export const EMPTY_DRAFT: PolicyDraft = {
  id: '',
  description: '',
  matchType: 'literal',
  matchValue: '',
  action: 'block',
  severity: 'medium',
};

export function draftToYaml(draft: PolicyDraft): string {
  const lines: string[] = [];
  lines.push(`id: ${draft.id || 'new-policy'}`);
  if (draft.description) lines.push(`description: ${yamlQuote(draft.description)}`);
  lines.push('match:');
  lines.push(`  ${draft.matchType}: ${yamlQuote(draft.matchValue)}`);
  lines.push(`action: ${draft.action}`);
  lines.push(`severity: ${draft.severity}`);
  if (draft.action === 'rewrite' && draft.rewrite) {
    lines.push(`rewrite: ${yamlQuote(draft.rewrite)}`);
  }
  return `${lines.join('\n')}\n`;
}

function yamlQuote(raw: string): string {
  if (raw === '') return '""';
  if (/[:#&*!|>'"%@`{}[\],\n]/.test(raw) || /^[-?\s]/.test(raw) || /\s$/.test(raw)) {
    return JSON.stringify(raw);
  }
  return raw;
}
