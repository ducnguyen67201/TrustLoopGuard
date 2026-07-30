'use client';

import type { Dispatch, ReactNode, SetStateAction } from 'react';
import { useEffect, useMemo, useState } from 'react';
import { toast } from 'sonner';

import { Button } from '@/components/ui/button';
import { Checkbox } from '@/components/ui/checkbox';
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
import type { FamilyPolicyRow } from '@/lib/server/dashboard-data';

import { safeError } from './financial-utils';

type FinancialControlForm = {
  id: string;
  description: string;
  meter: 'actions' | 'llm_usage';
  agent: string;
  actionKind: 'refund' | 'payment' | 'payout';
  operation: string;
  currency: string;
  rail: 'payment_http' | 'x402' | 'card' | 'ach' | 'wire' | 'internal' | 'other';
  perAction: string;
  approvalAbove: string;
  daily: string;
  weekly: string;
  monthly: string;
  grantRequired: boolean;
  onBreach: 'deny' | 'require_approval';
  missingEvidenceEffect: 'deny' | 'require_approval';
  failedPreconditionEffect: 'deny' | 'require_approval';
  requiredPreconditions: string[];
};

const ACTION_KINDS: ReadonlyArray<FinancialControlForm['actionKind']> = [
  'refund',
  'payment',
  'payout',
];
const RAILS: ReadonlyArray<FinancialControlForm['rail']> = [
  'payment_http',
  'x402',
  'card',
  'ach',
  'wire',
  'internal',
  'other',
];
const EFFECTS: ReadonlyArray<'deny' | 'require_approval'> = ['deny', 'require_approval'];

const REFUND_PRECONDITIONS = [
  { id: 'order_exists', label: 'Order exists' },
  { id: 'payment_captured', label: 'Payment captured' },
  { id: 'refund_window_open', label: 'Refund window open' },
  { id: 'amount_lte_refundable_balance', label: 'Amount within refundable balance' },
  { id: 'destination_is_original_payment_method', label: 'Original payment method' },
  { id: 'no_duplicate_refund', label: 'No duplicate refund' },
] as const;

const DEFAULT_FORM: FinancialControlForm = {
  id: 'x402-agentic-payment-grant-required',
  description: 'x402 payment controls for commerce agents',
  meter: 'actions',
  agent: 'spid:commerce-agent',
  actionKind: 'payment',
  operation: 'x402_read_paid_resource',
  currency: 'USD',
  rail: 'x402',
  perAction: '5',
  approvalAbove: '',
  daily: '50',
  weekly: '',
  monthly: '5000',
  grantRequired: true,
  onBreach: 'deny',
  missingEvidenceEffect: 'require_approval',
  failedPreconditionEffect: 'deny',
  requiredPreconditions: [],
};

const DEFAULT_LLM_BUDGET_FORM: FinancialControlForm = {
  ...DEFAULT_FORM,
  id: 'llm-weekly-budget',
  description: 'Weekly LLM spend cap per principal',
  meter: 'llm_usage',
  agent: '',
  perAction: '',
  approvalAbove: '',
  daily: '',
  weekly: '50',
  monthly: '',
  grantRequired: false,
  requiredPreconditions: [],
};

export function FinancialPolicyCreateDialog({
  open,
  onOpenChange,
  contextQuery,
  initialPolicy,
  existingPolicyIds = [],
  onCreated,
  meter,
}: {
  open: boolean;
  onOpenChange: (open: boolean) => void;
  contextQuery: string;
  initialPolicy?: FamilyPolicyRow | undefined;
  existingPolicyIds?: string[];
  onCreated?: (policy: FamilyPolicyRow) => void;
  meter?: FinancialControlForm['meter'];
}) {
  const [saving, setSaving] = useState(false);
  const [form, setForm] = useState<FinancialControlForm>(DEFAULT_FORM);
  const policyIds = useMemo(() => new Set(existingPolicyIds), [existingPolicyIds]);
  const editing = initialPolicy !== undefined;

  useEffect(() => {
    if (open) {
      const initial = initialPolicy ? formFromPolicy(initialPolicy) : DEFAULT_FORM;
      setForm(meter ? formForMeter(initial, meter) : initial);
    }
  }, [initialPolicy, meter, open]);

  async function createControl() {
    let payload: ReturnType<typeof formPayload>;
    try {
      payload = formPayload(form);
    } catch (error) {
      toast.error(error instanceof Error ? error.message : 'Control is invalid');
      return;
    }
    setSaving(true);
    try {
      const response = await fetch(`/api/financial/policies${contextQuery}`, {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify(payload),
      });
      const text = await response.text();
      if (!response.ok) {
        throw new Error(safeError(text) ?? 'Unable to create financial control');
      }
      const created = JSON.parse(text) as FamilyPolicyRow;
      onCreated?.(created);
      onOpenChange(false);
      toast.success(editing ? 'Financial policy saved' : 'Financial policy created');
    } catch (error) {
      toast.error(error instanceof Error ? error.message : 'Unable to create financial policy');
    } finally {
      setSaving(false);
    }
  }

  return (
    <Dialog open={open} onOpenChange={onOpenChange}>
      <DialogContent className="max-w-3xl">
        <DialogHeader>
          <DialogTitle>{editing ? 'Edit financial policy' : 'Create financial policy'}</DialogTitle>
          <DialogDescription>
            {form.meter === 'llm_usage'
              ? 'Cap gateway LLM spend per principal. Requests with max_tokens get strict preflight enforcement; unbounded requests are allowed below the cap, settled to actual usage, and may overshoot once before future calls stop. Trusted model pricing is required.'
              : 'Define reusable-grant requirements, caps, evidence checks, and authorization effects TrustLoopGuard evaluates before execution.'}
          </DialogDescription>
        </DialogHeader>
        <div className="grid max-h-[70vh] gap-4 overflow-y-auto pr-1">
          <Field
            label="Applies to"
            hint="Choose whether this policy governs money-moving actions or Gateway LLM spend."
          >
            <Select
              value={form.meter}
              disabled={meter !== undefined}
              onValueChange={(value) =>
                setForm((prev) => formForMeter(prev, value as FinancialControlForm['meter']))
              }
            >
              <SelectTrigger className="w-full">
                <SelectValue />
              </SelectTrigger>
              <SelectContent>
                <SelectItem value="actions">Financial actions</SelectItem>
                <SelectItem value="llm_usage">LLM usage (gateway)</SelectItem>
              </SelectContent>
            </Select>
          </Field>
          <div className="grid gap-3 md:grid-cols-2">
            <Field
              label="Control id"
              hint="Use a stable lowercase identifier that will appear in policy lists, logs, and API responses."
            >
              <Input
                value={form.id}
                disabled={editing}
                onChange={(event) => setFormValue(setForm, 'id', event.target.value)}
              />
            </Field>
            <Field
              label={form.meter === 'llm_usage' ? 'Principal (optional)' : 'Agent'}
              hint={
                form.meter === 'llm_usage'
                  ? 'Limit this budget to one runtime principal. Leave blank to meter every principal separately.'
                  : 'Only actions from this agent id match. Use the same id your SDK sends.'
              }
            >
              <Input
                value={form.agent}
                onChange={(event) => setFormValue(setForm, 'agent', event.target.value)}
              />
            </Field>
            <Field
              label="Description"
              hint="Explain what this control protects so teammates can recognize it later."
            >
              <Input
                value={form.description}
                onChange={(event) => setFormValue(setForm, 'description', event.target.value)}
              />
            </Field>
            {form.meter === 'actions' ? (
              <>
                <Field
                  label="Operation"
                  hint="Optional operation name sent by the integration, such as issue_refund. It must match exactly."
                >
                  <Input
                    value={form.operation}
                    onChange={(event) => setFormValue(setForm, 'operation', event.target.value)}
                  />
                </Field>
                <Field
                  label="Currency"
                  hint="Use a three-letter currency code, such as USD. The amount fields use this currency."
                >
                  <Input
                    value={form.currency}
                    onChange={(event) => setFormValue(setForm, 'currency', event.target.value)}
                  />
                </Field>
                <Field
                  label="Action kind"
                  hint="Select the typed action this policy evaluates: refund, payment, or payout."
                >
                  <Select
                    value={form.actionKind}
                    onValueChange={(value) =>
                      setFormValue(
                        setForm,
                        'actionKind',
                        value as FinancialControlForm['actionKind'],
                      )
                    }
                  >
                    <SelectTrigger className="w-full">
                      <SelectValue />
                    </SelectTrigger>
                    <SelectContent>
                      <SelectItem value="refund">Refund</SelectItem>
                      <SelectItem value="payment">Payment</SelectItem>
                      <SelectItem value="payout">Payout</SelectItem>
                    </SelectContent>
                  </Select>
                </Field>
                <Field
                  label="Rail"
                  hint="Select how the money moves. The action must report the same rail to match."
                >
                  <Select
                    value={form.rail}
                    onValueChange={(value) =>
                      setFormValue(setForm, 'rail', value as FinancialControlForm['rail'])
                    }
                  >
                    <SelectTrigger className="w-full">
                      <SelectValue />
                    </SelectTrigger>
                    <SelectContent>
                      <SelectItem value="payment_http">Payment HTTP</SelectItem>
                      <SelectItem value="x402">x402</SelectItem>
                      <SelectItem value="card">Card</SelectItem>
                      <SelectItem value="ach">ACH</SelectItem>
                      <SelectItem value="wire">Wire</SelectItem>
                      <SelectItem value="internal">Internal</SelectItem>
                      <SelectItem value="other">Other</SelectItem>
                    </SelectContent>
                  </Select>
                </Field>
              </>
            ) : null}
          </div>
          <div className="grid gap-3 md:grid-cols-4">
            {form.meter === 'actions' ? (
              <>
                <MoneyField
                  label="Per-action cap"
                  hint="Maximum amount allowed for one action. Leave blank for no per-action cap."
                  valueKey="perAction"
                  form={form}
                  setForm={setForm}
                />
                <MoneyField
                  label="Require approval above"
                  hint="Actions above this amount require approval. A hard cap can still deny the action."
                  valueKey="approvalAbove"
                  form={form}
                  setForm={setForm}
                />
              </>
            ) : null}
            <MoneyField
              label="Daily cap"
              hint={
                form.meter === 'llm_usage'
                  ? 'Maximum estimated LLM spend per principal per UTC day. Leave blank for no daily cap.'
                  : 'Maximum total action amount per UTC day. Leave blank for no daily cap.'
              }
              valueKey="daily"
              form={form}
              setForm={setForm}
            />
            <MoneyField
              label="Weekly cap"
              hint={
                form.meter === 'llm_usage'
                  ? 'Maximum estimated LLM spend per principal per UTC week. Leave blank for no weekly cap.'
                  : 'Maximum total action amount per UTC week. Leave blank for no weekly cap.'
              }
              valueKey="weekly"
              form={form}
              setForm={setForm}
            />
            <MoneyField
              label="Monthly cap"
              hint={
                form.meter === 'llm_usage'
                  ? 'Maximum estimated LLM spend per principal per UTC month. Leave blank for no monthly cap.'
                  : 'Maximum total action amount per UTC month. Leave blank for no monthly cap.'
              }
              valueKey="monthly"
              form={form}
              setForm={setForm}
            />
          </div>
          {form.meter === 'actions' ? (
            <div className="rounded-md border p-3">
              <label className="flex items-start gap-3 text-sm">
                <Checkbox
                  checked={form.grantRequired}
                  onCheckedChange={(checked) =>
                    setFormValue(setForm, 'grantRequired', checked === true)
                  }
                />
                <span className="grid gap-2">
                  <span className="font-medium">Require user intent proof</span>
                  <span className="text-muted-foreground">
                    Turn this on when each payment must point back to the user&apos;s request, such
                    as “buy this article” or “buy this coffee.”
                  </span>
                  <span className="grid gap-1 rounded-md bg-muted/40 p-3 text-xs text-muted-foreground">
                    <span>
                      <span className="font-medium text-foreground">Where it comes from:</span> The
                      customer app can create a reusable authorization grant from verified user
                      intent and pass its grant and attempt ids with the action.
                    </span>
                    <span>
                      <span className="font-medium text-foreground">What this policy does:</span>{' '}
                      requires a matching, active grant before the common kernel permits signing.
                    </span>
                    <span>
                      <span className="font-medium text-foreground">What gets checked:</span> agent,
                      amount, rail, merchant/resource, pay-to, and x402 network/asset.
                    </span>
                  </span>
                </span>
              </label>
            </div>
          ) : null}
          <div className="grid gap-3 md:grid-cols-3">
            <ActionField
              label="Cap breach"
              hint={
                form.meter === 'llm_usage'
                  ? 'Effect returned when estimated LLM spend exceeds one of the configured caps.'
                  : 'Effect returned when an action exceeds one of the configured caps.'
              }
              value={form.onBreach}
              onValueChange={(value) => setFormValue(setForm, 'onBreach', value)}
            />
            {form.meter === 'actions' ? (
              <>
                <ActionField
                  label="Missing evidence"
                  hint="Effect returned when required evidence was not provided."
                  value={form.missingEvidenceEffect}
                  onValueChange={(value) => setFormValue(setForm, 'missingEvidenceEffect', value)}
                />
                <ActionField
                  label="Failed evidence"
                  hint="Effect returned when supplied evidence says a required precondition is false."
                  value={form.failedPreconditionEffect}
                  onValueChange={(value) =>
                    setFormValue(setForm, 'failedPreconditionEffect', value)
                  }
                />
              </>
            ) : null}
          </div>
          {form.meter === 'actions' && form.actionKind === 'refund' ? (
            <div className="grid gap-2">
              <Label>Required refund evidence</Label>
              <p className="text-xs leading-relaxed text-muted-foreground">
                Select the facts the caller must provide and satisfy before a refund can be
                authorized.
              </p>
              <div className="grid gap-2 md:grid-cols-2">
                {REFUND_PRECONDITIONS.map((item) => (
                  <label
                    key={item.id}
                    className="flex items-center gap-2 rounded-md border px-3 py-2 text-sm"
                  >
                    <Checkbox
                      checked={form.requiredPreconditions.includes(item.id)}
                      onCheckedChange={(checked) =>
                        setForm((prev) => ({
                          ...prev,
                          requiredPreconditions: checked
                            ? [...prev.requiredPreconditions, item.id]
                            : prev.requiredPreconditions.filter((id) => id !== item.id),
                        }))
                      }
                    />
                    {item.label}
                  </label>
                ))}
              </div>
            </div>
          ) : null}
          {policyIds.has(form.id) ? (
            <p className="text-sm text-muted-foreground">
              Saving will update the existing control with this id.
            </p>
          ) : null}
        </div>
        <DialogFooter>
          <Button type="button" variant="outline" onClick={() => onOpenChange(false)}>
            Cancel
          </Button>
          <Button type="button" disabled={saving} onClick={createControl}>
            {saving ? 'Saving...' : editing ? 'Save financial policy' : 'Create financial policy'}
          </Button>
        </DialogFooter>
      </DialogContent>
    </Dialog>
  );
}

function formFromPolicy(policy: FamilyPolicyRow): FinancialControlForm {
  const meter: FinancialControlForm['meter'] =
    policy.meter === 'llm_usage' ? 'llm_usage' : 'actions';
  return {
    id: policy.id,
    description: policy.description ?? '',
    meter,
    // An LLM budget with no principal selector applies to every principal;
    // do not backfill the actions-flavored default agent.
    agent: policy.when?.agents?.[0] ?? (meter === 'llm_usage' ? '' : DEFAULT_FORM.agent),
    actionKind: pick(policy.when?.action_kinds?.[0], ACTION_KINDS, DEFAULT_FORM.actionKind),
    operation: policy.when?.operations?.[0] ?? DEFAULT_FORM.operation,
    currency: policy.when?.currencies?.[0] ?? DEFAULT_FORM.currency,
    rail: pick(policy.when?.rails?.[0], RAILS, DEFAULT_FORM.rail),
    perAction: minorToDollars(policy.per_transaction_minor),
    approvalAbove: minorToDollars(policy.approval_threshold_minor),
    daily: minorToDollars(policy.daily_minor),
    weekly: minorToDollars(policy.weekly_minor),
    monthly: minorToDollars(policy.monthly_minor),
    grantRequired:
      policy.grant_required ?? (meter === 'llm_usage' ? false : DEFAULT_FORM.grantRequired),
    onBreach: pick(policy.on_breach, EFFECTS, DEFAULT_FORM.onBreach),
    missingEvidenceEffect: pick(
      policy.missing_evidence_effect,
      EFFECTS,
      DEFAULT_FORM.missingEvidenceEffect,
    ),
    failedPreconditionEffect: pick(
      policy.failed_precondition_effect,
      EFFECTS,
      DEFAULT_FORM.failedPreconditionEffect,
    ),
    requiredPreconditions:
      policy.required_preconditions ??
      (policy.when?.action_kinds?.[0] === 'refund' ? DEFAULT_FORM.requiredPreconditions : []),
  };
}

function formForMeter(
  form: FinancialControlForm,
  meter: FinancialControlForm['meter'],
): FinancialControlForm {
  if (meter === form.meter) return form;
  if (meter === 'llm_usage') {
    return {
      ...form,
      meter,
      id: form.id === DEFAULT_FORM.id ? DEFAULT_LLM_BUDGET_FORM.id : form.id,
      description:
        form.description === DEFAULT_FORM.description
          ? DEFAULT_LLM_BUDGET_FORM.description
          : form.description,
      agent: form.agent === DEFAULT_FORM.agent ? '' : form.agent,
      perAction: '',
      approvalAbove: '',
      daily: form.daily === DEFAULT_FORM.daily ? DEFAULT_LLM_BUDGET_FORM.daily : form.daily,
      weekly: form.weekly === DEFAULT_FORM.weekly ? DEFAULT_LLM_BUDGET_FORM.weekly : form.weekly,
      monthly:
        form.monthly === DEFAULT_FORM.monthly ? DEFAULT_LLM_BUDGET_FORM.monthly : form.monthly,
      grantRequired: false,
      requiredPreconditions: [],
    };
  }
  return {
    ...form,
    meter,
    id: form.id === DEFAULT_LLM_BUDGET_FORM.id ? DEFAULT_FORM.id : form.id,
    description:
      form.description === DEFAULT_LLM_BUDGET_FORM.description
        ? DEFAULT_FORM.description
        : form.description,
    agent: form.agent === DEFAULT_LLM_BUDGET_FORM.agent ? DEFAULT_FORM.agent : form.agent,
    perAction: form.perAction === '' ? DEFAULT_FORM.perAction : form.perAction,
    approvalAbove: form.approvalAbove === '' ? DEFAULT_FORM.approvalAbove : form.approvalAbove,
    daily: form.daily === DEFAULT_LLM_BUDGET_FORM.daily ? DEFAULT_FORM.daily : form.daily,
    weekly: form.weekly === DEFAULT_LLM_BUDGET_FORM.weekly ? DEFAULT_FORM.weekly : form.weekly,
    monthly: form.monthly === DEFAULT_LLM_BUDGET_FORM.monthly ? DEFAULT_FORM.monthly : form.monthly,
    grantRequired: form.grantRequired || DEFAULT_FORM.grantRequired,
  };
}

function minorToDollars(value: number | null | undefined): string {
  if (value == null) return '';
  return (value / 100).toFixed(2).replace(/\.00$/, '');
}

function pick<T extends string>(
  value: string | null | undefined,
  allowed: ReadonlyArray<T>,
  fallback: T,
): T {
  return allowed.includes(value as T) ? (value as T) : fallback;
}

function Field({
  label,
  hint,
  children,
}: {
  label: string;
  hint: string;
  children: ReactNode;
}) {
  return (
    <div className="grid gap-1.5">
      <Label>{label}</Label>
      {children}
      <p className="text-xs leading-relaxed text-muted-foreground">{hint}</p>
    </div>
  );
}

function MoneyField({
  label,
  hint,
  valueKey,
  form,
  setForm,
}: {
  label: string;
  hint: string;
  valueKey: 'perAction' | 'approvalAbove' | 'daily' | 'weekly' | 'monthly';
  form: FinancialControlForm;
  setForm: Dispatch<SetStateAction<FinancialControlForm>>;
}) {
  return (
    <Field label={label} hint={hint}>
      <Input
        inputMode="decimal"
        value={form[valueKey]}
        onChange={(event) => setFormValue(setForm, valueKey, event.target.value)}
      />
    </Field>
  );
}

function ActionField({
  label,
  hint,
  value,
  onValueChange,
}: {
  label: string;
  hint: string;
  value: 'deny' | 'require_approval';
  onValueChange: (value: 'deny' | 'require_approval') => void;
}) {
  return (
    <Field label={label} hint={hint}>
      <Select
        value={value}
        onValueChange={(next) => onValueChange(next as 'deny' | 'require_approval')}
      >
        <SelectTrigger className="w-full">
          <SelectValue />
        </SelectTrigger>
        <SelectContent>
          <SelectItem value="deny">Deny</SelectItem>
          <SelectItem value="require_approval">Require approval</SelectItem>
        </SelectContent>
      </Select>
    </Field>
  );
}

function formPayload(form: FinancialControlForm) {
  const id = form.id.trim();
  const agent = form.agent.trim();
  if (id === '') throw new Error('Control id is required');

  if (form.meter === 'llm_usage') {
    const daily = dollarsToMinorOrUndefined(form.daily);
    const weekly = dollarsToMinorOrUndefined(form.weekly);
    const monthly = dollarsToMinorOrUndefined(form.monthly);
    if (daily === undefined && weekly === undefined && monthly === undefined) {
      throw new Error('Set at least one cap window');
    }
    return {
      id,
      description: form.description.trim() || undefined,
      severity: 'high',
      meter: 'llm_usage',
      // Empty selectors = every principal gets its own cap.
      when: agent === '' ? {} : { agents: [agent] },
      daily_minor: daily,
      weekly_minor: weekly,
      monthly_minor: monthly,
      on_breach: form.onBreach,
    };
  }

  const operation = form.operation.trim();
  const currency = form.currency.trim().toUpperCase();
  if (agent === '') throw new Error('Agent is required');
  if (operation === '') throw new Error('Operation is required');
  if (currency === '') throw new Error('Currency is required');
  return {
    id,
    description: form.description.trim() || undefined,
    severity: 'high',
    meter: 'actions',
    when: {
      agents: [agent],
      action_kinds: [form.actionKind],
      operations: [operation],
      currencies: [currency],
      rails: [form.rail],
    },
    per_transaction_minor: dollarsToMinorOrUndefined(form.perAction),
    approval_threshold_minor: dollarsToMinorOrUndefined(form.approvalAbove),
    daily_minor: dollarsToMinorOrUndefined(form.daily),
    weekly_minor: dollarsToMinorOrUndefined(form.weekly),
    monthly_minor: dollarsToMinorOrUndefined(form.monthly),
    grant_required: form.grantRequired,
    required_preconditions: form.actionKind === 'refund' ? form.requiredPreconditions : [],
    missing_evidence_effect: form.missingEvidenceEffect,
    failed_precondition_effect: form.failedPreconditionEffect,
    on_breach: form.onBreach,
  };
}

function dollarsToMinorOrUndefined(value: string): number | undefined {
  const trimmed = value.trim();
  if (trimmed === '') return undefined;
  if (!/^\d+(\.\d{1,2})?$/.test(trimmed)) {
    throw new Error('Amounts must be non-negative dollars with up to two decimals');
  }
  const [dollars, cents = ''] = trimmed.split('.');
  return Number(dollars) * 100 + Number(cents.padEnd(2, '0'));
}

function setFormValue<K extends keyof FinancialControlForm>(
  setForm: Dispatch<SetStateAction<FinancialControlForm>>,
  key: K,
  value: FinancialControlForm[K],
) {
  setForm((prev) => ({ ...prev, [key]: value }));
}
