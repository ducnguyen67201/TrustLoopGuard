import {
  toHumanReviewOutcomeRows,
  toHumanReviewReasonRows,
  toSummaryMetrics,
} from './transforms';

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

if (toSummaryMetrics(runs, analytics).humanInterventionRateLabel !== '50%') {
  throw new Error('expected human intervention rate from Rust analytics');
}

if (toHumanReviewOutcomeRows(analytics)[1]?.outcome !== 'corrected') {
  throw new Error('expected corrected review outcome row');
}

if (toHumanReviewReasonRows(analytics)[0]?.reasonCode !== 'field_mismatch') {
  throw new Error('expected top review reason row');
}
