'use client';

import { CheckCircle2, FileCode2, Loader2, Sparkles } from 'lucide-react';
import {
  useEffect,
  useMemo,
  useState,
  type ChangeEvent,
  type FormEvent,
  type ReactNode,
} from 'react';
import { toast } from 'sonner';

import { Badge } from '@/components/ui/badge';
import { Button } from '@/components/ui/button';
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from '@/components/ui/dialog';
import { Input } from '@/components/ui/input';
import { Label } from '@/components/ui/label';
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from '@/components/ui/select';
import { Skeleton } from '@/components/ui/skeleton';
import { Switch } from '@/components/ui/switch';
import { Textarea } from '@/components/ui/textarea';
import { generatePolicyDraft, getPolicy, upsertPolicy, validatePolicy } from '@/lib/policies';
import {
  draftToYaml,
  EMPTY_DRAFT,
  POLICY_ACTIONS,
  POLICY_MATCH_TYPES,
  POLICY_SEVERITIES,
  policyDraftSchema,
  type PolicyDraft,
} from '@/lib/policy-draft';

type Mode = { kind: 'create' } | { kind: 'edit'; policyId: string };
type AgentOption = { id: string; name: string };
type SavePolicyOptions = { agentId: string | null; enabled: boolean };

interface PolicyEditorDialogProps {
  open: boolean;
  mode: Mode;
  onOpenChange: (open: boolean) => void;
  onSaved: (policyId: string) => void;
  onSaveDraft?: (draft: PolicyDraft, options: SavePolicyOptions) => Promise<string>;
  showValidate?: boolean;
  agents?: AgentOption[];
  selectedAgentId?: string;
  onSelectedAgentIdChange?: (agentId: string) => void;
  enabled?: boolean;
  onEnabledChange?: (enabled: boolean) => void;
}

type FieldErrors = Partial<Record<keyof PolicyDraft, string>>;
type ValidationState =
  | { kind: 'idle' }
  | { kind: 'ok' }
  | { kind: 'errors'; issues: ReadonlyArray<{ path: string; message: string }> };
type VerdictVariant = 'allow' | 'rewrite' | 'block' | 'escalate';

function actionVariant(action: PolicyDraft['action']): VerdictVariant {
  if (action === 'rewrite') return 'rewrite';
  if (action === 'escalate') return 'escalate';
  return 'block';
}

export function PolicyEditorDialog({
  open,
  mode,
  onOpenChange,
  onSaved,
  onSaveDraft,
  showValidate = true,
  agents = [],
  selectedAgentId = 'global',
  onSelectedAgentIdChange,
  enabled = true,
  onEnabledChange,
}: PolicyEditorDialogProps) {
  const [draft, setDraft] = useState<PolicyDraft>(EMPTY_DRAFT);
  const [fieldErrors, setFieldErrors] = useState<FieldErrors>({});
  const [validation, setValidation] = useState<ValidationState>({ kind: 'idle' });
  const [aiPrompt, setAiPrompt] = useState('');
  const [aiBusy, setAiBusy] = useState(false);
  const [validating, setValidating] = useState(false);
  const [saving, setSaving] = useState(false);
  const [loading, setLoading] = useState(false);

  const yaml = useMemo(() => draftToYaml(draft), [draft]);
  const showWorkspaceFields = onSaveDraft !== undefined;

  useEffect(() => {
    if (!open) return;
    setFieldErrors({});
    setValidation({ kind: 'idle' });
    setAiPrompt('');
    if (mode.kind === 'create') {
      setDraft(EMPTY_DRAFT);
      return;
    }
    let cancelled = false;
    setLoading(true);
    (async () => {
      try {
        const doc = await getPolicy(mode.policyId);
        if (cancelled) return;
        setDraft({
          id: doc.id,
          description: doc.description ?? '',
          matchType: 'literal',
          matchValue: '',
          action: 'block',
          severity: doc.severity,
        });
      } catch (err) {
        if (!cancelled) toast.error(describeError(err));
      } finally {
        if (!cancelled) setLoading(false);
      }
    })();
    return () => {
      cancelled = true;
    };
  }, [open, mode]);

  function update<K extends keyof PolicyDraft>(key: K, value: PolicyDraft[K]) {
    setDraft((prev) => ({ ...prev, [key]: value }));
    setValidation({ kind: 'idle' });
    if (fieldErrors[key]) {
      setFieldErrors((prev) => {
        const next = { ...prev };
        delete next[key];
        return next;
      });
    }
  }

  async function runAiGenerate() {
    if (aiPrompt.trim().length < 3) {
      toast.error('Describe the policy in a few words first.');
      return;
    }
    setAiBusy(true);
    try {
      const nextDraft = await generatePolicyDraft(aiPrompt);
      setDraft(nextDraft);
      setValidation({ kind: 'idle' });
      setFieldErrors({});
      toast.success('Drafted with AI — review and save');
    } catch (err) {
      toast.error(describeError(err));
    } finally {
      setAiBusy(false);
    }
  }

  async function runValidate() {
    setValidating(true);
    try {
      const result = await validatePolicy(yaml);
      if (result.valid) {
        setValidation({ kind: 'ok' });
        toast.success('Policy is valid');
      } else {
        setValidation({
          kind: 'errors',
          issues: result.errors.map((e) => ({ path: e.path, message: e.message })),
        });
        toast.error('Policy has issues');
      }
    } catch (err) {
      toast.error(describeError(err));
    } finally {
      setValidating(false);
    }
  }

  async function onSubmit(event: FormEvent<HTMLFormElement>) {
    event.preventDefault();
    const parsed = policyDraftSchema.safeParse(draft);
    if (!parsed.success) {
      const next: FieldErrors = {};
      for (const issue of parsed.error.issues) {
        const head = issue.path[0];
        if (typeof head === 'string' && head in EMPTY_DRAFT) {
          next[head as keyof PolicyDraft] = issue.message;
        }
      }
      setFieldErrors(next);
      return;
    }
    setSaving(true);
    try {
      const savedId =
        onSaveDraft !== undefined
          ? await onSaveDraft(parsed.data, {
              agentId: selectedAgentId === 'global' ? null : selectedAgentId,
              enabled,
            })
          : (await upsertPolicy(draftToYaml(parsed.data))).id;
      toast.success(mode.kind === 'create' ? 'Policy created' : 'Policy updated');
      onSaved(savedId);
      onOpenChange(false);
    } catch (err) {
      toast.error(describeError(err));
    } finally {
      setSaving(false);
    }
  }

  const verdict = actionVariant(draft.action);

  return (
    <Dialog open={open} onOpenChange={onOpenChange}>
      <DialogContent className="max-w-4xl">
        <DialogHeader>
          <DialogTitle>{mode.kind === 'create' ? 'New policy' : 'Edit policy'}</DialogTitle>
          <DialogDescription>
            Describe the guardrail in plain English to draft with AI, then refine the fields.
          </DialogDescription>
        </DialogHeader>

        <div className="rounded-lg border bg-muted/40 p-3">
          <Label
            htmlFor="ai-prompt"
            className="flex items-center gap-1.5 text-xs font-medium text-muted-foreground"
          >
            <Sparkles className="size-3.5" aria-hidden />
            Draft with AI
          </Label>
          <div className="mt-2 flex gap-2">
            <Input
              id="ai-prompt"
              placeholder="e.g. block messages that promise a full refund"
              value={aiPrompt}
              onChange={(e: ChangeEvent<HTMLInputElement>) => setAiPrompt(e.target.value)}
              disabled={aiBusy || loading}
            />
            <Button
              type="button"
              onClick={runAiGenerate}
              disabled={aiBusy || loading}
              variant="secondary"
              className="shrink-0"
            >
              {aiBusy ? <Loader2 className="animate-spin" aria-hidden /> : <Sparkles aria-hidden />}
              Draft
            </Button>
          </div>
        </div>

        <form onSubmit={onSubmit} className="grid gap-6 md:grid-cols-[minmax(0,1fr)_minmax(0,1fr)]">
          <fieldset disabled={loading || saving} className="min-w-0 space-y-4">
            {loading ? (
              <div className="space-y-4" aria-busy>
                <Skeleton className="h-9 w-full" />
                <Skeleton className="h-9 w-full" />
                <Skeleton className="h-9 w-2/3" />
                <Skeleton className="h-24 w-full" />
              </div>
            ) : (
              <>
                <Field label="Policy ID" htmlFor="id" error={fieldErrors.id}>
                  <Input
                    id="id"
                    value={draft.id}
                    onChange={(e) => update('id', e.target.value)}
                    placeholder="refund-guarantee"
                    className="font-mono"
                    readOnly={mode.kind === 'edit'}
                  />
                </Field>
                <Field label="Description" htmlFor="description" error={fieldErrors.description}>
                  <Input
                    id="description"
                    value={draft.description}
                    onChange={(e) => update('description', e.target.value)}
                    placeholder="Prevent guaranteed refund promises."
                  />
                </Field>
                {showWorkspaceFields ? (
                  <div className="grid grid-cols-2 gap-3">
                    <Field label="Agent" htmlFor="agent">
                      <Select
                        value={selectedAgentId}
                        onValueChange={(value) => onSelectedAgentIdChange?.(value)}
                        disabled={mode.kind === 'edit'}
                      >
                        <SelectTrigger id="agent">
                          <SelectValue />
                        </SelectTrigger>
                        <SelectContent>
                          <SelectItem value="global">Global</SelectItem>
                          {agents.map((agent) => (
                            <SelectItem key={agent.id} value={agent.id}>
                              {agent.name}
                            </SelectItem>
                          ))}
                        </SelectContent>
                      </Select>
                    </Field>
                    <div className="space-y-1.5">
                      <Label htmlFor="enabled" className="text-xs font-medium text-foreground">
                        Enabled
                      </Label>
                      <div className="flex h-9 items-center justify-between gap-3 rounded-md border px-3">
                        <span className="text-sm text-muted-foreground">
                          {enabled ? 'Yes' : 'No'}
                        </span>
                        <Switch
                          id="enabled"
                          checked={enabled}
                          disabled={mode.kind === 'edit'}
                          {...(onEnabledChange ? { onCheckedChange: onEnabledChange } : {})}
                        />
                      </div>
                    </div>
                  </div>
                ) : null}
                <div className="grid grid-cols-2 gap-3">
                  <Field label="Match type" htmlFor="matchType">
                    <Select
                      value={draft.matchType}
                      onValueChange={(v) => update('matchType', v as PolicyDraft['matchType'])}
                    >
                      <SelectTrigger id="matchType" className="font-mono">
                        <SelectValue />
                      </SelectTrigger>
                      <SelectContent>
                        {POLICY_MATCH_TYPES.map((t) => (
                          <SelectItem key={t} value={t} className="font-mono">
                            {t}
                          </SelectItem>
                        ))}
                      </SelectContent>
                    </Select>
                  </Field>
                  <Field label="Severity" htmlFor="severity">
                    <Select
                      value={draft.severity}
                      onValueChange={(v) => update('severity', v as PolicyDraft['severity'])}
                    >
                      <SelectTrigger id="severity" className="font-mono">
                        <SelectValue />
                      </SelectTrigger>
                      <SelectContent>
                        {POLICY_SEVERITIES.map((s) => (
                          <SelectItem key={s} value={s} className="font-mono">
                            {s}
                          </SelectItem>
                        ))}
                      </SelectContent>
                    </Select>
                  </Field>
                </div>
                <Field label="Match value" htmlFor="matchValue" error={fieldErrors.matchValue}>
                  <Textarea
                    id="matchValue"
                    rows={3}
                    value={draft.matchValue}
                    onChange={(e) => update('matchValue', e.target.value)}
                    placeholder={
                      draft.matchType === 'regex' ? '\\bguarantee\\w*\\b' : 'guaranteed refund'
                    }
                    className="font-mono"
                  />
                </Field>
                <Field label="Action" htmlFor="action">
                  <Select
                    value={draft.action}
                    onValueChange={(v) => update('action', v as PolicyDraft['action'])}
                  >
                    <SelectTrigger id="action" className="font-mono">
                      <SelectValue />
                    </SelectTrigger>
                    <SelectContent>
                      {POLICY_ACTIONS.map((a) => (
                        <SelectItem key={a} value={a} className="font-mono">
                          {a}
                        </SelectItem>
                      ))}
                    </SelectContent>
                  </Select>
                </Field>
                {draft.action === 'rewrite' ? (
                  <Field label="Safe rewrite" htmlFor="rewrite">
                    <Textarea
                      id="rewrite"
                      rows={2}
                      value={draft.rewrite ?? ''}
                      onChange={(e) => update('rewrite', e.target.value)}
                      placeholder="We can review your case and consider a refund."
                    />
                  </Field>
                ) : null}
              </>
            )}
          </fieldset>

          <div className="flex min-w-0 flex-col gap-2">
            <div className="flex items-center justify-between">
              <Label className="flex items-center gap-1.5 text-xs font-medium text-muted-foreground">
                <FileCode2 className="size-3.5" aria-hidden />
                YAML preview
              </Label>
              <div className="flex items-center gap-1.5">
                <Badge variant={verdict} className="font-mono uppercase">
                  {draft.action}
                </Badge>
                {validation.kind === 'ok' ? (
                  <Badge variant="allow" className="gap-1 font-mono uppercase">
                    <CheckCircle2 className="size-3" aria-hidden />
                    valid
                  </Badge>
                ) : null}
              </div>
            </div>
            <pre className="max-h-[420px] min-h-[280px] overflow-auto whitespace-pre-wrap rounded-lg border bg-muted/30 p-3 font-mono text-xs leading-relaxed">
              {yaml}
            </pre>
            {validation.kind === 'errors' ? (
              <ul className="space-y-1 rounded-md border border-destructive/40 bg-destructive/5 p-2.5 text-xs text-destructive">
                {validation.issues.map((issue) => (
                  <li key={`${issue.path}-${issue.message}`} className="font-mono">
                    {issue.path || '(root)'}: {issue.message}
                  </li>
                ))}
              </ul>
            ) : null}
          </div>

          <DialogFooter className="md:col-span-2">
            {showValidate ? (
              <Button
                type="button"
                variant="outline"
                onClick={runValidate}
                disabled={validating || saving || loading}
              >
                {validating ? <Loader2 className="animate-spin" aria-hidden /> : null}
                Validate
              </Button>
            ) : null}
            <Button type="button" variant="ghost" onClick={() => onOpenChange(false)}>
              Cancel
            </Button>
            <Button type="submit" disabled={saving || loading}>
              {saving ? <Loader2 className="animate-spin" aria-hidden /> : null}
              {mode.kind === 'create' ? 'Create' : 'Save'}
            </Button>
          </DialogFooter>
        </form>
      </DialogContent>
    </Dialog>
  );
}

interface FieldProps {
  label: string;
  htmlFor: string;
  error?: string | undefined;
  children: ReactNode;
}

function Field({ label, htmlFor, error, children }: FieldProps) {
  return (
    <div className="space-y-1.5">
      <Label htmlFor={htmlFor} className="text-xs font-medium text-foreground">
        {label}
      </Label>
      {children}
      {error !== undefined ? <p className="text-xs text-destructive">{error}</p> : null}
    </div>
  );
}

function describeError(err: unknown): string {
  if (err instanceof Error) return err.message;
  return 'unknown error';
}
