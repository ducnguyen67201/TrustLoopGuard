import { cleanup, render, screen, waitFor, within } from '@testing-library/react';
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

  it('renders the recorded outcome for an already-reviewed action without a Review button', async () => {
    stubFetch();
    render(<ReviewQueueContent workspaceSlug="demo" />);

    await waitFor(() => expect(screen.getByText('Rejected')).toBeInTheDocument());
    // Two unreviewed actionable rows → exactly two Review triggers.
    expect(screen.getAllByRole('button', { name: /^Review / })).toHaveLength(2);
  });

  it('posts the review outcome to the review-events endpoint', async () => {
    const mock = stubFetch();
    const user = userEvent.setup();
    render(<ReviewQueueContent workspaceSlug="demo" />);

    await waitFor(() => expect(screen.getAllByRole('button', { name: /^Review / })).toHaveLength(2));
    const [firstReview] = screen.getAllByRole('button', { name: /^Review / });
    await user.click(firstReview!);

    const dialog = await screen.findByRole('dialog');
    await user.click(within(dialog).getByRole('button', { name: 'Record decision' }));

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
  });
});
