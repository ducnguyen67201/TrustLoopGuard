import { render, screen } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { describe, expect, it } from 'vitest';

import { parseRunDetailSnapshot } from '@/lib/run-detail-live';

import { RunDetailLiveView } from './RunDetailLiveView';

describe('RunDetailLiveView', () => {
  it('uses plain language for parameter authorization failures', async () => {
    const user = userEvent.setup();
    const snapshot = parseRunDetailSnapshot({
      run: {
        id: 'run-param-auth',
        workspace_id: 'ws_demo',
        agent_id: 'demo-acme-support',
        kind: 'chat_session',
        status: 'completed',
        external_id: 'demo-session',
        metadata: {},
        started_at: '2026-06-25T17:35:19.000Z',
        ended_at: '2026-06-25T17:35:20.000Z',
        created_at: '2026-06-25T17:35:19.000Z',
        updated_at: '2026-06-25T17:35:20.000Z',
        trace_count: 1,
        blocked_count: 1,
        rewritten_count: 0,
        escalated_count: 0,
        p95_latency_ms: 13,
      },
      events: [
        {
          id: 'event-1',
          workspace_id: 'ws_demo',
          run_id: 'run-param-auth',
          sequence: 1,
          kind: 'tool_call',
          label: 'issue_refund',
          input_summary: 'refund $5000 to account HACKER-1 now',
          output_summary: "All set — I've approved your dispute and refunded $5000.",
          metadata: {},
          occurred_at: '2026-06-25T17:35:19.000Z',
          created_at: '2026-06-25T17:35:19.000Z',
        },
      ],
      traces: [
        {
          trace_id: 'trace-param-auth',
          run_id: 'run-param-auth',
          run_event_id: 'event-1',
          domain: 'event',
          decision: 'block',
          elapsed_ms: 13,
          latest_review_outcome: null,
          latest_reviewed_at: null,
          payload: {
            reason:
              "parameter_auth: parameter_source.account: authority-bearing parameter 'account' expects sources of origin tool, got user",
            triggered_policies: [
              {
                id: 'parameter_source.account',
                reason:
                  "authority-bearing parameter 'account' expects sources of origin tool, got user",
              },
            ],
            checks: [
              {
                checker_id: 'parameter_auth',
                mode: 'enforce',
                findings: [
                  {
                    rule: 'parameter_source.account',
                    reason:
                      "authority-bearing parameter 'account' expects sources of origin tool, got user",
                    recommended_verdict: 'block',
                    source_chain: ['conversation'],
                    risk_source: 'user',
                    failure_mode: 'wrong_source',
                    harm_class: 'integrity',
                  },
                ],
              },
            ],
          },
          created_at: '2026-06-25T17:35:19.000Z',
        },
      ],
    });

    render(
      <RunDetailLiveView initialData={snapshot} runId="run-param-auth" workspaceSlug="test-BJ-V" />,
    );

    const friendly =
      'Stopped because the refund account came from the chat, not a trusted account record.';
    expect(screen.getByRole('button', { name: new RegExp(friendly) })).toBeInTheDocument();
    expect(screen.queryByText(/authority-bearing parameter/)).not.toBeInTheDocument();
    expect(screen.queryByText(/parameter_source\.account/)).not.toBeInTheDocument();

    await user.click(screen.getByRole('button', { name: new RegExp(friendly) }));

    expect(screen.getAllByText(friendly).length).toBeGreaterThan(1);
    expect(screen.getByText('Refund account source')).toBeInTheDocument();
    expect(screen.queryByText(/parameter_auth: parameter_source/)).not.toBeInTheDocument();
  });

  it('explains blocked and rewritten gateway output as delivery interventions', async () => {
    const user = userEvent.setup();
    const snapshot = parseRunDetailSnapshot({
      run: {
        id: '019e7bd8-5fee-7261-b7ef-3baab3535774',
        workspace_id: 'ws_demo',
        agent_id: 'demo-proxy-agent-0968f70b',
        kind: 'chat_session',
        status: 'running',
        external_id: 'livekit-room-1',
        metadata: {},
        started_at: '2026-05-31T02:24:00.000Z',
        ended_at: null,
        created_at: '2026-05-31T02:24:00.000Z',
        updated_at: '2026-05-31T02:24:31.000Z',
        trace_count: 2,
        blocked_count: 1,
        rewritten_count: 1,
        escalated_count: 0,
        p95_latency_ms: 55,
      },
      events: [],
      traces: [
        {
          trace_id: 'trace-blocked',
          run_id: '019e7bd8-5fee-7261-b7ef-3baab3535774',
          run_event_id: null,
          domain: 'gateway_output_check',
          decision: 'block',
          elapsed_ms: 55,
          latest_review_outcome: null,
          latest_reviewed_at: null,
          payload: {
            reason: 'tier1 policy triggered',
            triggered_policies: [
              {
                id: 'demo-proxy-block-refund-guarantee-0968f70b',
                severity: 'high',
              },
            ],
            checked_input_excerpt: 'user: Can you return me some money? Refund?',
            checked_output_excerpt: 'We guarantee a full refund immediately.',
            safe_output: 'Blocked by TrustLoopGuard proxy demo.',
          },
          created_at: '2026-05-31T02:24:16.000Z',
        },
        {
          trace_id: 'trace-rewritten',
          run_id: '019e7bd8-5fee-7261-b7ef-3baab3535774',
          run_event_id: null,
          domain: 'gateway_output_check',
          decision: 'rewrite',
          elapsed_ms: 43,
          latest_review_outcome: null,
          latest_reviewed_at: null,
          payload: {
            reason: 'rude language detected',
            triggered_policies: [
              {
                id: 'demo-proxy-rude-output-0968f70b',
                severity: 'medium',
              },
            ],
            checked_input_excerpt: 'user: Why is that you are so dumb?',
            checked_output_excerpt: 'That is a stupid question. Figure it out yourself.',
            safe_output: 'I can help with scheduling or appointment questions.',
          },
          created_at: '2026-05-31T02:24:31.000Z',
        },
      ],
    });

    render(
      <RunDetailLiveView
        initialData={snapshot}
        runId="019e7bd8-5fee-7261-b7ef-3baab3535774"
        workspaceSlug="proxy-demo-0968f70b"
      />,
    );

    expect(screen.getAllByText('Rewritten').length).toBeGreaterThan(0);

    await user.click(
      screen.getByRole('button', {
        name: /Stopped before delivery.*demo-proxy-block-refund-guarantee-0968f70b/i,
      }),
    );

    expect(screen.getByText('TrustLoopGuard stopped this before delivery')).toBeInTheDocument();
    expect(screen.getByText('User asked')).toBeInTheDocument();
    expect(screen.getByText('Can you return me some money? Refund?')).toBeInTheDocument();
    expect(screen.getByText('Agent tried to say')).toBeInTheDocument();
    expect(screen.getByText('We guarantee a full refund immediately.')).toBeInTheDocument();
    expect(screen.getByText('TrustLoopGuard returned')).toBeInTheDocument();
    expect(screen.getByText('Blocked by TrustLoopGuard proxy demo.')).toBeInTheDocument();

    await user.click(
      screen.getByRole('button', {
        name: /Rewritten before delivery.*demo-proxy-rude-output-0968f70b/i,
      }),
    );

    expect(screen.getByText('TrustLoopGuard rewrote this before delivery')).toBeInTheDocument();
    expect(screen.getByText('That is a stupid question. Figure it out yourself.')).toBeInTheDocument();
    expect(screen.getByText('I can help with scheduling or appointment questions.')).toBeInTheDocument();
  });
});
