'use client';

import { FileCode2, Info, Target } from 'lucide-react';
import { useMemo } from 'react';
import type { ReactNode } from 'react';

import { Badge } from '@/components/ui/badge';
import { EmptyState } from '@/components/ui/empty-state';
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

type VerdictVariant = 'allow' | 'rewrite' | 'block' | 'escalate';

function actionVariant(action: PolicyDraft['action']): VerdictVariant {
  if (action === 'rewrite') return 'rewrite';
  if (action === 'escalate') return 'escalate';
  return 'block';
}

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
        title="Too advanced for the visual builder"
        description={`${parsed.reason} Switch to the YAML tab to edit nested matchers or fields the form does not support yet.`}
        action={
          <Badge variant="outline" className="font-mono uppercase tracking-wide">
            Advanced YAML
          </Badge>
        }
        className="min-h-[360px]"
      />
    );
  }

  const draft = parsed.draft;
  const update = <K extends keyof PolicyDraft>(key: K, value: PolicyDraft[K]) => {
    onYamlChange(draftToYaml({ ...draft, [key]: value }));
  };

  const verdict = actionVariant(draft.action);

  return (
    <div className="grid min-w-0 gap-6 lg:grid-cols-[minmax(0,1fr)_minmax(20rem,0.82fr)]">
      <fieldset disabled={disabled} className="grid min-w-0 gap-6">
        <Section title="Identity">
          <Field label="Policy ID" htmlFor="builder-policy-id">
            <Input
              id="builder-policy-id"
              value={draft.id}
              onChange={(event) => update('id', event.target.value)}
              className="font-mono"
            />
          </Field>
          <Field label="Description" htmlFor="builder-description">
            <Input
              id="builder-description"
              value={draft.description}
              onChange={(event) => update('description', event.target.value)}
            />
          </Field>
        </Section>

        <Section title="Targeting">
          <div className="grid gap-4 md:grid-cols-2">
            <Field label="Channels" htmlFor="builder-channels">
              <Input
                id="builder-channels"
                value={joinList(draft.channels)}
                onChange={(event) => update('channels', splitList(event.target.value))}
                placeholder="chat"
                className="font-mono"
              />
            </Field>
            <Field label="Domains" htmlFor="builder-domains">
              <Input
                id="builder-domains"
                value={joinList(draft.domains)}
                onChange={(event) => update('domains', splitList(event.target.value))}
                placeholder="gateway_output_check"
                className="font-mono"
              />
            </Field>
          </div>
          <div className="grid gap-4 md:grid-cols-[12rem_1fr]">
            <Field label="Match type" htmlFor="builder-match-type">
              <Select
                value={draft.matchType}
                onValueChange={(value) => update('matchType', value as PolicyDraft['matchType'])}
              >
                <SelectTrigger id="builder-match-type" className="font-mono">
                  <SelectValue />
                </SelectTrigger>
                <SelectContent>
                  {POLICY_MATCH_TYPES.map((matchType) => (
                    <SelectItem key={matchType} value={matchType} className="font-mono">
                      {matchType}
                    </SelectItem>
                  ))}
                </SelectContent>
              </Select>
            </Field>
            <Field label="Match value" htmlFor="builder-match-value">
              <Textarea
                id="builder-match-value"
                value={draft.matchValue}
                onChange={(event) => update('matchValue', event.target.value)}
                rows={3}
                className="font-mono"
              />
            </Field>
          </div>
        </Section>

        <Section title="Verdict">
          <div className="grid gap-4 md:grid-cols-2">
            <Field label="Severity" htmlFor="builder-severity">
              <Select
                value={draft.severity}
                onValueChange={(value) => update('severity', value as PolicyDraft['severity'])}
              >
                <SelectTrigger id="builder-severity" className="font-mono">
                  <SelectValue />
                </SelectTrigger>
                <SelectContent>
                  {POLICY_SEVERITIES.map((severity) => (
                    <SelectItem key={severity} value={severity} className="font-mono">
                      {severity}
                    </SelectItem>
                  ))}
                </SelectContent>
              </Select>
            </Field>
            <Field label="Action" htmlFor="builder-action">
              <Select
                value={draft.action}
                onValueChange={(value) => update('action', value as PolicyDraft['action'])}
              >
                <SelectTrigger id="builder-action" className="font-mono">
                  <SelectValue />
                </SelectTrigger>
                <SelectContent>
                  {POLICY_ACTIONS.map((action) => (
                    <SelectItem key={action} value={action} className="font-mono">
                      {action}
                    </SelectItem>
                  ))}
                </SelectContent>
              </Select>
            </Field>
          </div>
          {draft.action === 'rewrite' ? (
            <Field label="Safe rewrite" htmlFor="builder-rewrite">
              <Textarea
                id="builder-rewrite"
                value={draft.rewrite ?? ''}
                onChange={(event) => update('rewrite', event.target.value)}
                rows={2}
              />
            </Field>
          ) : null}
          <Field label="Owner agent ID" htmlFor="builder-owner-agent">
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
            Runtime verdict
          </div>
          <div className="mt-3 flex flex-wrap items-center gap-2">
            <Badge variant={verdict} className="font-mono uppercase">
              {draft.action}
            </Badge>
            <Badge variant="outline" className="font-mono uppercase tracking-wide">
              {draft.severity}
            </Badge>
          </div>
          <p className="mt-3 flex gap-2 text-xs leading-relaxed text-muted-foreground">
            <Info className="mt-0.5 size-3.5 shrink-0" aria-hidden />
            <span>
              These fields define what content is caught and which verdict the runtime returns. The
              response spoken after a gateway block is still controlled by the route&apos;s
              enforcement profile fallback message.
            </span>
          </p>
        </div>

        <div className="min-w-0 overflow-hidden rounded-lg border bg-card shadow-sm">
          <div className="flex items-center gap-2 border-b bg-muted/40 px-3 py-2">
            <FileCode2 className="size-3.5 text-muted-foreground" aria-hidden />
            <Label className="font-mono text-[10px] uppercase tracking-wider text-muted-foreground">
              Generated YAML
            </Label>
          </div>
          <pre className="max-h-[300px] overflow-auto whitespace-pre-wrap break-words bg-muted/30 p-3 font-mono text-xs leading-relaxed">
            {draftToYaml(draft)}
          </pre>
        </div>
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
  children,
}: {
  label: string;
  htmlFor: string;
  children: ReactNode;
}) {
  return (
    <div className="grid gap-1.5">
      <Label htmlFor={htmlFor} className="text-xs font-medium text-foreground">
        {label}
      </Label>
      {children}
    </div>
  );
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
