'use client';

import { useEffect, useState } from 'react';
import type { RunDetail, RunSummary } from '@featherlane-ai/sdk';

const TERMINAL_RUN_STATUSES = new Set(['completed', 'failed', 'canceled', 'timed_out']);
const TERMINAL_EVALUATION_JOB_STATUSES = new Set(['completed', 'failed', 'inconclusive', 'error']);

function hasCompletedEvaluation(detail: RunDetail): boolean {
  if (detail.finalization === undefined || !TERMINAL_RUN_STATUSES.has(detail.run.status)) {
    return false;
  }
  if (
    detail.evaluations.length > 0 &&
    detail.evaluations.every((result) => result.verdict !== 'not_configured')
  ) {
    return true;
  }
  return (
    detail.evaluation_jobs.length > 0 &&
    detail.evaluation_jobs.every((job) => TERMINAL_EVALUATION_JOB_STATUSES.has(job.status))
  );
}

export function useActivationRun(
  verificationSessionId: string | null,
  workspaceSlug: string,
  environmentId: string,
) {
  const [detail, setDetail] = useState<RunDetail | null>(null);
  const [errors, setErrors] = useState(0);

  useEffect(() => {
    setDetail(null);
    setErrors(0);
    if (verificationSessionId === null) return;
    let cancelled = false;
    let timer: ReturnType<typeof setTimeout> | null = null;

    async function poll() {
      try {
        const params = new URLSearchParams({
          workspace: workspaceSlug,
          environment: environmentId,
          external_id: verificationSessionId ?? '',
          limit: '10',
        });
        const response = await fetch(`/api/runs?${params.toString()}`, { cache: 'no-store' });
        if (!response.ok) throw new Error('Run verification failed');
        const payload = (await response.json()) as { runs?: RunSummary[] };
        const exact = payload.runs?.find(
          (candidate) => candidate.external_id === verificationSessionId,
        );
        if (exact !== undefined) {
          const detailResponse = await fetch(
            `/api/runs/${encodeURIComponent(exact.id)}?${new URLSearchParams({
              workspace: workspaceSlug,
              environment: environmentId,
            }).toString()}`,
            { cache: 'no-store' },
          );
          if (!detailResponse.ok) throw new Error('Run detail verification failed');
          const nextDetail = (await detailResponse.json()) as RunDetail;
          if (cancelled) return;
          setDetail(nextDetail);
          if (hasCompletedEvaluation(nextDetail)) return;
        }
      } catch {
        if (!cancelled) setErrors((current) => current + 1);
      }
      if (!cancelled) timer = setTimeout(poll, 4_000);
    }

    void poll();
    return () => {
      cancelled = true;
      if (timer !== null) clearTimeout(timer);
    };
  }, [environmentId, verificationSessionId, workspaceSlug]);

  return {
    run: detail?.run ?? null,
    detail,
    evaluationComplete: detail !== null && hasCompletedEvaluation(detail),
    errors,
  };
}
