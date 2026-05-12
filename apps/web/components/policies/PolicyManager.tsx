'use client';

import { CheckCircle2, FilePlus2, Play, Power, RefreshCw, Save, Trash2, Wand2 } from 'lucide-react';
import { useEffect, useMemo, useState, type ChangeEvent, type ReactNode } from 'react';
import { toast } from 'sonner';
import type { PolicySummary, Severity } from '@trustloopguard/sdk';
import { Badge } from '@/components/ui/badge';
import { Button } from '@/components/ui/button';
import { Input } from '@/components/ui/input';
import { Label } from '@/components/ui/label';
import { Tooltip, TooltipContent, TooltipProvider, TooltipTrigger } from '@/components/ui/tooltip';
import {
  deletePolicy,
  getPolicy,
  listPolicies,
  setPolicyEnabled,
  upsertPolicy,
  validatePolicy,
  type PolicyValidationResult,
} from '@/lib/policies';
import { cn } from '@/lib/utils';

const EMPTY_POLICY = `id: refund-guarantee
description: Prevent guaranteed refund promises.
match:
  literal: guaranteed refund
action: block
severity: high
`;

const INITIAL_BUILDER = {
  id: 'refund-guarantee',
  description: 'Prevent guaranteed refund promises.',
  channel: 'chat',
  domain: 'customer_support',
  agent: '',
  matcherType: 'literal',
  matcherValue: 'guaranteed refund',
  action: 'block',
  severity: 'high',
  rewrite: '',
};

type BuilderValues = typeof INITIAL_BUILDER;
type BuilderKey = keyof BuilderValues;
type LoadState = 'idle' | 'loading' | 'saving';

export function PolicyManager() {
  const [policies, setPolicies] = useState<PolicySummary[]>([]);
  const [selectedId, setSelectedId] = useState<string | null>(null);
  const [sourceYaml, setSourceYaml] = useState(EMPTY_POLICY);
  const [validation, setValidation] = useState<PolicyValidationResult | null>(null);
  const [builder, setBuilder] = useState<BuilderValues>(INITIAL_BUILDER);
  const [state, setState] = useState<LoadState>('idle');
  const selected = useMemo(
    () => policies.find((policy) => policy.id === selectedId) ?? null,
    [policies, selectedId],
  );

  useEffect(() => {
    void refreshPolicies();
  }, []);

  async function refreshPolicies(nextSelectedId = selectedId) {
    setState('loading');
    try {
      const result = await listPolicies();
      setPolicies(result.policies);
      const id = nextSelectedId ?? result.policies[0]?.id ?? null;
      setSelectedId(id);
      if (id !== null) {
        const document = await getPolicy(id);
        setSourceYaml(document.source_yaml);
      }
    } catch (error) {
      toast.error(describeError(error));
    } finally {
      setState('idle');
    }
  }

  async function selectPolicy(policyId: string) {
    setSelectedId(policyId);
    setState('loading');
    try {
      const document = await getPolicy(policyId);
      setSourceYaml(document.source_yaml);
      setValidation(null);
    } catch (error) {
      toast.error(describeError(error));
    } finally {
      setState('idle');
    }
  }

  function startNewPolicy() {
    setSelectedId(null);
    setSourceYaml(EMPTY_POLICY);
    setValidation(null);
  }

  async function runValidation() {
    setState('loading');
    try {
      const result = await validatePolicy(sourceYaml);
      setValidation(result);
      toast[result.valid ? 'success' : 'error'](
        result.valid ? 'Policy is valid' : 'Policy needs changes',
      );
    } catch (error) {
      toast.error(describeError(error));
    } finally {
      setState('idle');
    }
  }

  async function publishPolicy() {
    setState('saving');
    try {
      const document = await upsertPolicy(sourceYaml);
      setSourceYaml(document.source_yaml);
      setSelectedId(document.id);
      setValidation(null);
      await refreshPolicies(document.id);
      toast.success('Policy saved');
    } catch (error) {
      toast.error(describeError(error));
    } finally {
      setState('idle');
    }
  }

  async function togglePolicy(policy: PolicySummary) {
    setState('saving');
    try {
      await setPolicyEnabled(policy.id, !policy.enabled);
      await refreshPolicies(policy.id);
      toast.success(policy.enabled ? 'Policy disabled' : 'Policy enabled');
    } catch (error) {
      toast.error(describeError(error));
    } finally {
      setState('idle');
    }
  }

  async function removePolicy(policy: PolicySummary) {
    setState('saving');
    try {
      await deletePolicy(policy.id);
      const next = policies.find((candidate) => candidate.id !== policy.id)?.id ?? null;
      await refreshPolicies(next);
      if (selectedId === policy.id) startNewPolicy();
      toast.success('Policy deleted');
    } catch (error) {
      toast.error(describeError(error));
    } finally {
      setState('idle');
    }
  }

  function updateBuilder(key: BuilderKey) {
    return (event: ChangeEvent<HTMLInputElement | HTMLSelectElement>) => {
      setBuilder((prev) => ({ ...prev, [key]: event.target.value }));
    };
  }

  function applyBuilder() {
    setSourceYaml(buildYaml(builder));
    setValidation(null);
  }

  const busy = state === 'loading' || state === 'saving';

  return (
    <TooltipProvider>
      <div className="grid gap-6 xl:grid-cols-[320px_minmax(0,1fr)_360px]">
        <section className="rounded-lg border border-border bg-card">
          <PanelHeader
            title="Policies"
            actions={
              <>
                <IconButton label="Refresh" onClick={() => void refreshPolicies()} disabled={busy}>
                  <RefreshCw />
                </IconButton>
                <IconButton label="New policy" onClick={startNewPolicy} disabled={busy}>
                  <FilePlus2 />
                </IconButton>
              </>
            }
          />
          <div className="divide-y divide-border">
            {policies.length === 0 ? (
              <p className="p-4 text-sm text-muted-foreground">No policies</p>
            ) : (
              policies.map((policy) => (
                <button
                  key={policy.id}
                  type="button"
                  onClick={() => void selectPolicy(policy.id)}
                  className={cn(
                    'block w-full px-4 py-3 text-left transition hover:bg-muted/50',
                    selectedId === policy.id ? 'bg-muted' : '',
                  )}
                >
                  <div className="flex items-center justify-between gap-3">
                    <span className="truncate font-mono text-sm">{policy.id}</span>
                    <StatusBadge enabled={policy.enabled} />
                  </div>
                  <div className="mt-2 flex items-center justify-between gap-3">
                    <span className="truncate text-xs text-muted-foreground">
                      {policy.description ?? 'No description'}
                    </span>
                    <SeverityBadge severity={policy.severity} />
                  </div>
                </button>
              ))
            )}
          </div>
        </section>

        <section className="rounded-lg border border-border bg-card">
          <PanelHeader
            title={selected?.id ?? 'Draft policy'}
            subtitle={selected?.enabled === false ? 'disabled' : undefined}
            actions={
              <>
                <IconButton label="Validate" onClick={() => void runValidation()} disabled={busy}>
                  <CheckCircle2 />
                </IconButton>
                <IconButton label="Publish" onClick={() => void publishPolicy()} disabled={busy}>
                  <Save />
                </IconButton>
              </>
            }
          />

          <div className="grid min-h-[640px] grid-rows-[minmax(0,1fr)_auto]">
            <textarea
              value={sourceYaml}
              onChange={(event) => {
                setSourceYaml(event.target.value);
                setValidation(null);
              }}
              spellCheck={false}
              className="min-h-[520px] w-full resize-none border-0 bg-muted/30 p-4 font-mono text-sm leading-6 outline-none focus:ring-1 focus:ring-ring"
            />
            <ValidationPanel validation={validation} />
          </div>
        </section>

        <aside className="rounded-lg border border-border bg-card">
          <PanelHeader
            title="Builder"
            actions={
              <IconButton label="Use builder output" onClick={applyBuilder} disabled={busy}>
                <Wand2 />
              </IconButton>
            }
          />
          <div className="space-y-4 p-4">
            <TextField label="id" value={builder.id} onChange={updateBuilder('id')} />
            <TextField
              label="description"
              value={builder.description}
              onChange={updateBuilder('description')}
            />
            <div className="grid grid-cols-2 gap-3">
              <SelectField
                label="channel"
                value={builder.channel}
                onChange={updateBuilder('channel')}
                options={['chat', 'voice', 'email']}
              />
              <SelectField
                label="severity"
                value={builder.severity}
                onChange={updateBuilder('severity')}
                options={['low', 'medium', 'high', 'critical']}
              />
            </div>
            <TextField label="domain" value={builder.domain} onChange={updateBuilder('domain')} />
            <TextField label="agent" value={builder.agent} onChange={updateBuilder('agent')} />
            <div className="grid grid-cols-[120px_minmax(0,1fr)] gap-3">
              <SelectField
                label="match"
                value={builder.matcherType}
                onChange={updateBuilder('matcherType')}
                options={['literal', 'regex', 'semantic']}
              />
              <TextField
                label="value"
                value={builder.matcherValue}
                onChange={updateBuilder('matcherValue')}
              />
            </div>
            <SelectField
              label="action"
              value={builder.action}
              onChange={updateBuilder('action')}
              options={['allow', 'block', 'rewrite', 'escalate']}
            />
            <TextField
              label="rewrite"
              value={builder.rewrite}
              onChange={updateBuilder('rewrite')}
            />
            <Button className="w-full" onClick={applyBuilder} disabled={busy}>
              <Play />
              Apply
            </Button>
          </div>
        </aside>
      </div>

      {selected !== null ? (
        <div className="mt-4 flex flex-wrap gap-2">
          <Button variant="outline" onClick={() => void togglePolicy(selected)} disabled={busy}>
            <Power />
            {selected.enabled ? 'Disable' : 'Enable'}
          </Button>
          <Button variant="destructive" onClick={() => void removePolicy(selected)} disabled={busy}>
            <Trash2 />
            Delete
          </Button>
        </div>
      ) : null}
    </TooltipProvider>
  );
}

function PanelHeader({
  title,
  subtitle,
  actions,
}: {
  title: string;
  subtitle?: string | undefined;
  actions?: ReactNode;
}) {
  return (
    <header className="flex min-h-14 items-center justify-between gap-3 border-b border-border px-4">
      <div className="min-w-0">
        <h2 className="truncate font-mono text-sm font-semibold">{title}</h2>
        {subtitle !== undefined ? (
          <p className="text-xs text-muted-foreground">{subtitle}</p>
        ) : null}
      </div>
      {actions !== undefined ? <div className="flex items-center gap-2">{actions}</div> : null}
    </header>
  );
}

function IconButton({
  label,
  children,
  onClick,
  disabled,
}: {
  label: string;
  children: ReactNode;
  onClick: () => void;
  disabled?: boolean;
}) {
  return (
    <Tooltip>
      <TooltipTrigger asChild>
        <Button type="button" variant="outline" size="icon" onClick={onClick} disabled={disabled}>
          {children}
          <span className="sr-only">{label}</span>
        </Button>
      </TooltipTrigger>
      <TooltipContent>{label}</TooltipContent>
    </Tooltip>
  );
}

function StatusBadge({ enabled }: { enabled: boolean }) {
  return (
    <Badge variant={enabled ? 'secondary' : 'outline'} className="font-mono">
      {enabled ? 'enabled' : 'disabled'}
    </Badge>
  );
}

function SeverityBadge({ severity }: { severity: Severity }) {
  const className =
    severity === 'critical' || severity === 'high'
      ? 'border-[color:var(--color-block)] text-[color:var(--color-block)]'
      : severity === 'medium'
        ? 'border-[color:var(--color-rewrite)] text-[color:var(--color-rewrite)]'
        : 'border-[color:var(--color-allow)] text-[color:var(--color-allow)]';
  return (
    <Badge variant="outline" className={cn('font-mono', className)}>
      {severity}
    </Badge>
  );
}

function ValidationPanel({ validation }: { validation: PolicyValidationResult | null }) {
  if (validation === null) {
    return (
      <footer className="border-t border-border p-4 text-sm text-muted-foreground">
        Not validated
      </footer>
    );
  }
  if (validation.valid) {
    return (
      <footer className="border-t border-border p-4 text-sm text-[color:var(--color-allow)]">
        Valid
      </footer>
    );
  }
  return (
    <footer className="border-t border-border p-4">
      <p className="text-sm text-[color:var(--color-block)]">Invalid</p>
      <ul className="mt-2 space-y-1">
        {validation.errors.map((issue) => (
          <li key={`${issue.path}:${issue.message}`} className="font-mono text-xs">
            <span className="text-muted-foreground">{issue.path}</span>
            <span className="mx-2 text-muted-foreground">-</span>
            <span>{issue.message}</span>
          </li>
        ))}
      </ul>
    </footer>
  );
}

function TextField({
  label,
  value,
  onChange,
}: {
  label: string;
  value: string;
  onChange: (event: ChangeEvent<HTMLInputElement>) => void;
}) {
  return (
    <div className="space-y-1.5">
      <Label className="font-mono text-xs uppercase tracking-wider text-muted-foreground">
        {label}
      </Label>
      <Input value={value} onChange={onChange} className="font-mono" />
    </div>
  );
}

function SelectField({
  label,
  value,
  options,
  onChange,
}: {
  label: string;
  value: string;
  options: string[];
  onChange: (event: ChangeEvent<HTMLSelectElement>) => void;
}) {
  return (
    <div className="space-y-1.5">
      <Label className="font-mono text-xs uppercase tracking-wider text-muted-foreground">
        {label}
      </Label>
      <select
        value={value}
        onChange={onChange}
        className="h-9 w-full rounded-md border border-input bg-transparent px-3 py-1 font-mono text-sm shadow-sm outline-none focus:ring-1 focus:ring-ring"
      >
        {options.map((option) => (
          <option key={option} value={option}>
            {option}
          </option>
        ))}
      </select>
    </div>
  );
}

function buildYaml(values: BuilderValues): string {
  const lines = [`id: ${yamlString(values.id)}`, `description: ${yamlString(values.description)}`];
  const channels = values.channel.trim() === '' ? [] : [values.channel.trim()];
  const domains = values.domain.trim() === '' ? [] : [values.domain.trim()];
  const agents = values.agent.trim() === '' ? [] : [values.agent.trim()];
  if (channels.length > 0 || domains.length > 0 || agents.length > 0) {
    lines.push('when:');
    if (channels.length > 0) lines.push(`  channels: [${channels.map(yamlString).join(', ')}]`);
    if (domains.length > 0) lines.push(`  domains: [${domains.map(yamlString).join(', ')}]`);
    if (agents.length > 0) lines.push(`  agents: [${agents.map(yamlString).join(', ')}]`);
  }
  lines.push('match:');
  lines.push(`  ${values.matcherType}: ${yamlString(values.matcherValue)}`);
  lines.push(`action: ${values.action}`);
  if (values.rewrite.trim() !== '') lines.push(`rewrite: ${yamlString(values.rewrite)}`);
  lines.push(`severity: ${values.severity}`);
  return `${lines.join('\n')}\n`;
}

function yamlString(value: string): string {
  return JSON.stringify(value.trim());
}

function describeError(error: unknown): string {
  if (error instanceof Error) return error.message;
  return 'unknown error';
}
