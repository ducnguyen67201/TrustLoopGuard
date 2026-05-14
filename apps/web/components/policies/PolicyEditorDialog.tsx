'use client';

import { Loader2, Sparkles } from 'lucide-react';
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
import { Textarea } from '@/components/ui/textarea';
import { getPolicy, upsertPolicy, validatePolicy } from '@/lib/policies';
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

interface PolicyEditorDialogProps {
  open: boolean;
  mode: Mode;
  onOpenChange: (open: boolean) => void;
  onSaved: (policyId: string) => void;
}

type FieldErrors = Partial<Record<keyof PolicyDraft, string>>;
type ValidationState =
  | { kind: 'idle' }
  | { kind: 'ok' }
  | { kind: 'errors'; issues: ReadonlyArray<{ path: string; message: string }> };

export function PolicyEditorDialog({ open, mode, onOpenChange, onSaved }: PolicyEditorDialogProps) {
  const [draft, setDraft] = useState<PolicyDraft>(EMPTY_DRAFT);
  const [fieldErrors, setFieldErrors] = useState<FieldErrors>({});
  const [validation, setValidation] = useState<ValidationState>({ kind: 'idle' });
  const [aiPrompt, setAiPrompt] = useState('');
  const [aiBusy, setAiBusy] = useState(false);
  const [validating, setValidating] = useState(false);
  const [saving, setSaving] = useState(false);
  const [loading, setLoading] = useState(false);

  const yaml = useMemo(() => draftToYaml(draft), [draft]);

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
      const res = await fetch('/api/policies/generate', {
        method: 'POST',
        headers: { 'content-type': 'application/json' },
        body: JSON.stringify({ prompt: aiPrompt }),
      });
      const json = await res.json();
      if (!res.ok) {
        toast.error(typeof json?.error === 'string' ? json.error : 'AI generate failed');
        return;
      }
      const parsed = policyDraftSchema.safeParse(json.draft);
      if (!parsed.success) {
        toast.error('AI returned an invalid policy shape');
        return;
      }
      setDraft(parsed.data);
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
      const saved = await upsertPolicy(draftToYaml(parsed.data));
      toast.success(mode.kind === 'create' ? 'Policy created' : 'Policy updated');
      onSaved(saved.id);
      onOpenChange(false);
    } catch (err) {
      toast.error(describeError(err));
    } finally {
      setSaving(false);
    }
  }

  return (
    <Dialog open={open} onOpenChange={onOpenChange}>
      <DialogContent className="max-w-4xl">
        <DialogHeader>
          <DialogTitle>{mode.kind === 'create' ? 'New policy' : 'Edit policy'}</DialogTitle>
          <DialogDescription>
            Describe the guardrail in plain English to draft with AI, then refine the fields.
          </DialogDescription>
        </DialogHeader>

        <div className="rounded-md border bg-muted/30 p-3">
          <Label htmlFor="ai-prompt" className="font-mono text-[10px] uppercase tracking-wider text-muted-foreground">
            AI draft
          </Label>
          <div className="mt-2 flex gap-2">
            <Input
              id="ai-prompt"
              placeholder="e.g. block messages that promise a full refund"
              value={aiPrompt}
              onChange={(e: ChangeEvent<HTMLInputElement>) => setAiPrompt(e.target.value)}
              disabled={aiBusy || loading}
            />
            <Button type="button" onClick={runAiGenerate} disabled={aiBusy || loading} variant="secondary">
              {aiBusy ? <Loader2 className="animate-spin" /> : <Sparkles />}
              Draft
            </Button>
          </div>
        </div>

        <form onSubmit={onSubmit} className="grid gap-6 md:grid-cols-[minmax(0,1fr)_minmax(0,1fr)]">
          <fieldset disabled={loading || saving} className="space-y-4">
            <Field label="id" htmlFor="id" error={fieldErrors.id}>
              <Input
                id="id"
                value={draft.id}
                onChange={(e) => update('id', e.target.value)}
                placeholder="refund-guarantee"
                className="font-mono"
                readOnly={mode.kind === 'edit'}
              />
            </Field>
            <Field label="description" htmlFor="description" error={fieldErrors.description}>
              <Input
                id="description"
                value={draft.description}
                onChange={(e) => update('description', e.target.value)}
                placeholder="Prevent guaranteed refund promises."
              />
            </Field>
            <div className="grid grid-cols-2 gap-3">
              <Field label="match type" htmlFor="matchType">
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
              <Field label="severity" htmlFor="severity">
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
            <Field label="match value" htmlFor="matchValue" error={fieldErrors.matchValue}>
              <Textarea
                id="matchValue"
                rows={3}
                value={draft.matchValue}
                onChange={(e) => update('matchValue', e.target.value)}
                placeholder={draft.matchType === 'regex' ? '\\bguarantee\\w*\\b' : 'guaranteed refund'}
                className="font-mono"
              />
            </Field>
            <Field label="action" htmlFor="action">
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
              <Field label="safe rewrite" htmlFor="rewrite">
                <Textarea
                  id="rewrite"
                  rows={2}
                  value={draft.rewrite ?? ''}
                  onChange={(e) => update('rewrite', e.target.value)}
                  placeholder="We can review your case and consider a refund."
                />
              </Field>
            ) : null}
          </fieldset>

          <div className="space-y-2">
            <div className="flex items-center justify-between">
              <Label className="font-mono text-[10px] uppercase tracking-wider text-muted-foreground">
                yaml preview
              </Label>
              {validation.kind === 'ok' ? (
                <Badge variant="secondary" className="font-mono uppercase">
                  valid
                </Badge>
              ) : null}
            </div>
            <pre className="max-h-[420px] min-h-[280px] overflow-auto rounded-md border bg-muted p-3 font-mono text-xs whitespace-pre-wrap">
              {yaml}
            </pre>
            {validation.kind === 'errors' ? (
              <ul className="space-y-1 text-xs text-[color:var(--destructive,oklch(0.5_0.2_25))]">
                {validation.issues.map((issue) => (
                  <li key={`${issue.path}-${issue.message}`} className="font-mono">
                    {issue.path || '(root)'}: {issue.message}
                  </li>
                ))}
              </ul>
            ) : null}
          </div>

          <DialogFooter className="md:col-span-2">
            <Button
              type="button"
              variant="outline"
              onClick={runValidate}
              disabled={validating || saving || loading}
            >
              {validating ? <Loader2 className="animate-spin" /> : null}
              Validate
            </Button>
            <Button type="button" variant="ghost" onClick={() => onOpenChange(false)}>
              Cancel
            </Button>
            <Button type="submit" disabled={saving || loading}>
              {saving ? <Loader2 className="animate-spin" /> : null}
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
      <Label
        htmlFor={htmlFor}
        className="font-mono text-[10px] uppercase tracking-wider text-muted-foreground"
      >
        {label}
      </Label>
      {children}
      {error !== undefined ? (
        <p className="font-mono text-xs text-[color:var(--destructive,oklch(0.5_0.2_25))]">
          {error}
        </p>
      ) : null}
    </div>
  );
}

function describeError(err: unknown): string {
  if (err instanceof Error) return err.message;
  return 'unknown error';
}
