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
  holdAbove: string;
  daily: string;
  weekly: string;
  monthly: string;
  mandateRequired: boolean;
  onBreach: 'block' | 'escalate';
  missingEvidenceAction: 'block' | 'escalate';
  failedPreconditionAction: 'block' | 'escalate';
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
const ACTIONS: ReadonlyArray<'block' | 'escalate'> = ['block', 'escalate'];

const REFUND_PRECONDITIONS = [
  { id: 'order_exists', label: 'Order exists' },
  { id: 'payment_captured', label: 'Payment captured' },
  { id: 'refund_window_open', label: 'Refund window open' },
  { id: 'amount_lte_refundable_balance', label: 'Amount within refundable balance' },
  { id: 'destination_is_original_payment_method', label: 'Original payment method' },
  { id: 'no_duplicate_refund', label: 'No duplicate refund' },
] as const;

const DEFAULT_FORM: FinancialControlForm = {
  id: 'x402-agentic-payment-mandate-required',
  description: 'x402 payment controls for commerce agents',
  meter: 'actions',
  agent: 'spid:commerce-agent',
  actionKind: 'payment',
  operation: 'x402_read_paid_resource',
  currency: 'USD',
  rail: 'x402',
  perAction: '5',
  holdAbove: '',
  daily: '50',
  weekly: '',
  monthly: '5000',
  mandateRequired: true,
  onBreach: 'block',
  missingEvidenceAction: 'escalate',
  failedPreconditionAction: 'block',
  requiredPreconditions: [],
};

const DEFAULT_LLM_BUDGET_FORM: FinancialControlForm = {
  ...DEFAULT_FORM,
  id: 'llm-weekly-budget',
  description: 'Weekly LLM spend cap per principal',
  meter: 'llm_usage',
  agent: '',
  perAction: '',
  holdAbove: '',
  daily: '',
  weekly: '50',
  monthly: '',
  mandateRequired: false,
  requiredPreconditions: [],
};

export function FinancialPolicyCreateDialog({
  open,
  onOpenChange,
  contextQuery,
  initialPolicy,
  existingPolicyIds = [],
  onCreated,
}: {
  open: boolean;
  onOpenChange: (open: boolean) => void;
  contextQuery: string;
  initialPolicy?: FamilyPolicyRow | undefined;
  existingPolicyIds?: string[];
  onCreated?: (policy: FamilyPolicyRow) => void;
}) {
  const [saving, setSaving] = useState(false);
  const [form, setForm] = useState<FinancialControlForm>(DEFAULT_FORM);
  const policyIds = useMemo(() => new Set(existingPolicyIds), [existingPolicyIds]);
  const editing = initialPolicy !== undefined;

  useEffect(() => {
    if (open) setForm(initialPolicy ? formFromPolicy(initialPolicy) : DEFAULT_FORM);
  }, [initialPolicy, open]);

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
              ? 'Hard-cap gateway LLM spend per principal. TrustLoopGuard reserves each request’s maximum cost before the provider call, then settles to actual usage. Budgeted calls require trusted model pricing and a max_tokens bound.'
              : 'Define the mandate requirement, caps, evidence checks, and approval behavior TrustLoopGuard evaluates before agent execution.'}
          </DialogDescription>
        </DialogHeader>
        <div className="grid max-h-[70vh] gap-4 overflow-y-auto pr-1">
          <Field label="Applies to">
            <Select
              value={form.meter}
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
            <Field label="Control id">
              <Input
                value={form.id}
                disabled={editing}
                onChange={(event) => setFormValue(setForm, 'id', event.target.value)}
              />
            </Field>
            <Field label={form.meter === 'llm_usage' ? 'Principal (optional)' : 'Agent'}>
              <Input
                value={form.agent}
                onChange={(event) => setFormValue(setForm, 'agent', event.target.value)}
              />
            </Field>
            <Field label="Description">
              <Input
                value={form.description}
                onChange={(event) => setFormValue(setForm, 'description', event.target.value)}
              />
            </Field>
            {form.meter === 'actions' ? (
              <>
                <Field label="Operation">
                  <Input
                    value={form.operation}
                    onChange={(event) => setFormValue(setForm, 'operation', event.target.value)}
                  />
                </Field>
                <Field label="Currency">
                  <Input
                    value={form.currency}
                    onChange={(event) => setFormValue(setForm, 'currency', event.target.value)}
                  />
                </Field>
                <Field label="Action kind">
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
                <Field label="Rail">
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
                  valueKey="perAction"
                  form={form}
                  setForm={setForm}
                />
                <MoneyField label="Hold above" valueKey="holdAbove" form={form} setForm={setForm} />
              </>
            ) : null}
            <MoneyField label="Daily cap" valueKey="daily" form={form} setForm={setForm} />
            <MoneyField label="Weekly cap" valueKey="weekly" form={form} setForm={setForm} />
            <MoneyField label="Monthly cap" valueKey="monthly" form={form} setForm={setForm} />
          </div>
          {form.meter === 'actions' ? (
            <div className="rounded-md border p-3">
              <label className="flex items-start gap-3 text-sm">
                <Checkbox
                  checked={form.mandateRequired}
                  onCheckedChange={(checked) =>
                    setFormValue(setForm, 'mandateRequired', checked === true)
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
                      <span className="font-medium text-foreground">Where it comes from:</span>{' '}
                      TrustLoopGuard can store an internal mandate from the user message, or the
                      customer app can send an external mandate reference.
                    </span>
                    <span>
                      <span className="font-medium text-foreground">What this policy does:</span>{' '}
                      requires the payment action to include that mandate before signing.
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
              value={form.onBreach}
              onValueChange={(value) => setFormValue(setForm, 'onBreach', value)}
            />
            {form.meter === 'actions' ? (
              <>
                <ActionField
                  label="Missing evidence"
                  value={form.missingEvidenceAction}
                  onValueChange={(value) => setFormValue(setForm, 'missingEvidenceAction', value)}
                />
                <ActionField
                  label="Failed evidence"
                  value={form.failedPreconditionAction}
                  onValueChange={(value) =>
                    setFormValue(setForm, 'failedPreconditionAction', value)
                  }
                />
              </>
            ) : null}
          </div>
          {form.meter === 'actions' && form.actionKind === 'refund' ? (
            <div className="grid gap-2">
              <Label>Required refund evidence</Label>
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
    holdAbove: minorToDollars(policy.hold_above_minor),
    daily: minorToDollars(policy.daily_minor),
    weekly: minorToDollars(policy.weekly_minor),
    monthly: minorToDollars(policy.monthly_minor),
    mandateRequired:
      policy.mandate_required ?? (meter === 'llm_usage' ? false : DEFAULT_FORM.mandateRequired),
    onBreach: pick(policy.on_breach, ACTIONS, DEFAULT_FORM.onBreach),
    missingEvidenceAction: pick(
      policy.missing_evidence_action,
      ACTIONS,
      DEFAULT_FORM.missingEvidenceAction,
    ),
    failedPreconditionAction: pick(
      policy.failed_precondition_action,
      ACTIONS,
      DEFAULT_FORM.failedPreconditionAction,
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
      holdAbove: '',
      daily: form.daily === DEFAULT_FORM.daily ? DEFAULT_LLM_BUDGET_FORM.daily : form.daily,
      weekly: form.weekly === DEFAULT_FORM.weekly ? DEFAULT_LLM_BUDGET_FORM.weekly : form.weekly,
      monthly:
        form.monthly === DEFAULT_FORM.monthly ? DEFAULT_LLM_BUDGET_FORM.monthly : form.monthly,
      mandateRequired: false,
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
    holdAbove: form.holdAbove === '' ? DEFAULT_FORM.holdAbove : form.holdAbove,
    daily: form.daily === DEFAULT_LLM_BUDGET_FORM.daily ? DEFAULT_FORM.daily : form.daily,
    weekly: form.weekly === DEFAULT_LLM_BUDGET_FORM.weekly ? DEFAULT_FORM.weekly : form.weekly,
    monthly: form.monthly === DEFAULT_LLM_BUDGET_FORM.monthly ? DEFAULT_FORM.monthly : form.monthly,
    mandateRequired: form.mandateRequired || DEFAULT_FORM.mandateRequired,
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

function Field({ label, children }: { label: string; children: ReactNode }) {
  return (
    <div className="grid gap-1.5">
      <Label>{label}</Label>
      {children}
    </div>
  );
}

function MoneyField({
  label,
  valueKey,
  form,
  setForm,
}: {
  label: string;
  valueKey: 'perAction' | 'holdAbove' | 'daily' | 'weekly' | 'monthly';
  form: FinancialControlForm;
  setForm: Dispatch<SetStateAction<FinancialControlForm>>;
}) {
  return (
    <Field label={label}>
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
  value,
  onValueChange,
}: {
  label: string;
  value: 'block' | 'escalate';
  onValueChange: (value: 'block' | 'escalate') => void;
}) {
  return (
    <Field label={label}>
      <Select value={value} onValueChange={(next) => onValueChange(next as 'block' | 'escalate')}>
        <SelectTrigger className="w-full">
          <SelectValue />
        </SelectTrigger>
        <SelectContent>
          <SelectItem value="block">Deny</SelectItem>
          <SelectItem value="escalate">Hold</SelectItem>
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
    hold_above_minor: dollarsToMinorOrUndefined(form.holdAbove),
    daily_minor: dollarsToMinorOrUndefined(form.daily),
    weekly_minor: dollarsToMinorOrUndefined(form.weekly),
    monthly_minor: dollarsToMinorOrUndefined(form.monthly),
    mandate_required: form.mandateRequired,
    required_preconditions: form.actionKind === 'refund' ? form.requiredPreconditions : [],
    missing_evidence_action: form.missingEvidenceAction,
    failed_precondition_action: form.failedPreconditionAction,
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
