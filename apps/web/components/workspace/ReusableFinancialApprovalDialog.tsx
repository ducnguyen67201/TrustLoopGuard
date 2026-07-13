'use client';

import { IconFingerprint } from '@tabler/icons-react';
import { useEffect, useState } from 'react';
import { toast } from 'sonner';
import { z } from 'zod';
import type {
  ApproveMatchingFinancialActionsResponse,
  FinancialApprovalEnvelope,
} from '@trustloopguard/sdk';

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
import { safeError } from './financial-utils';

type ReusableFinancialApprovalDialogProps = {
  actionId: string;
  contextQuery: string;
  onClose: () => void;
  onApproved: (result: ApproveMatchingFinancialActionsResponse) => Promise<void> | void;
};

export function ReusableFinancialApprovalDialog({
  actionId,
  contextQuery,
  onClose,
  onApproved,
}: ReusableFinancialApprovalDialogProps) {
  const [envelope, setEnvelope] = useState<FinancialApprovalEnvelope | null>(null);
  const [loadingEnvelope, setLoadingEnvelope] = useState(true);
  const [submitting, setSubmitting] = useState(false);
  const [maxAmount, setMaxAmount] = useState('');
  const [expiresAt, setExpiresAt] = useState(defaultExpiryInput());

  useEffect(() => {
    let active = true;

    async function loadEnvelope() {
      setLoadingEnvelope(true);
      try {
        const response = await fetch(
          `/api/financial/actions/${encodeURIComponent(actionId)}/approval-envelope${contextQuery}`,
        );
        const text = await response.text();
        if (!response.ok) {
          throw new Error(safeError(text) ?? 'Unable to load approval fingerprint');
        }
        const nextEnvelope = financialApprovalEnvelopeSchema.parse(
          JSON.parse(text),
        ) as FinancialApprovalEnvelope;
        if (!active) return;
        setEnvelope(nextEnvelope);
        setMaxAmount(minorUnitsInput(nextEnvelope.current_amount_minor));
      } catch (error) {
        if (!active) return;
        toast.error(error instanceof Error ? error.message : 'Unable to load approval fingerprint');
        onClose();
      } finally {
        if (active) setLoadingEnvelope(false);
      }
    }

    void loadEnvelope();
    return () => {
      active = false;
    };
    // The dialog is remounted for each action. Closing it must not restart the request.
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [actionId, contextQuery]);

  async function approveMatching() {
    if (!envelope) return;
    let maxAmountMinor: number;
    try {
      maxAmountMinor = parseMinorUnits(maxAmount);
      if (maxAmountMinor < Number(envelope.current_amount_minor)) {
        throw new Error('Maximum must cover this action');
      }
    } catch (error) {
      toast.error(error instanceof Error ? error.message : 'Maximum is invalid');
      return;
    }
    const expiry = new Date(expiresAt);
    if (Number.isNaN(expiry.getTime()) || expiry.getTime() <= Date.now()) {
      toast.error('Expiry must be in the future');
      return;
    }

    setSubmitting(true);
    try {
      const response = await fetch(
        `/api/financial/actions/${encodeURIComponent(actionId)}/approve-matching${contextQuery}`,
        {
          method: 'POST',
          headers: { 'Content-Type': 'application/json' },
          body: JSON.stringify({
            action_fingerprint: envelope.action_fingerprint,
            max_amount_minor: maxAmountMinor,
            expires_at: expiry.toISOString(),
          }),
        },
      );
      const text = await response.text();
      if (!response.ok) {
        throw new Error(safeError(text) ?? 'Unable to activate reusable approval');
      }
      const result = approveMatchingResponseSchema.parse(
        JSON.parse(text),
      ) as ApproveMatchingFinancialActionsResponse;
      await onApproved(result);
      toast.success('Reusable approval active', {
        description: 'Matching actions can reuse human review until this approval expires.',
      });
      onClose();
    } catch (error) {
      toast.error(error instanceof Error ? error.message : 'Reusable approval failed');
    } finally {
      setSubmitting(false);
    }
  }

  return (
    <Dialog
      open
      onOpenChange={(open) => {
        if (!open && !submitting) onClose();
      }}
    >
      <DialogContent className="max-w-2xl">
        <DialogHeader>
          <DialogTitle>Approve matching actions</DialogTitle>
          <DialogDescription>
            Reuse this human approval for the same bounded action shape. TrustLoopGuard still checks
            every future action before money moves.
          </DialogDescription>
        </DialogHeader>
        {loadingEnvelope || !envelope ? (
          <p className="text-sm text-muted-foreground">Computing action fingerprint…</p>
        ) : (
          <div className="grid gap-4">
            <div className="grid gap-3 rounded-lg border bg-muted/30 p-4 sm:grid-cols-2">
              <ApprovalDetail label="Principal" value={envelope.principal_id} mono />
              <ApprovalDetail
                label="Action"
                value={`${envelope.action_kind} · ${envelope.operation}`}
              />
              <ApprovalDetail label="Rail" value={envelope.rail} />
              <ApprovalDetail
                label="Counterparty"
                value={envelope.counterparty_id ?? 'No counterparty'}
                mono
              />
            </div>
            <div className="grid gap-2">
              <div className="flex items-center justify-between gap-2">
                <Label>Action fingerprint</Label>
                <Badge variant="outline">v{envelope.fingerprint_version}</Badge>
              </div>
              <code className="break-all rounded-md border bg-muted px-3 py-2 text-xs">
                {envelope.action_fingerprint}
              </code>
              <p className="text-xs text-muted-foreground">
                Approval is bound to this action version. Amount is controlled separately by the
                maximum below.
              </p>
            </div>
            <div className="grid gap-3 sm:grid-cols-2">
              <div className="grid gap-2">
                <Label htmlFor="reuse-max-amount">Maximum per action ({envelope.currency})</Label>
                <Input
                  id="reuse-max-amount"
                  inputMode="decimal"
                  value={maxAmount}
                  onChange={(event) => setMaxAmount(event.target.value)}
                />
              </div>
              <div className="grid gap-2">
                <Label htmlFor="reuse-expires-at">Expires at</Label>
                <Input
                  id="reuse-expires-at"
                  type="datetime-local"
                  value={expiresAt}
                  onChange={(event) => setExpiresAt(event.target.value)}
                />
                <p className="text-xs text-muted-foreground">Maximum reusable window: 30 days.</p>
              </div>
            </div>
            <p className="text-sm text-muted-foreground">
              Changes to principal, action kind, operation, rail, currency, counterparty, or x402
              destination require a new approval.
            </p>
            <div className="grid gap-2 rounded-lg border border-primary/20 bg-primary/5 p-4 text-sm">
              <p className="font-medium">Only the matching human-review step is reused.</p>
              <p className="text-muted-foreground">
                Mandate status, hard policies, eligibility evidence, and the live available budget
                are checked again for every action. A matching fingerprint never reserves or
                guarantees funds in advance.
              </p>
            </div>
          </div>
        )}
        <DialogFooter>
          <Button type="button" variant="outline" disabled={submitting} onClick={onClose}>
            Cancel
          </Button>
          <Button type="button" disabled={!envelope || submitting} onClick={approveMatching}>
            <IconFingerprint />
            Approve once and reuse
          </Button>
        </DialogFooter>
      </DialogContent>
    </Dialog>
  );
}

function ApprovalDetail({
  label,
  value,
  mono = false,
}: {
  label: string;
  value: string;
  mono?: boolean;
}) {
  return (
    <div className="grid min-w-0 gap-1">
      <span className="text-xs text-muted-foreground">{label}</span>
      <span className={mono ? 'truncate font-mono text-xs' : 'truncate text-sm font-medium'}>
        {value}
      </span>
    </div>
  );
}

const financialActionRecordSchema = z.looseObject({
  id: z.string(),
  workspace_id: z.string(),
  status: z.enum([
    'proposed',
    'authorized',
    'held',
    'executed',
    'denied',
    'failed',
    'reversed',
    'expired',
  ]),
  status_reason: z.string().optional().nullable(),
  action: z.looseObject({
    kind: z.string(),
    principal_id: z.string(),
    amount: z.looseObject({
      amount_minor: z.union([z.number(), z.bigint()]),
      currency: z.string(),
    }),
  }),
  evidence: z.array(z.looseObject({})),
  created_at: z.string(),
  updated_at: z.string(),
});

const financialApprovalEnvelopeSchema = z.looseObject({
  action_id: z.string(),
  action_fingerprint: z.string().startsWith('sha256:v'),
  fingerprint_version: z.number().int().positive(),
  principal_id: z.string(),
  action_kind: z.string(),
  operation: z.string(),
  rail: z.string(),
  currency: z.string(),
  counterparty_id: z.string().optional().nullable(),
  current_amount_minor: z.union([z.number(), z.bigint()]),
  recommended_max_amount_minor: z.union([z.number(), z.bigint()]),
});

const approveMatchingResponseSchema = z.looseObject({
  action: financialActionRecordSchema,
  mandate: z.looseObject({ id: z.string(), version: z.number(), status: z.string() }),
  approval_envelope: financialApprovalEnvelopeSchema,
});

function defaultExpiryInput(): string {
  const date = new Date(Date.now() + 24 * 60 * 60 * 1000);
  const local = new Date(date.getTime() - date.getTimezoneOffset() * 60_000);
  return local.toISOString().slice(0, 16);
}

function minorUnitsInput(amount: number | bigint): string {
  const minor = BigInt(amount);
  return `${minor / 100n}.${(minor % 100n).toString().padStart(2, '0')}`;
}

function parseMinorUnits(value: string): number {
  const match = /^(\d+)(?:\.(\d{1,2}))?$/.exec(value.trim());
  if (!match) throw new Error('Enter an amount with up to two decimal places');
  const major = match[1];
  if (!major) throw new Error('Enter a valid amount');
  const minor = BigInt(major) * 100n + BigInt((match[2] ?? '').padEnd(2, '0'));
  if (minor > BigInt(Number.MAX_SAFE_INTEGER)) throw new Error('Maximum is too large');
  return Number(minor);
}
