'use client';

import { useMemo } from 'react';
import type { ReactNode } from 'react';

import { Badge } from '@/components/ui/badge';
import { Input } from '@/components/ui/input';
import { Label } from '@/components/ui/label';
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from '@/components/ui/select';
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

export function PolicyBuilderEditor({
  yaml,
  onYamlChange,
  disabled = false,
}: PolicyBuilderEditorProps) {
  const parsed = useMemo(() => yamlToDraft(yaml), [yaml]);

  if (!parsed.ok) {
    return (
      <div className="grid min-h-[360px] place-items-center border bg-muted/30 p-6 text-center">
        <div className="max-w-md">
          <Badge variant="outline" className="rounded-sm font-mono uppercase">
            Advanced YAML
          </Badge>
          <h3 className="mt-3 text-lg font-semibold">This policy is too complex for the builder</h3>
          <p className="mt-2 text-sm text-muted-foreground">{parsed.reason}</p>
          <p className="mt-3 text-sm text-muted-foreground">
            Use the YAML tab to edit nested matchers or fields the form does not support yet.
          </p>
        </div>
      </div>
    );
  }

  const draft = parsed.draft;
  const update = <K extends keyof PolicyDraft>(key: K, value: PolicyDraft[K]) => {
    onYamlChange(draftToYaml({ ...draft, [key]: value }));
  };

  return (
    <div className="grid min-w-0 gap-5 lg:grid-cols-[minmax(0,1fr)_minmax(18rem,0.8fr)]">
      <fieldset disabled={disabled} className="grid min-w-0 gap-4">
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
        <div className="grid gap-3 md:grid-cols-2">
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
        <div className="grid gap-3 md:grid-cols-2">
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
        <div className="grid gap-3 md:grid-cols-[12rem_1fr]">
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
      </fieldset>

      <div className="grid min-w-0 content-start gap-3">
        <div className="border bg-muted/30 p-3">
          <div className="font-medium">What this controls</div>
          <p className="mt-2 text-sm text-muted-foreground">
            These fields define what content is caught and which verdict the runtime returns.
            The response spoken after a gateway block is still controlled by the route&apos;s
            enforcement profile fallback message.
          </p>
        </div>
        <div>
          <Label className="font-mono text-[10px] uppercase tracking-wider text-muted-foreground">
            Generated YAML
          </Label>
          <pre className="mt-2 max-h-[300px] overflow-auto break-words border bg-muted p-3 font-mono text-xs whitespace-pre-wrap">
            {draftToYaml(draft)}
          </pre>
        </div>
      </div>
    </div>
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
    <div className="space-y-1.5">
      <Label
        htmlFor={htmlFor}
        className="font-mono text-[10px] uppercase tracking-wider text-muted-foreground"
      >
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
