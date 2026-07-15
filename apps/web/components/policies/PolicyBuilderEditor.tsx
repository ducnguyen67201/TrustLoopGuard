'use client';

import { FileCode2, Info, Target } from 'lucide-react';
import { useMemo } from 'react';
import type { ReactNode } from 'react';

import { Badge } from '@/components/ui/badge';
import { EmptyState } from '@/components/ui/empty-state';
import { InfoHint } from '@/components/ui/info-hint';
import { Input } from '@/components/ui/input';
import { Label } from '@/components/ui/label';
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from '@/components/ui/select';
import { Separator } from '@/components/ui/separator';
import { Textarea } from '@/components/ui/textarea';
import {
  draftToYaml,
  POLICY_ACTIONS,
  POLICY_MATCH_TYPES,
  POLICY_SEVERITIES,
  yamlToDraft,
  type PolicyDraft,
} from '@/lib/policy-draft';

interface PolicyBuilderEditorProps {
  yaml: string;
  onYamlChange: (yaml: string) => void;
  disabled?: boolean;
}

type EffectVariant = 'permit' | 'transform' | 'deny' | 'require_approval';

function actionVariant(action: PolicyDraft['action']): EffectVariant {
  if (action === 'transform') return 'transform';
  if (action === 'require_approval') return 'require_approval';
  return 'deny';
}

const ACTION_LABEL: Record<PolicyDraft['action'], string> = {
  deny: 'Deny it',
  transform: 'Use a safe transformed value',
  require_approval: 'Require approval',
};

const SEVERITY_LABEL: Record<PolicyDraft['severity'], string> = {
  low: 'Low',
  medium: 'Medium',
  high: 'High',
  critical: 'Critical',
};

const MATCH_TYPE_LABEL: Record<PolicyDraft['matchType'], string> = {
  literal: 'Exact words',
  regex: 'Pattern (regex)',
};

export function PolicyBuilderEditor({
  yaml,
  onYamlChange,
  disabled = false,
}: PolicyBuilderEditorProps) {
  const parsed = useMemo(() => yamlToDraft(yaml), [yaml]);

  if (!parsed.ok) {
    return (
      <EmptyState
        icon={<FileCode2 className="size-6" aria-hidden />}
        title="This rule is too detailed for the guided form"
        description="It uses options the simple form can't show yet. Open the Advanced (YAML) tab to edit it directly — nothing is lost."
        className="min-h-[360px]"
      />
    );
  }

  const draft = parsed.draft;
  const update = <K extends keyof PolicyDraft>(key: K, value: PolicyDraft[K]) => {
    onYamlChange(draftToYaml({ ...draft, [key]: value }));
  };

  const effect = actionVariant(draft.action);

  return (
    <div className="grid min-w-0 gap-6 lg:grid-cols-[minmax(0,1fr)_minmax(20rem,0.82fr)]">
      <fieldset disabled={disabled} className="grid min-w-0 gap-6">
        <Section title="Name it">
          <Field
            label="Description"
            htmlFor="builder-description"
            hint="A plain-language name for this rule. Shows up in your rules list."
          >
            <Input
              id="builder-description"
              value={draft.description}
              onChange={(event) => update('description', event.target.value)}
              placeholder="Block guaranteed-refund promises"
            />
          </Field>
          <Field
            label="Rule ID"
            htmlFor="builder-policy-id"
            hint="A short, lowercase id the engine uses (e.g. no-pii). Not the friendly name."
          >
            <Input
              id="builder-policy-id"
              value={draft.id}
              onChange={(event) => update('id', event.target.value)}
              className="font-mono"
            />
          </Field>
        </Section>

        <Section title="What to catch">
          <div className="grid gap-4 md:grid-cols-[12rem_1fr]">
            <Field
              label="Look for"
              htmlFor="builder-match-type"
              hint="Exact words, or a flexible pattern for advanced cases."
            >
              <Select
                value={draft.matchType}
                onValueChange={(value) => update('matchType', value as PolicyDraft['matchType'])}
              >
                <SelectTrigger id="builder-match-type">
                  <SelectValue />
                </SelectTrigger>
                <SelectContent>
                  {POLICY_MATCH_TYPES.map((matchType) => (
                    <SelectItem key={matchType} value={matchType}>
                      {MATCH_TYPE_LABEL[matchType]}
                    </SelectItem>
                  ))}
                </SelectContent>
              </Select>
            </Field>
            <Field
              label={draft.matchType === 'regex' ? 'Pattern to match' : 'Words to match'}
              htmlFor="builder-match-value"
              hint={
                draft.matchType === 'regex'
                  ? 'A regular expression. Capitalization rules depend on the pattern.'
                  : 'The phrase to catch. Capitalization is ignored.'
              }
            >
              <Textarea
                id="builder-match-value"
                value={draft.matchValue}
                onChange={(event) => update('matchValue', event.target.value)}
                rows={3}
                className="font-mono"
              />
            </Field>
          </div>
          <details className="text-sm">
            <summary className="cursor-pointer text-xs font-medium text-muted-foreground select-none">
              Narrow it down (optional)
            </summary>
            <div className="mt-3 grid gap-4 md:grid-cols-2">
              <Field
                label="Channels"
                htmlFor="builder-channels"
                hint="Limit to certain channels, like chat. Leave blank for all."
              >
                <Input
                  id="builder-channels"
                  value={joinList(draft.channels)}
                  onChange={(event) => update('channels', splitList(event.target.value))}
                  placeholder="chat"
                  className="font-mono"
                />
              </Field>
              <Field
                label="Check points"
                htmlFor="builder-domains"
                hint="Where in the flow to check, e.g. gateway_output_check. Leave blank for the default."
              >
                <Input
                  id="builder-domains"
                  value={joinList(draft.domains)}
                  onChange={(event) => update('domains', splitList(event.target.value))}
                  placeholder="gateway_output_check"
                  className="font-mono"
                />
              </Field>
            </div>
          </details>
        </Section>

        <Section title="What happens on a match">
          <div className="grid gap-4 md:grid-cols-2">
            <Field
              label="The guardrail will…"
              htmlFor="builder-action"
              hint={<InfoHint term="effect" />}
            >
              <Select
                value={draft.action}
                onValueChange={(value) => update('action', value as PolicyDraft['action'])}
              >
                <SelectTrigger id="builder-action">
                  <SelectValue />
                </SelectTrigger>
                <SelectContent>
                  {POLICY_ACTIONS.map((action) => (
                    <SelectItem key={action} value={action}>
                      {ACTION_LABEL[action]}
                    </SelectItem>
                  ))}
                </SelectContent>
              </Select>
            </Field>
            <Field label="Severity" htmlFor="builder-severity" hint={<InfoHint term="severity" />}>
              <Select
                value={draft.severity}
                onValueChange={(value) => update('severity', value as PolicyDraft['severity'])}
              >
                <SelectTrigger id="builder-severity">
                  <SelectValue />
                </SelectTrigger>
                <SelectContent>
                  {POLICY_SEVERITIES.map((severity) => (
                    <SelectItem key={severity} value={severity}>
                      {SEVERITY_LABEL[severity]}
                    </SelectItem>
                  ))}
                </SelectContent>
              </Select>
            </Field>
          </div>
          {draft.action === 'transform' ? (
            <Field
              label="Replace it with"
              htmlFor="builder-rewrite"
              hint="The safe wording sent through instead of the flagged text."
            >
              <Textarea
                id="builder-rewrite"
                value={draft.rewrite ?? ''}
                onChange={(event) => update('rewrite', event.target.value)}
                rows={2}
              />
            </Field>
          ) : null}
          <Field
            label="Applies to one assistant"
            htmlFor="builder-owner-agent"
            hint="Optional. The ID of a single AI assistant to limit this rule to."
          >
            <Input
              id="builder-owner-agent"
              value={draft.ownerAgentId ?? ''}
              onChange={(event) => update('ownerAgentId', event.target.value)}
              placeholder="optional"
              className="font-mono"
            />
          </Field>
        </Section>
      </fieldset>

      <aside className="grid min-w-0 content-start gap-4">
        <div className="rounded-lg border bg-card p-4 shadow-sm">
          <div className="flex items-center gap-2 text-sm font-medium">
            <Target className="size-4 text-muted-foreground" aria-hidden />
            What this rule does
          </div>
          <p className="mt-3 text-sm leading-relaxed text-foreground [text-wrap:pretty]">
            When a request {matchSummary(draft)},{' '}
            <Badge variant={effect} className="align-middle">
              {ACTION_LABEL[draft.action]}
            </Badge>
            .
          </p>
          <p className="mt-3 flex gap-2 text-xs leading-relaxed text-muted-foreground">
            <Info className="mt-0.5 size-3.5 shrink-0" aria-hidden />
            <span>
              This policy action is applied directly by both SDK checks and Gateway routes. Rewrites
              use the safe replacement in this policy; blocks use the standard safe response.
            </span>
          </p>
        </div>

        <details className="min-w-0 overflow-hidden rounded-lg border bg-card shadow-sm">
          <summary className="flex cursor-pointer items-center gap-2 px-3 py-2 select-none">
            <FileCode2 className="size-3.5 text-muted-foreground" aria-hidden />
            <span className="text-[10px] uppercase tracking-wider text-muted-foreground">
              Advanced: raw rule definition
            </span>
          </summary>
          <pre className="max-h-[300px] overflow-auto whitespace-pre-wrap break-words border-t bg-muted/30 p-3 font-mono text-xs leading-relaxed">
            {draftToYaml(draft)}
          </pre>
        </details>
      </aside>
    </div>
  );
}

function Section({ title, children }: { title: string; children: ReactNode }) {
  return (
    <section className="grid gap-4">
      <div className="flex items-center gap-3">
        <h3 className="font-mono text-[10px] uppercase tracking-[0.16em] text-muted-foreground">
          {title}
        </h3>
        <Separator className="flex-1" />
      </div>
      {children}
    </section>
  );
}

function Field({
  label,
  htmlFor,
  hint,
  children,
}: {
  label: string;
  htmlFor: string;
  hint?: ReactNode;
  children: ReactNode;
}) {
  const inlineHint = hint !== undefined && typeof hint !== 'string';
  return (
    <div className="grid gap-1.5">
      <Label
        htmlFor={htmlFor}
        className="flex items-center gap-1 text-xs font-medium text-foreground"
      >
        {label}
        {inlineHint ? hint : null}
      </Label>
      {children}
      {typeof hint === 'string' ? <p className="text-xs text-muted-foreground">{hint}</p> : null}
    </div>
  );
}

/** A short, plain-language phrase describing what content the rule catches. */
function matchSummary(draft: PolicyDraft): string {
  const value = draft.matchValue.trim();
  if (value === '') {
    return draft.matchType === 'regex' ? 'matches your pattern' : 'contains your chosen words';
  }
  if (draft.matchType === 'regex') return `matches the pattern ${value}`;
  return `contains “${value}”`;
}

function joinList(values: string[] | undefined): string {
  return values?.join(', ') ?? '';
}

function splitList(value: string): string[] | undefined {
  const values = value
    .split(',')
    .map((item) => item.trim())
    .filter(Boolean);
  return values.length === 0 ? undefined : values;
}
