import {
  toHumanReviewOutcomeRows,
  toHumanReviewReasonRows,
  toSummaryMetrics,
} from './transforms';
import { describe, expect, it } from 'vitest';

const analytics = {
  summary: {
    traceCount: 4,
    automatedInterventionCount: 3,
    humanReviewCount: 3,
    humanInterventionCount: 2,
    humanInterventionRate: 50,
    falsePositiveRate: 25,
  },
  outcomes: {
    acceptedCount: 0,
    correctedCount: 1,
    rejectedCount: 0,
    falsePositiveCount: 1,
    missedIssueCount: 1,
    ignoredCount: 0,
  },
  byWorkflowStep: [],
  byPolicy: [],
  byAgent: [],
  byRunKind: [],
  topReasons: [{ reasonCode: 'field_mismatch', count: 2 }],
};

const runs = [
  {
    id: '018f0c43-0000-7000-9000-000000000001',
    shortId: '018f0c43',
    agent: 'tax-agent',
    kind: 'Workflow',
    status: 'Running',
    externalId: 'None',
    traces: 4,
    blocked: 1,
    rewritten: 1,
    escalated: 1,
    p95LatencyMs: 42,
    latency: '42ms',
    started: 'now',
    startedAt: 'May 19, 2026',
    endedAt: 'Still running',
    metadata: [],
    href: '/runs/018f0c43-0000-7000-9000-000000000001',
  },
];

describe('analytics transforms', () => {
  it('prefers Rust human-review analytics for summary metrics', () => {
    expect(toSummaryMetrics(runs, analytics).humanInterventionRateLabel).toBe('50%');
  });

  it('maps non-zero human review outcome rows', () => {
    expect(toHumanReviewOutcomeRows(analytics)).toContainEqual({
      outcome: 'corrected',
      count: 1,
    });
  });

  it('keeps the top human review reason rows', () => {
    expect(toHumanReviewReasonRows(analytics)[0]?.reasonCode).toBe('field_mismatch');
  });
});
