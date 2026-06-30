import { cleanup, render, screen, waitFor } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { afterEach, describe, expect, it, vi } from 'vitest';

import { ReviewQueueContent } from './ReviewQueueContent';

vi.mock('sonner', () => ({
  toast: { success: vi.fn(), error: vi.fn() },
}));

const TRACES = {
  traces: [
    {
      trace_id: 'trace-escalate',
      decision: 'escalate',
      domain: 'payments',
      created_at: '2026-06-30T12:00:00.000Z',
      latest_review_outcome: null,
      payload: { reason: 'wire_transfer requires human approval' },
    },
    {
      trace_id: 'trace-block',
      decision: 'block',
      domain: 'payments',
      created_at: '2026-06-30T12:00:00.000Z',
      latest_review_outcome: null,
      payload: { reason: 'refund over cap' },
    },
    {
      trace_id: 'trace-allow',
      decision: 'allow',
      domain: 'payments',
      created_at: '2026-06-30T12:00:00.000Z',
      latest_review_outcome: null,
      payload: { reason: 'legit refund' },
    },
    {
      trace_id: 'trace-reviewed',
      decision: 'escalate',
      domain: 'payments',
      created_at: '2026-06-30T12:00:00.000Z',
      latest_review_outcome: 'rejected',
      payload: { reason: 'already handled' },
    },
  ],
};

function stubFetch(): ReturnType<typeof vi.fn<typeof fetch>> {
  const mock = vi.fn<typeof fetch>(async (input, init) => {
    const url = typeof input === 'string' ? input : input.toString();
    if (url.includes('/review-events')) {
      return new Response(JSON.stringify({ id: 'evt_1' }), { status: 201 });
    }
    return new Response(JSON.stringify(TRACES), { status: 200 });
  });
  vi.stubGlobal('fetch', mock);
  return mock;
}

afterEach(() => {
  cleanup();
  vi.unstubAllGlobals();
  vi.clearAllMocks();
});

describe('ReviewQueueContent', () => {
  it('shows only escalated and blocked actions, not allowed ones', async () => {
    stubFetch();
    render(<ReviewQueueContent workspaceSlug="demo" />);

    await waitFor(() =>
      expect(screen.getByText('wire_transfer requires human approval')).toBeInTheDocument(),
    );
    expect(screen.getByText('refund over cap')).toBeInTheDocument();
    expect(screen.queryByText('legit refund')).not.toBeInTheDocument();
  });

  it('renders the recorded outcome for an already-reviewed action without inline actions', async () => {
    stubFetch();
    render(<ReviewQueueContent workspaceSlug="demo" />);

    await waitFor(() => expect(screen.getByText('Rejected')).toBeInTheDocument());
    // Two unreviewed actionable rows → exactly two Approve and two Reject controls;
    // the reviewed row shows its outcome instead of inline actions.
    expect(screen.getAllByRole('button', { name: /^Approve / })).toHaveLength(2);
    expect(screen.getAllByRole('button', { name: /^Reject / })).toHaveLength(2);
  });

  it('posts accepted in a single click on Approve, no dialog', async () => {
    const mock = stubFetch();
    const user = userEvent.setup();
    render(<ReviewQueueContent workspaceSlug="demo" />);

    await waitFor(() =>
      expect(
        screen.getByRole('button', { name: 'Approve escalate action trace-escalate' }),
      ).toBeInTheDocument(),
    );

    // No dialog is involved in the common path.
    expect(screen.queryByRole('dialog')).not.toBeInTheDocument();

    await user.click(
      screen.getByRole('button', { name: 'Approve escalate action trace-escalate' }),
    );

    await waitFor(() => {
      const post = mock.mock.calls.find(([url]) =>
        (typeof url === 'string' ? url : url.toString()).includes('/review-events'),
      );
      expect(post).toBeDefined();
      const [url, init] = post!;
      expect(typeof url === 'string' ? url : url.toString()).toContain(
        '/api/traces/trace-escalate/review-events',
      );
      expect(JSON.parse(String(init?.body))).toMatchObject({
        outcome: 'accepted',
        reason_codes: [],
        metadata: { source: 'dashboard' },
      });
    });

    expect(screen.queryByRole('dialog')).not.toBeInTheDocument();
  });

  it('posts rejected in a single click on Reject', async () => {
    const mock = stubFetch();
    const user = userEvent.setup();
    render(<ReviewQueueContent workspaceSlug="demo" />);

    await waitFor(() =>
      expect(
        screen.getByRole('button', { name: 'Reject block action trace-block' }),
      ).toBeInTheDocument(),
    );

    await user.click(screen.getByRole('button', { name: 'Reject block action trace-block' }));

    await waitFor(() => {
      const post = mock.mock.calls.find(([url]) =>
        (typeof url === 'string' ? url : url.toString()).includes('/review-events'),
      );
      expect(post).toBeDefined();
      const [url, init] = post!;
      expect(typeof url === 'string' ? url : url.toString()).toContain(
        '/api/traces/trace-block/review-events',
      );
      expect(JSON.parse(String(init?.body))).toMatchObject({
        outcome: 'rejected',
        reason_codes: [],
        metadata: { source: 'dashboard' },
      });
    });
  });
});
