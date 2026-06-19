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
import { Skeleton } from '@/components/ui/skeleton';
import { Switch } from '@/components/ui/switch';
import { Textarea } from '@/components/ui/textarea';
import { VerdictLegend } from '@/components/ui/verdict-legend';
import { generatePolicyDraft, getPolicy, upsertPolicy, validatePolicy } from '@/lib/policies';
import {
  draftToYaml,
  EMPTY_DRAFT,
  POLICY_ACTIONS,
  POLICY_MATCH_TYPES,
  POLICY_SEVERITIES,
  policyDraftSchema,
  yamlToDraft,
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

// Friendly, capitalized labels so the dropdowns read like English.
const ACTION_LABEL: Record<PolicyDraft['action'], string> = {
  block: 'Block it',
  rewrite: 'Clean it up (rewrite)',
  escalate: 'Send for review (escalate)',
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
  // In edit mode we load the rule's real YAML. When the builder cannot faithfully
  // round-trip it (e.g. an unsupported match shape), we keep the original YAML as
  // the source of truth so saving never rewrites the rule's logic, and we suppress
  // the plain-language summary that would otherwise misrepresent it.
  const [editSourceYaml, setEditSourceYaml] = useState<string | null>(null);
  const [editRoundTrips, setEditRoundTrips] = useState(true);

  // Guided draft serializes back to YAML for create, and for edit only when the
  // original parsed cleanly. Otherwise show the untouched source as the truth.
  const yaml = useMemo(() => {
    if (mode.kind === 'edit' && !editRoundTrips && editSourceYaml !== null) {
      return editSourceYaml;
    }
    return draftToYaml(draft);
  }, [draft, editRoundTrips, editSourceYaml, mode]);
  const showWorkspaceFields = onSaveDraft !== undefined;

  useEffect(() => {
    if (!open) return;
    setFieldErrors({});
    setValidation({ kind: 'idle' });
    setAiPrompt('');
    if (mode.kind === 'create') {
      setDraft(EMPTY_DRAFT);
      setEditSourceYaml(null);
      setEditRoundTrips(true);
      return;
    }
    let cancelled = false;
    setLoading(true);
    (async () => {
      try {
        const doc = await getPolicy(mode.policyId);
        if (cancelled) return;
        // The rule's real match/action live encoded in source_yaml, not as
        // structured fields on the document. Parse them so the form reflects the
        // actual rule instead of hardcoded defaults.
        const parsed = yamlToDraft(doc.source_yaml);
        setEditSourceYaml(doc.source_yaml);
        if (parsed.ok) {
          // Builder can faithfully round-trip this rule; seed the guided form
          // from the real values (real match, real action, real severity).
          setEditRoundTrips(true);
          setDraft({ ...parsed.draft, id: doc.id });
        } else {
          // Cannot reconstruct the structured match safely. Keep description /
          // severity from the document so those stay readable, but mark the rule
          // as not round-trippable so save preserves the original YAML verbatim
          // and the summary does not invent a match it cannot read. The action
          // here is a neutral placeholder only — it is never used for saving in
          // this branch (the original source_yaml is the source of truth).
          setEditRoundTrips(false);
          setDraft({
            id: doc.id,
            description: doc.description ?? '',
            matchType: 'literal',
            matchValue: '',
            action: 'block',
            severity: doc.severity,
          });
        }
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
      toast.error('Tell us what to catch in a few words first.');
      return;
    }
    setAiBusy(true);
    try {
      const nextDraft = await generatePolicyDraft(aiPrompt);
      setDraft(nextDraft);
      setValidation({ kind: 'idle' });
      setFieldErrors({});
      toast.success('Draft ready — look it over, then save');
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
        toast.success('No problems found');
      } else {
        setValidation({
          kind: 'errors',
          issues: result.errors.map((e) => ({ path: e.path, message: e.message })),
        });
        toast.error('A few things need fixing');
      }
    } catch (err) {
      toast.error(describeError(err));
    } finally {
      setValidating(false);
    }
  }

  async function onSubmit(event: FormEvent<HTMLFormElement>) {
    event.preventDefault();
    // Editing a rule the builder cannot round-trip: the guided draft has no
    // faithful match, so serializing it would silently overwrite the rule's real
    // match/logic. Save the untouched source_yaml — the rule is left exactly as it
    // was. (Editing such rules requires the raw-YAML editor on the policies page.)
    if (mode.kind === 'edit' && !editRoundTrips && editSourceYaml !== null) {
      setSaving(true);
      try {
        const savedId = (await upsertPolicy(editSourceYaml)).id;
        toast.success('Rule saved');
        onSaved(savedId);
        onOpenChange(false);
      } catch (err) {
        toast.error(describeError(err));
      } finally {
        setSaving(false);
      }
      return;
    }
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
      toast.success(mode.kind === 'create' ? 'Rule created' : 'Rule saved');
      onSaved(savedId);
      onOpenChange(false);
    } catch (err) {
      toast.error(describeError(err));
    } finally {
      setSaving(false);
    }
  }

  const verdict = actionVariant(draft.action);
  // True when editing a rule whose structured match cannot be read back from its
  // YAML. The guided match fields and plain-language summary cannot honestly
  // describe such a rule, so we hide them and point to the raw definition.
  const matchUnknown = mode.kind === 'edit' && !editRoundTrips;

  return (
    <Dialog open={open} onOpenChange={onOpenChange}>
      <DialogContent className="max-w-4xl">
        <DialogHeader>
          <DialogTitle>
            {mode.kind === 'create' ? 'New protection rule' : 'Edit protection rule'}
          </DialogTitle>
          <DialogDescription>
            Describe what you want to catch in plain English and let AI draft it, or fill in the
            fields yourself. You decide what happens on a match.
          </DialogDescription>
        </DialogHeader>

        <div className="rounded-lg border bg-muted/40 p-3">
          <Label
            htmlFor="ai-prompt"
            className="flex items-center gap-1.5 text-xs font-medium text-muted-foreground"
          >
            <Sparkles className="size-3.5" aria-hidden />
            Describe it and let AI fill in the form
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
              Draft for me
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
                <Field
                  label="Description"
                  htmlFor="description"
                  error={fieldErrors.description}
                  hint="A plain-language name for this rule. Shows up in your rules list."
                >
                  <Input
                    id="description"
                    value={draft.description}
                    onChange={(e) => update('description', e.target.value)}
                    placeholder="Block guaranteed-refund promises"
                  />
                </Field>
                <Field
                  label="Rule ID"
                  htmlFor="id"
                  error={fieldErrors.id}
                  hint="A short, lowercase id the engine uses (e.g. no-pii). Not the friendly name."
                >
                  <Input
                    id="id"
                    value={draft.id}
                    onChange={(e) => update('id', e.target.value)}
                    placeholder="refund-guarantee"
                    className="font-mono"
                    readOnly={mode.kind === 'edit'}
                  />
                </Field>
                {showWorkspaceFields ? (
                  <div className="grid grid-cols-2 gap-3">
                    <Field
                      label="Applies to"
                      htmlFor="agent"
                      hint="Which AI assistant this rule checks. “Global” means all of them."
                    >
                      <Select
                        value={selectedAgentId}
                        onValueChange={(value) => onSelectedAgentIdChange?.(value)}
                        disabled={mode.kind === 'edit'}
                      >
                        <SelectTrigger id="agent">
                          <SelectValue />
                        </SelectTrigger>
                        <SelectContent>
                          <SelectItem value="global">All assistants (global)</SelectItem>
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
                        Turn on now
                      </Label>
                      <div className="flex h-9 items-center justify-between gap-3 rounded-md border px-3">
                        <span className="text-sm text-muted-foreground">
                          {enabled ? 'On — checks traffic' : 'Off — saved as draft'}
                        </span>
                        <Switch
                          id="enabled"
                          checked={enabled}
                          disabled={mode.kind === 'edit'}
                          aria-label="Turn this rule on as soon as it is saved"
                          {...(onEnabledChange ? { onCheckedChange: onEnabledChange } : {})}
                        />
                      </div>
                    </div>
                  </div>
                ) : null}
                {matchUnknown ? (
                  <p
                    className="rounded-md border border-dashed bg-muted/40 p-3 text-xs leading-relaxed text-muted-foreground"
                    role="note"
                  >
                    This rule&apos;s match was written in a form the guided editor can&apos;t show.
                    Its match is shown unchanged under <span className="font-medium">Advanced</span> and
                    is preserved exactly when you save. Edit the description, severity, or action here,
                    or open the raw definition to change the match.
                  </p>
                ) : null}
                <div className="grid grid-cols-2 gap-3">
                  {matchUnknown ? null : (
                    <Field
                      label="What to look for"
                      htmlFor="matchType"
                      hint="Match the exact words, or a flexible pattern for advanced cases."
                    >
                      <Select
                        value={draft.matchType}
                        onValueChange={(v) => update('matchType', v as PolicyDraft['matchType'])}
                      >
                        <SelectTrigger id="matchType">
                          <SelectValue />
                        </SelectTrigger>
                        <SelectContent>
                          {POLICY_MATCH_TYPES.map((t) => (
                            <SelectItem key={t} value={t}>
                              {MATCH_TYPE_LABEL[t]}
                            </SelectItem>
                          ))}
                        </SelectContent>
                      </Select>
                    </Field>
                  )}
                  <Field
                    label="Severity"
                    htmlFor="severity"
                    hint={<InfoHint term="severity" />}
                  >
                    <Select
                      value={draft.severity}
                      onValueChange={(v) => update('severity', v as PolicyDraft['severity'])}
                    >
                      <SelectTrigger id="severity">
                        <SelectValue />
                      </SelectTrigger>
                      <SelectContent>
                        {POLICY_SEVERITIES.map((s) => (
                          <SelectItem key={s} value={s}>
                            {SEVERITY_LABEL[s]}
                          </SelectItem>
                        ))}
                      </SelectContent>
                    </Select>
                  </Field>
                </div>
                {matchUnknown ? null : (
                  <Field
                    label={draft.matchType === 'regex' ? 'Pattern to match' : 'Words to match'}
                    htmlFor="matchValue"
                    error={fieldErrors.matchValue}
                    hint={
                      draft.matchType === 'regex'
                        ? 'A regular expression. Leave this to a teammate who writes regex if unsure.'
                        : 'The phrase to catch. Capitalization is ignored.'
                    }
                  >
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
                )}
                <Field
                  label="When it matches, the guardrail will…"
                  htmlFor="action"
                  hint={<InfoHint term="verdict" />}
                >
                  <Select
                    value={draft.action}
                    onValueChange={(v) => update('action', v as PolicyDraft['action'])}
                  >
                    <SelectTrigger id="action">
                      <SelectValue />
                    </SelectTrigger>
                    <SelectContent>
                      {POLICY_ACTIONS.map((a) => (
                        <SelectItem key={a} value={a}>
                          {ACTION_LABEL[a]}
                        </SelectItem>
                      ))}
                    </SelectContent>
                  </Select>
                </Field>
                {draft.action === 'rewrite' ? (
                  <Field
                    label="Replace it with"
                    htmlFor="rewrite"
                    hint="The safe wording sent through instead of the flagged text."
                  >
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

          <div className="flex min-w-0 flex-col gap-3">
            <div className="rounded-lg border bg-muted/40 p-4">
              <div className="flex items-center justify-between gap-2">
                <span className="text-xs font-medium text-muted-foreground">
                  Here&apos;s what this rule will do
                </span>
                {validation.kind === 'ok' ? (
                  <Badge variant="allow" className="gap-1">
                    <CheckCircle2 className="size-3" aria-hidden />
                    Looks good
                  </Badge>
                ) : null}
              </div>
              {matchUnknown ? (
                <p className="mt-3 text-sm leading-relaxed text-foreground [text-wrap:pretty]">
                  When this rule matches, the guardrail will{' '}
                  <Badge variant={verdict} className="align-middle">
                    {ACTION_LABEL[draft.action]}
                  </Badge>
                  . See <span className="font-medium">Advanced</span> below for exactly what it
                  matches.
                </p>
              ) : (
                <p className="mt-3 text-sm leading-relaxed text-foreground [text-wrap:pretty]">
                  When a request {matchSummary(draft)},{' '}
                  <Badge variant={verdict} className="align-middle">
                    {ACTION_LABEL[draft.action]}
                  </Badge>
                  .
                </p>
              )}
              <div className="mt-4 border-t pt-3">
                <p className="mb-3 text-xs font-medium text-muted-foreground">
                  What each choice means
                </p>
                <VerdictLegend
                  verdicts={['rewrite', 'escalate', 'block']}
                  className="sm:grid-cols-1 xl:grid-cols-1"
                />
              </div>
            </div>

            {validation.kind === 'errors' ? (
              <ul className="space-y-1 rounded-md border border-destructive/40 bg-destructive/5 p-2.5 text-xs text-destructive">
                {validation.issues.map((issue) => (
                  <li key={`${issue.path}-${issue.message}`} className="font-mono">
                    {issue.path || '(root)'}: {issue.message}
                  </li>
                ))}
              </ul>
            ) : null}

            <details className="rounded-lg border bg-card text-sm">
              <summary className="flex cursor-pointer items-center gap-1.5 px-3 py-2 text-xs font-medium text-muted-foreground select-none">
                <FileCode2 className="size-3.5" aria-hidden />
                Advanced: view the raw rule definition
              </summary>
              <pre className="max-h-[280px] overflow-auto whitespace-pre-wrap border-t bg-muted/30 p-3 font-mono text-xs leading-relaxed">
                {yaml}
              </pre>
            </details>
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
                Check for problems
              </Button>
            ) : null}
            <Button type="button" variant="ghost" onClick={() => onOpenChange(false)}>
              Cancel
            </Button>
            <Button type="submit" disabled={saving || loading}>
              {saving ? <Loader2 className="animate-spin" aria-hidden /> : null}
              {mode.kind === 'create' ? 'Create rule' : 'Save changes'}
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
  /** Plain-language guidance under the field. A string renders as helper text;
   * a node (e.g. an InfoHint) renders inline next to the label. */
  hint?: ReactNode;
  children: ReactNode;
}

function Field({ label, htmlFor, error, hint, children }: FieldProps) {
  const inlineHint = hint !== undefined && typeof hint !== 'string';
  return (
    <div className="space-y-1.5">
      <Label htmlFor={htmlFor} className="flex items-center gap-1 text-xs font-medium text-foreground">
        {label}
        {inlineHint ? hint : null}
      </Label>
      {children}
      {error !== undefined ? <p className="text-xs text-destructive">{error}</p> : null}
      {typeof hint === 'string' ? (
        <p className="text-xs text-muted-foreground">{hint}</p>
      ) : null}
    </div>
  );
}

/** A short, plain-language phrase describing what content the rule catches, used
 * in the "what this rule will do" summary. */
function matchSummary(draft: PolicyDraft): string {
  const value = draft.matchValue.trim();
  if (value === '') {
    return draft.matchType === 'regex' ? 'matches your pattern' : 'contains your chosen words';
  }
  if (draft.matchType === 'regex') return `matches the pattern ${value}`;
  return `contains “${value}”`;
}

function describeError(err: unknown): string {
  if (err instanceof Error) return err.message;
  return 'unknown error';
}
