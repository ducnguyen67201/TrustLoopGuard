'use client';

import { Copy, ExternalLink, FileText, Loader2, Share2, Trash2 } from 'lucide-react';
import { useState } from 'react';
import { toast } from 'sonner';

import {
  landedPercent,
  redteam,
  type CreateReportInput,
  type ListJobsParams,
  type RedteamJobSummary,
  type RedteamReportShare,
} from '@/lib/redteam-jobs';
import { Button } from '@/components/ui/button';
import { Card, CardContent } from '@/components/ui/card';
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

const NONE = 'none';
const COMPARE_LIMIT = 20;
const TTL_OPTIONS = [
  { value: '7', label: '7 days' },
  { value: '30', label: '30 days' },
  { value: '90', label: '90 days' },
] as const;

interface ReportShareCardProps {
  job: RedteamJobSummary;
}

function messageOf(err: unknown): string {
  return err instanceof Error ? err.message : String(err);
}

function formatDate(iso: string): string {
  const date = new Date(iso);
  if (Number.isNaN(date.getTime())) return iso;
  return date.toLocaleDateString('en-US', { month: 'short', day: 'numeric', year: 'numeric' });
}

/** Two jobs are the same agent by `agent_id`, falling back to identical target. */
function sameAgent(a: RedteamJobSummary, b: RedteamJobSummary): boolean {
  if (a.agent_id !== null && b.agent_id !== null) return a.agent_id === b.agent_id;
  return a.target === b.target;
}

function compareLabel(job: RedteamJobSummary): string {
  return `${formatDate(job.created_at)} · ${job.profile} · ${landedPercent(job)}% landed`;
}

export function ReportShareCard({ job }: ReportShareCardProps) {
  const [open, setOpen] = useState(false);
  const [compareJobs, setCompareJobs] = useState<readonly RedteamJobSummary[]>([]);
  const [compareJobId, setCompareJobId] = useState<string>(NONE);
  const [ttlDays, setTtlDays] = useState<string>('30');
  const [submitting, setSubmitting] = useState(false);
  const [share, setShare] = useState<RedteamReportShare | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [revoking, setRevoking] = useState(false);

  const shareUrl =
    share !== null && typeof window !== 'undefined'
      ? new URL(share.path, window.location.origin).toString()
      : '';

  // Load other completed runs of the same agent when the dialog opens. Comparison
  // is optional, so a load failure is swallowed (the picker just stays empty).
  async function handleOpenChange(next: boolean): Promise<void> {
    setOpen(next);
    if (!next) return;
    setShare(null);
    setError(null);
    setCompareJobId(NONE);
    const params: ListJobsParams = { limit: COMPARE_LIMIT };
    if (job.agent_id !== null) params.agentId = job.agent_id;
    try {
      const jobs = await redteam.listJobs(params);
      setCompareJobs(
        jobs.filter((other) => other.id !== job.id && other.status === 'complete' && sameAgent(job, other)),
      );
    } catch {
      setCompareJobs([]);
    }
  }

  async function handleCreate(): Promise<void> {
    if (submitting) return;
    setSubmitting(true);
    setError(null);
    const input: CreateReportInput = { jobId: job.id, ttlDays: Number(ttlDays) };
    if (compareJobId !== NONE) input.compareJobId = compareJobId;
    try {
      const created = await redteam.createReport(input);
      setShare(created);
      toast.success('Shareable report link created');
    } catch (err) {
      setError(messageOf(err));
      toast.error(messageOf(err));
    } finally {
      setSubmitting(false);
    }
  }

  async function handleCopy(): Promise<void> {
    if (shareUrl === '') return;
    try {
      await navigator.clipboard.writeText(shareUrl);
      toast.success('Link copied');
    } catch {
      toast.error('Could not copy — copy it manually');
    }
  }

  async function handleRevoke(): Promise<void> {
    if (share === null || revoking) return;
    setRevoking(true);
    try {
      await redteam.revokeReport(share.token);
      toast.success('Link revoked');
      setShare(null);
      setOpen(false);
    } catch (err) {
      toast.error(messageOf(err));
    } finally {
      setRevoking(false);
    }
  }

  return (
    <Card className="border-l-4 border-l-primary">
      <CardContent className="flex flex-wrap items-center justify-between gap-3 pt-6">
        <div className="grid gap-1">
          <p className="flex items-center gap-2 text-xs font-semibold tracking-wide text-primary uppercase">
            <FileText className="size-4" />
            Shareable report
          </p>
          <p className="max-w-md text-sm text-muted-foreground">
            Turn this result into a simple link anyone can open — no login needed. Share just this
            test, or a before-and-after against an earlier test of the same agent.
          </p>
        </div>

        <Dialog open={open} onOpenChange={(next) => void handleOpenChange(next)}>
          <DialogTrigger asChild>
            <Button variant="outline">
              <Share2 className="size-4" />
              Create report link
            </Button>
          </DialogTrigger>
          <DialogContent className="w-[calc(100vw-2rem)] sm:max-w-md">
            <DialogHeader>
              <DialogTitle>Create shareable report</DialogTitle>
              <DialogDescription className="truncate">{job.target}</DialogDescription>
            </DialogHeader>

            {share === null ? (
              <div className="grid gap-4">
                <div className="grid gap-2">
                  <Label htmlFor="report-compare">Compare against</Label>
                  <Select value={compareJobId} onValueChange={setCompareJobId}>
                    <SelectTrigger id="report-compare">
                      <SelectValue />
                    </SelectTrigger>
                    <SelectContent>
                      <SelectItem value={NONE}>No comparison — single run</SelectItem>
                      {compareJobs.map((other) => (
                        <SelectItem key={other.id} value={other.id}>
                          {compareLabel(other)}
                        </SelectItem>
                      ))}
                    </SelectContent>
                  </Select>
                  {compareJobs.length === 0 ? (
                    <p className="text-xs text-muted-foreground">
                      No other completed runs of this agent yet — add a fix and test again to compare.
                    </p>
                  ) : null}
                </div>

                <div className="grid gap-2">
                  <Label htmlFor="report-ttl">Link expires in</Label>
                  <Select value={ttlDays} onValueChange={setTtlDays}>
                    <SelectTrigger id="report-ttl">
                      <SelectValue />
                    </SelectTrigger>
                    <SelectContent>
                      {TTL_OPTIONS.map((option) => (
                        <SelectItem key={option.value} value={option.value}>
                          {option.label}
                        </SelectItem>
                      ))}
                    </SelectContent>
                  </Select>
                </div>

                {error ? (
                  <p className="rounded-md border border-destructive/40 bg-destructive/10 px-3 py-2 text-sm text-destructive">
                    {error}
                  </p>
                ) : null}

                <DialogFooter>
                  <Button variant="ghost" onClick={() => setOpen(false)}>
                    Cancel
                  </Button>
                  <Button onClick={() => void handleCreate()} disabled={submitting}>
                    {submitting ? (
                      <Loader2 className="size-4 animate-spin motion-reduce:animate-none" />
                    ) : (
                      <Share2 className="size-4" />
                    )}
                    {submitting ? 'Creating…' : 'Create link'}
                  </Button>
                </DialogFooter>
              </div>
            ) : (
              <div className="grid min-w-0 gap-3">
                <div className="flex min-w-0 items-center gap-2 rounded-md border bg-muted/40 px-3 py-2">
                  <span className="min-w-0 flex-1 truncate font-mono text-xs" aria-label="Shareable report link">
                    {shareUrl}
                  </span>
                  <Button size="sm" variant="ghost" aria-label="Copy link" onClick={() => void handleCopy()}>
                    <Copy className="size-4" />
                  </Button>
                </div>
                <p className="text-xs text-muted-foreground">
                  Opens a branded PDF — no login needed.
                  {share.expires_at ? ` Expires ${formatDate(share.expires_at)}.` : ''}
                </p>
                <DialogFooter className="gap-2 sm:justify-between">
                  <Button variant="ghost" onClick={() => void handleRevoke()} disabled={revoking}>
                    {revoking ? (
                      <Loader2 className="size-4 animate-spin motion-reduce:animate-none" />
                    ) : (
                      <Trash2 className="size-4" />
                    )}
                    Revoke
                  </Button>
                  <Button asChild>
                    <a href={shareUrl} target="_blank" rel="noopener noreferrer">
                      <ExternalLink className="size-4" />
                      Open report
                    </a>
                  </Button>
                </DialogFooter>
              </div>
            )}
          </DialogContent>
        </Dialog>
      </CardContent>
    </Card>
  );
}
