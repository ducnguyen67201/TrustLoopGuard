import { z } from 'zod';

export const POLICY_ACTIONS = ['deny', 'transform', 'require_approval'] as const;
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
  channels: z.array(z.string().trim().min(1)).optional(),
  domains: z.array(z.string().trim().min(1)).optional(),
  ownerAgentId: z.string().trim().optional(),
});

export type PolicyDraft = z.infer<typeof policyDraftSchema>;

export const EMPTY_DRAFT: PolicyDraft = {
  id: '',
  description: '',
  matchType: 'literal',
  matchValue: '',
  action: 'deny',
  severity: 'medium',
  channels: ['chat'],
};

export function draftToYaml(draft: PolicyDraft): string {
  const lines: string[] = [];
  lines.push(`id: ${draft.id || 'new-policy'}`);
  if (draft.description) lines.push(`description: ${yamlQuote(draft.description)}`);
  if (draft.channels?.length || draft.domains?.length) {
    lines.push('when:');
    if (draft.channels?.length) lines.push(`  channels: ${yamlArray(draft.channels)}`);
    if (draft.domains?.length) lines.push(`  domains: ${yamlArray(draft.domains)}`);
  }
  lines.push('match:');
  lines.push(`  ${draft.matchType}: ${yamlQuote(draft.matchValue)}`);
  lines.push(`action: ${draft.action}`);
  lines.push(`severity: ${draft.severity}`);
  if (draft.action === 'transform' && draft.rewrite) {
    lines.push(`rewrite: ${yamlQuote(draft.rewrite)}`);
  }
  if (draft.ownerAgentId) {
    lines.push(`owner_agent_id: ${yamlQuote(draft.ownerAgentId)}`);
  }
  return `${lines.join('\n')}\n`;
}

type PolicyDraftParseResult = { ok: true; draft: PolicyDraft } | { ok: false; reason: string };

export function yamlToDraft(sourceYaml: string): PolicyDraftParseResult {
  const lines = sourceYaml.split(/\r?\n/);
  const top = new Map<string, string>();
  const match = new Map<string, string>();
  const when = new Map<string, string>();
  let section: 'match' | 'when' | null = null;

  for (const rawLine of lines) {
    const withoutComment = stripYamlComment(rawLine);
    if (withoutComment.trim() === '') continue;
    const indent = withoutComment.match(/^\s*/)?.[0].length ?? 0;
    const line = withoutComment.trim();
    if (line.endsWith(':') && indent === 0) {
      const key = line.slice(0, -1);
      section = key === 'match' || key === 'when' ? key : null;
      continue;
    }
    const separator = line.indexOf(':');
    if (separator === -1) continue;
    const key = line.slice(0, separator).trim();
    const value = line.slice(separator + 1).trim();
    if (indent > 0 && section === 'match') {
      match.set(key, value);
    } else if (indent > 0 && section === 'when') {
      when.set(key, value);
    } else if (indent === 0) {
      section = null;
      top.set(key, value);
    }
  }

  const matchType = match.has('literal') ? 'literal' : match.has('regex') ? 'regex' : null;
  if (matchType === null) {
    return { ok: false, reason: 'Builder supports policies with match.literal or match.regex.' };
  }

  const action = parseEnum(top.get('action'), POLICY_ACTIONS, 'deny');
  if (action === null) {
    return { ok: false, reason: 'Builder supports deny, transform, and require_approval actions.' };
  }
  const severity = parseEnum(top.get('severity'), POLICY_SEVERITIES, 'medium');
  if (severity === null) {
    return { ok: false, reason: 'Builder supports low, medium, high, and critical severity.' };
  }

  const draft = policyDraftSchema.safeParse({
    id: unquoteYamlScalar(top.get('id') ?? ''),
    description: unquoteYamlScalar(top.get('description') ?? ''),
    matchType,
    matchValue: unquoteYamlScalar(match.get(matchType) ?? ''),
    action,
    severity,
    rewrite: optionalScalar(top.get('transform')),
    channels: parseYamlArray(when.get('channels')),
    domains: parseYamlArray(when.get('domains')),
    ownerAgentId: optionalScalar(top.get('owner_agent_id')),
  });

  if (!draft.success) {
    return {
      ok: false,
      reason: draft.error.issues[0]?.message ?? 'Policy YAML is missing a required builder field.',
    };
  }

  return { ok: true, draft: draft.data };
}

function yamlArray(values: string[]): string {
  return `[${values.map(yamlQuote).join(', ')}]`;
}

function yamlQuote(raw: string): string {
  if (raw === '') return '""';
  if (/[:#&*!|>'"%@`{}[\],\n]/.test(raw) || /^[-?\s]/.test(raw) || /\s$/.test(raw)) {
    return JSON.stringify(raw);
  }
  return raw;
}

function stripYamlComment(line: string): string {
  let quote: '"' | "'" | null = null;
  for (let i = 0; i < line.length; i += 1) {
    const char = line[i];
    if ((char === '"' || char === "'") && line[i - 1] !== '\\') {
      quote = quote === char ? null : (quote ?? char);
    }
    if (char === '#' && quote === null) return line.slice(0, i);
  }
  return line;
}

function unquoteYamlScalar(value: string): string {
  const trimmed = value.trim();
  if (trimmed === '') return '';
  if (
    (trimmed.startsWith('"') && trimmed.endsWith('"')) ||
    (trimmed.startsWith("'") && trimmed.endsWith("'"))
  ) {
    try {
      return trimmed.startsWith('"') ? JSON.parse(trimmed) : trimmed.slice(1, -1);
    } catch {
      return trimmed.slice(1, -1);
    }
  }
  return trimmed;
}

function optionalScalar(value: string | undefined): string | undefined {
  const scalar = value === undefined ? '' : unquoteYamlScalar(value);
  return scalar.trim() === '' ? undefined : scalar;
}

function parseYamlArray(value: string | undefined): string[] | undefined {
  if (value === undefined) return undefined;
  const trimmed = value.trim();
  if (!trimmed.startsWith('[') || !trimmed.endsWith(']')) return undefined;
  const inner = trimmed.slice(1, -1).trim();
  if (inner === '') return [];
  return inner
    .split(',')
    .map((item) => unquoteYamlScalar(item))
    .map((item) => item.trim())
    .filter(Boolean);
}

function parseEnum<T extends readonly string[]>(
  value: string | undefined,
  options: T,
  fallback: T[number],
): T[number] | null {
  const parsed = optionalScalar(value) ?? fallback;
  return options.includes(parsed) ? parsed : null;
}
