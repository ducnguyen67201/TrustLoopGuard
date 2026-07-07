import { cleanup, render, screen } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { afterEach, beforeAll, describe, expect, it, vi } from 'vitest';

import { FinancialPolicyCreateDialog } from './FinancialSpendingControlsCard';

vi.mock('sonner', () => ({
  toast: { success: vi.fn(), error: vi.fn() },
}));

beforeAll(() => {
  Object.defineProperty(HTMLElement.prototype, 'hasPointerCapture', {
    configurable: true,
    value: vi.fn(() => false),
  });
  Object.defineProperty(HTMLElement.prototype, 'scrollIntoView', {
    configurable: true,
    value: vi.fn(),
  });
});

describe('FinancialPolicyCreateDialog', () => {
  afterEach(() => {
    cleanup();
    vi.unstubAllGlobals();
  });

  it('creates an LLM usage budget without financial-action selectors', async () => {
    const fetchMock = vi.fn<typeof fetch>(
      async () =>
        new Response(
          JSON.stringify({
            id: 'llm-weekly-budget',
            meter: 'llm_usage',
            weekly_minor: 5000,
          }),
          { status: 200 },
        ),
    );
    vi.stubGlobal('fetch', fetchMock);

    render(
      <FinancialPolicyCreateDialog
        open
        onOpenChange={vi.fn()}
        contextQuery="?workspace=demo&environment=production"
      />,
    );

    const meterSelect = screen.getAllByRole('combobox')[0];
    if (meterSelect === undefined) {
      throw new Error('expected meter select');
    }
    await userEvent.click(meterSelect);
    await userEvent.click(await screen.findByText('LLM usage (gateway)'));
    await userEvent.click(screen.getByRole('button', { name: 'Create financial policy' }));

    expect(fetchMock).toHaveBeenCalledTimes(1);
    const call = fetchMock.mock.calls[0];
    if (call === undefined) {
      throw new Error('expected fetch call');
    }
    const [url, init] = call;
    expect(String(url)).toBe('/api/financial/policies?workspace=demo&environment=production');
    expect(init?.method).toBe('POST');
    expect(JSON.parse(String(init?.body))).toEqual({
      id: 'llm-weekly-budget',
      description: 'Weekly LLM spend cap per principal',
      severity: 'high',
      meter: 'llm_usage',
      when: {},
      weekly_minor: 5000,
      on_breach: 'block',
    });
  });
});
