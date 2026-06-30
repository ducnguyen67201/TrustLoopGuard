'use client';

import { IconCheck, IconDotsVertical, IconX } from '@tabler/icons-react';
import { useState } from 'react';
import { toast } from 'sonner';

import { Button } from '@/components/ui/button';
import {
  DropdownMenu,
  DropdownMenuContent,
  DropdownMenuItem,
  DropdownMenuTrigger,
} from '@/components/ui/dropdown-menu';
import { ReviewActionDialog, type Verdict } from '@/components/workspace/ReviewActionDialog';
import { buildReviewEventPayload, type ReviewOutcome } from '@/lib/review-outcomes';

interface ReviewRowActionsProps {
  traceId: string;
  verdict: Verdict;
  reason?: string | undefined;
  workspaceSlug: string;
  /** Stamp the row with the chosen outcome. For one-click actions this is
   * called optimistically (before the server responds); the dialog path calls
   * it after a confirmed success. Either way the row paints without a refetch. */
  onRecorded: (traceId: string, outcome: ReviewOutcome) => void;
  /** Called if a one-click post fails so the row can roll back. */
  onRevert: (traceId: string) => void;
}

async function postReviewOutcome(
  traceId: string,
  outcome: ReviewOutcome,
  workspaceSlug: string,
): Promise<void> {
  const query = workspaceSlug === '' ? '' : `?workspace=${encodeURIComponent(workspaceSlug)}`;
  const res = await fetch(`/api/traces/${encodeURIComponent(traceId)}/review-events${query}`, {
    method: 'POST',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify(buildReviewEventPayload({ outcome, reasonCodes: [], note: '' })),
  });
  if (!res.ok) {
    throw new Error(`review failed with ${res.status}`);
  }
}

export function ReviewRowActions({
  traceId,
  verdict,
  reason,
  workspaceSlug,
  onRecorded,
  onRevert,
}: ReviewRowActionsProps) {
  const [dialogOpen, setDialogOpen] = useState(false);
  const [pending, setPending] = useState(false);

  async function recordOutcome(outcome: ReviewOutcome) {
    if (pending) return;
    setPending(true);
    onRecorded(traceId, outcome); // optimistic stamp; revert on failure
    try {
      await postReviewOutcome(traceId, outcome, workspaceSlug);
      toast.success('Review recorded');
    } catch (err) {
      onRevert(traceId);
      toast.error(err instanceof Error ? err.message : 'Could not record review');
    } finally {
      setPending(false);
    }
  }

  return (
    <div className="flex items-center justify-end gap-1.5">
      <Button
        size="xs"
        variant="outline"
        disabled={pending}
        onClick={() => void recordOutcome('accepted')}
        aria-label={`Approve ${verdict} action ${traceId}`}
        className="text-[var(--color-allow)] hover:text-[var(--color-allow)]"
      >
        <IconCheck aria-hidden />
        Approve
      </Button>
      <Button
        size="xs"
        variant="outline"
        disabled={pending}
        onClick={() => void recordOutcome('rejected')}
        aria-label={`Reject ${verdict} action ${traceId}`}
        className="text-[var(--color-block)] hover:text-[var(--color-block)]"
      >
        <IconX aria-hidden />
        Reject
      </Button>
      <DropdownMenu>
        <DropdownMenuTrigger asChild>
          <Button
            size="icon-xs"
            variant="ghost"
            disabled={pending}
            aria-label={`More outcomes for ${verdict} action ${traceId}`}
          >
            <IconDotsVertical aria-hidden />
          </Button>
        </DropdownMenuTrigger>
        <DropdownMenuContent align="end" className="min-w-44">
          <DropdownMenuItem onSelect={() => setDialogOpen(true)}>
            Other outcome &amp; note…
          </DropdownMenuItem>
        </DropdownMenuContent>
      </DropdownMenu>

      <ReviewActionDialog
        traceId={traceId}
        verdict={verdict}
        reason={reason}
        workspaceSlug={workspaceSlug}
        onRecorded={(outcome) => onRecorded(traceId, outcome)}
        open={dialogOpen}
        onOpenChange={setDialogOpen}
      />
    </div>
  );
}
