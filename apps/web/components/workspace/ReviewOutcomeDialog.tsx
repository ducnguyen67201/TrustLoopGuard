'use client';

import { ClipboardCheck, Loader2 } from 'lucide-react';
import { useRouter } from 'next/navigation';
import { useState } from 'react';
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
  DialogTrigger,
} from '@/components/ui/dialog';
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
  REVIEW_OUTCOME_OPTIONS,
  REVIEW_REASON_OPTIONS,
  buildReviewEventPayload,
  canSubmitReviewOutcome,
  type ReviewOutcome,
  type ReviewReasonCode,
} from '@/lib/review-outcomes';

type ReviewOutcomeDialogProps = {
  traceId: string;
  workspaceSlug: string;
  currentOutcome: string;
};

export function ReviewOutcomeDialog({
  traceId,
  workspaceSlug,
  currentOutcome,
}: ReviewOutcomeDialogProps) {
  const router = useRouter();
  const [open, setOpen] = useState(false);
  const [outcome, setOutcome] = useState<ReviewOutcome | ''>('accepted');
  const [reasonCodes, setReasonCodes] = useState<ReviewReasonCode[]>([]);
  const [note, setNote] = useState('');
  const [submitting, setSubmitting] = useState(false);
  const canSubmit = canSubmitReviewOutcome(outcome);

  function reset() {
    setOutcome('accepted');
    setReasonCodes([]);
    setNote('');
    setSubmitting(false);
  }

  function handleOpenChange(next: boolean) {
    if (submitting) return;
    setOpen(next);
    if (!next) reset();
  }

  function toggleReason(reasonCode: ReviewReasonCode, checked: boolean) {
    setReasonCodes((current) => {
      if (checked) {
        return current.includes(reasonCode) ? current : [...current, reasonCode];
      }
      return current.filter((value) => value !== reasonCode);
    });
  }

  async function handleSubmit(event: React.FormEvent<HTMLFormElement>) {
    event.preventDefault();
    if (!canSubmit || submitting) return;

    setSubmitting(true);
    try {
      const response = await fetch(
        `/api/traces/${encodeURIComponent(traceId)}/review-events?workspace=${encodeURIComponent(
          workspaceSlug,
        )}`,
        {
          method: 'POST',
          headers: { 'Content-Type': 'application/json' },
          body: JSON.stringify(buildReviewEventPayload({ outcome, reasonCodes, note })),
        },
      );

      if (!response.ok) {
        throw new Error(await readErrorMessage(response));
      }

      toast.success('Review outcome recorded');
      setOpen(false);
      reset();
      router.refresh();
    } catch (error) {
      toast.error(error instanceof Error ? error.message : 'review outcome failed');
      setSubmitting(false);
    }
  }

  return (
    <Dialog open={open} onOpenChange={handleOpenChange}>
      <DialogTrigger asChild>
        <Button type="button" variant="ghost" size="sm">
          <ClipboardCheck className="size-4" />
          Record
        </Button>
      </DialogTrigger>
      <DialogContent className="max-w-lg">
        <DialogHeader>
          <DialogTitle>Record review outcome</DialogTitle>
          <DialogDescription>Current outcome: {currentOutcome}</DialogDescription>
        </DialogHeader>

        <form onSubmit={handleSubmit} className="grid gap-4">
          <fieldset disabled={submitting} className="grid gap-4">
            <div className="grid gap-2">
              <Label htmlFor={`review-outcome-${traceId}`}>Outcome</Label>
              <Select value={outcome} onValueChange={(value) => setOutcome(value as ReviewOutcome)}>
                <SelectTrigger id={`review-outcome-${traceId}`} className="w-full">
                  <SelectValue placeholder="Select outcome" />
                </SelectTrigger>
                <SelectContent>
                  {REVIEW_OUTCOME_OPTIONS.map((option) => (
                    <SelectItem key={option.value} value={option.value}>
                      {option.label}
                    </SelectItem>
                  ))}
                </SelectContent>
              </Select>
            </div>

            <div className="grid gap-2">
              <Label>Reasons</Label>
              <div className="grid gap-2 rounded-md border p-3 sm:grid-cols-2">
                {REVIEW_REASON_OPTIONS.map((option) => (
                  <label
                    key={option.value}
                    className="flex items-center gap-2 text-sm leading-none"
                  >
                    <Checkbox
                      checked={reasonCodes.includes(option.value)}
                      onCheckedChange={(checked) => toggleReason(option.value, checked === true)}
                    />
                    <span>{option.label}</span>
                  </label>
                ))}
              </div>
            </div>

            <div className="grid gap-2">
              <Label htmlFor={`review-note-${traceId}`}>Note</Label>
              <Textarea
                id={`review-note-${traceId}`}
                value={note}
                onChange={(event) => setNote(event.target.value)}
                rows={4}
                placeholder="What changed during review?"
              />
            </div>
          </fieldset>

          <DialogFooter>
            <Button
              type="button"
              variant="ghost"
              onClick={() => handleOpenChange(false)}
              disabled={submitting}
            >
              Cancel
            </Button>
            <Button type="submit" disabled={!canSubmit || submitting}>
              {submitting ? (
                <>
                  <Loader2 className="size-4 animate-spin" />
                  Saving...
                </>
              ) : (
                'Save outcome'
              )}
            </Button>
          </DialogFooter>
        </form>
      </DialogContent>
    </Dialog>
  );
}

async function readErrorMessage(response: Response): Promise<string> {
  const fallback = `review outcome failed (${response.status})`;
  const text = await response.text();
  if (text.trim() === '') return fallback;

  try {
    const parsed = JSON.parse(text) as { error?: string; message?: string };
    return parsed.error ?? parsed.message ?? fallback;
  } catch {
    return text;
  }
}
