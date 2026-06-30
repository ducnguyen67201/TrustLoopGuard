'use client';

import { IconGavel } from '@tabler/icons-react';
import { useState } from 'react';
import type { FormEvent } from 'react';
import { toast } from 'sonner';

import { Badge } from '@/components/ui/badge';
import { Button } from '@/components/ui/button';
import { Dialog, DialogContent, DialogTrigger } from '@/components/ui/dialog';
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
  DialogFooterBar,
  DialogShellHeader,
  FieldHint,
  FormRow,
} from '@/components/workspace/dialog-scaffold';
import { buildReviewEventPayload, type ReviewOutcome } from '@/lib/review-outcomes';

// The four verdicts a trace can carry. Matches tl-core Verdict and the Badge
// variant names so a verdict string drops straight into <Badge variant>.
export type Verdict = 'allow' | 'block' | 'escalate' | 'rewrite';

// The inline row actions cover accepted/rejected in one click, so the dialog
// leads with the nuanced long-tail outcomes. accepted/rejected stay selectable
// here for completeness (e.g. when adding a note to a plain accept).
const OUTCOMES: ReadonlyArray<{ value: ReviewOutcome; label: string }> = [
  { value: 'false_positive', label: 'False positive — guard over-flagged' },
  { value: 'corrected', label: 'Corrected — fixed, then allowed' },
  { value: 'missed_issue', label: 'Missed issue — guard should have caught more' },
  { value: 'ignored', label: 'Ignored — no action needed' },
  { value: 'accepted', label: 'Accepted — the action was legitimate' },
  { value: 'rejected', label: 'Rejected — should not proceed' },
];

const DEFAULT_OUTCOME: ReviewOutcome = 'false_positive';

interface ReviewActionDialogProps {
  traceId: string;
  verdict: Verdict;
  reason?: string | undefined;
  workspaceSlug: string;
  /** Called with the recorded outcome on a successful submit, so the caller
   * can stamp the row without refetching. */
  onRecorded: (outcome: ReviewOutcome) => void;
  /**
   * Controlled open state. When provided, the dialog renders without its own
   * trigger button — the caller owns opening it (e.g. from a row overflow menu).
   * Omit both to keep the self-contained "Review" trigger behaviour.
   */
  open?: boolean;
  onOpenChange?: (open: boolean) => void;
}

export function ReviewActionDialog({
  traceId,
  verdict,
  reason,
  workspaceSlug,
  onRecorded,
  open: controlledOpen,
  onOpenChange,
}: ReviewActionDialogProps) {
  const isControlled = controlledOpen !== undefined;
  const [uncontrolledOpen, setUncontrolledOpen] = useState(false);
  const open = isControlled ? controlledOpen : uncontrolledOpen;
  const [outcome, setOutcome] = useState<ReviewOutcome>(DEFAULT_OUTCOME);
  const [note, setNote] = useState('');
  const [submitting, setSubmitting] = useState(false);

  function setOpen(nextOpen: boolean) {
    if (isControlled) {
      onOpenChange?.(nextOpen);
    } else {
      setUncontrolledOpen(nextOpen);
    }
  }

  function resetForm() {
    setOutcome(DEFAULT_OUTCOME);
    setNote('');
  }

  async function handleSubmit(event: FormEvent) {
    event.preventDefault();
    setSubmitting(true);
    try {
      const query = workspaceSlug === '' ? '' : `?workspace=${encodeURIComponent(workspaceSlug)}`;
      const res = await fetch(`/api/traces/${encodeURIComponent(traceId)}/review-events${query}`, {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify(buildReviewEventPayload({ outcome, reasonCodes: [], note })),
      });
      if (!res.ok) {
        throw new Error(`review failed with ${res.status}`);
      }
      toast.success('Review recorded');
      onRecorded(outcome);
      resetForm();
      setOpen(false);
    } catch (err) {
      toast.error(err instanceof Error ? err.message : 'Could not record review');
    } finally {
      setSubmitting(false);
    }
  }

  return (
    <Dialog
      open={open}
      onOpenChange={(nextOpen) => {
        setOpen(nextOpen);
        if (!nextOpen) resetForm();
      }}
    >
      {isControlled ? null : (
        <DialogTrigger asChild>
          <Button size="sm" variant="outline" aria-label={`Review ${verdict} action ${traceId}`}>
            Review
          </Button>
        </DialogTrigger>
      )}
      <DialogContent>
        <DialogShellHeader
          icon={<IconGavel />}
          eyebrow="Human review"
          title="Record a review decision"
          description="Logs your decision and an audit trail for this action. It does not resume or re-run the stopped action — your agent owns re-issuing the call."
        />
        <form onSubmit={handleSubmit} className="grid gap-4">
          <div className="flex items-center gap-2 text-xs text-muted-foreground">
            <Badge variant={verdict}>{verdict}</Badge>
            <span className="truncate font-mono">{traceId}</span>
          </div>
          {reason !== undefined ? <FieldHint>{reason}</FieldHint> : null}
          <FormRow>
            <Label htmlFor={`review-${traceId}-outcome`}>Outcome</Label>
            <Select value={outcome} onValueChange={(value) => setOutcome(value as ReviewOutcome)}>
              <SelectTrigger id={`review-${traceId}-outcome`} className="w-full">
                <SelectValue />
              </SelectTrigger>
              <SelectContent>
                {OUTCOMES.map((option) => (
                  <SelectItem key={option.value} value={option.value}>
                    {option.label}
                  </SelectItem>
                ))}
              </SelectContent>
            </Select>
          </FormRow>
          <FormRow>
            <Label htmlFor={`review-${traceId}-note`}>Note (optional)</Label>
            <Textarea
              id={`review-${traceId}-note`}
              value={note}
              onChange={(event) => setNote(event.target.value)}
              rows={3}
              placeholder="Why you made this call"
            />
          </FormRow>
          <DialogFooterBar
            onCancel={() => {
              resetForm();
              setOpen(false);
            }}
            submitting={submitting}
            submitLabel="Record decision"
            submittingLabel="Recording…"
          />
        </form>
      </DialogContent>
    </Dialog>
  );
}
