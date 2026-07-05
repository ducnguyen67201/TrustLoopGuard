import { cleanup, render, screen } from '@testing-library/react';
import { afterEach, describe, expect, it } from 'vitest';

import { TraceDetailPageContent } from './TraceDetailPageContent';
import type { TraceDetailPageData } from '@/lib/server/dashboard-data';

const SHELL = {
  user: { name: 'Duc', email: 'duc@example.com', avatar: '' },
  organization: { id: 'org_1', name: 'TrustLoop', slug: 'trustloop' },
  activeWorkspace: {
    id: 'ws_1',
    name: 'Demo Workspace',
    slug: 'demo',
    description: '',
    policyCount: 1,
    enabledPolicies: 1,
    agentCount: 1,
    sourceCount: 1,
    apiKeyCount: 1,
    role: 'owner',
  },
  workspaces: [],
  activeEnvironment: {
    id: 'env_1',
    slug: 'prod',
    name: 'Production',
    description: null,
    isDefault: true,
  },
  environments: [],
  agents: [],
};

function data(trace: TraceDetailPageData['trace']): TraceDetailPageData {
  return {
    ...SHELL,
    trace,
  };
}

afterEach(() => {
  cleanup();
});

describe('TraceDetailPageContent', () => {
  it('renders decision evidence, provenance, and raw JSON export for a rich trace', () => {
    render(
      <TraceDetailPageContent
        data={data({
          trace_id: 'trace-rich',
          run_id: 'run-123',
          run_event_id: 'event-456',
          session_id: 'session-789',
          environment_id: 'env_1',
          environment: 'Production',
          domain: 'payments',
          decision: 'escalate',
          elapsed_ms: 42,
          latest_review_outcome: null,
          latest_reviewed_at: null,
          created_at: '2026-06-30T12:00:00.000Z',
          payload: {
            reason: 'wire transfer requires human approval',
            event: {
              kind: 'tool.call.proposed',
              principal: { agent_id: 'payments-agent' },
              action: {
                operation: 'send_wire',
                parameters: { amount: 15000, currency: 'USD' },
              },
              sources: [
                {
                  id: 'payment_registry',
                  origin: 'tool',
                  uri: 'stripe://transfer/123',
                },
              ],
              provenance: {
                amount: ['payment_registry'],
              },
            },
            triggered_policies: [
              {
                id: 'wire-cap',
                severity: 'high',
                reason: 'amount exceeds manual review threshold',
              },
            ],
            checks: [
              {
                checker: 'payment-limit',
                status: 'failed',
                message: 'limit exceeded',
              },
            ],
          },
        })}
      />,
    );

    expect(screen.getByText('Trace replay')).toBeInTheDocument();
    expect(screen.getByText('Escalate')).toBeInTheDocument();
    expect(screen.getByText('wire transfer requires human approval')).toBeInTheDocument();
    expect(screen.getByText('send_wire')).toBeInTheDocument();
    expect(screen.getByText('Amount')).toBeInTheDocument();
    expect(screen.getByText('15000')).toBeInTheDocument();
    expect(screen.getByText('Source payment_registry')).toBeInTheDocument();
    expect(screen.getAllByText(/stripe:\/\/transfer\/123/).length).toBeGreaterThan(0);
    expect(screen.getByText('Provenance Amount')).toBeInTheDocument();
    expect(screen.getByText('wire-cap')).toBeInTheDocument();
    expect(screen.getByText('payment-limit')).toBeInTheDocument();
    expect(screen.getByRole('button', { name: 'Copy raw JSON' })).toBeInTheDocument();
    expect(screen.getByRole('link', { name: 'Download raw JSON' })).toHaveAttribute(
      'download',
      'trace-rich.json',
    );
  });

  it('renders stable empty states and raw JSON for an old minimal trace payload', () => {
    render(
      <TraceDetailPageContent
        data={data({
          trace_id: 'trace-minimal',
          run_id: null,
          run_event_id: null,
          session_id: null,
          environment_id: 'env_1',
          environment: 'Production',
          domain: 'legacy',
          decision: 'unknown-future-verdict',
          elapsed_ms: 7,
          latest_review_outcome: null,
          latest_reviewed_at: null,
          created_at: '2026-06-30T12:00:00.000Z',
          payload: {},
        })}
      />,
    );

    expect(screen.getByText('unknown-future-verdict')).toBeInTheDocument();
    expect(screen.getByText('No proposed action recorded')).toBeInTheDocument();
    expect(screen.getByText('No policy evidence recorded')).toBeInTheDocument();
    expect(screen.getByLabelText('Raw trace JSON')).toHaveTextContent(
      '"trace_id": "trace-minimal"',
    );
  });
});
