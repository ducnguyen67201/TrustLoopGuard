import { cleanup, render, screen, waitFor } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { afterEach, beforeEach, describe, expect, test, vi } from 'vitest';

import { ConnectAgentStep } from './ConnectAgentStep';

const CREATED = {
  api_key: { id: 'key_1', name: 'support-ai key', prefix: 'tl_live_abc' },
  plaintext_key: 'tl_live_abc123secret',
};

function jsonResponse(body: unknown, status = 201): Response {
  return new Response(JSON.stringify(body), {
    status,
    headers: { 'Content-Type': 'application/json' },
  });
}

describe('ConnectAgentStep', () => {
  const fetchMock = vi.fn<typeof fetch>();

  beforeEach(() => {
    fetchMock.mockReset();
    vi.stubGlobal('fetch', fetchMock);
  });

  afterEach(() => {
    cleanup();
    vi.unstubAllGlobals();
  });

  function renderStep(requestedEnvironmentId?: string) {
    return render(
      <ConnectAgentStep
        baseUrl="https://api.example.test"
        environmentId="env_default"
        defaultAgentId="support-ai"
        workspaceSlug="acme"
        requestedEnvironmentId={requestedEnvironmentId}
      />,
    );
  }

  test('creates a key for the active environment via the shared http client', async () => {
    fetchMock.mockResolvedValueOnce(jsonResponse(CREATED));
    renderStep();

    await userEvent.click(screen.getByRole('button', { name: /create my api key/i }));

    // lib/http.ts appends ?workspace=&environment= from the page URL in the
    // browser; jsdom has neither, so the bare path is expected here.
    await waitFor(() => {
      expect(fetchMock).toHaveBeenCalledWith(
        '/api/api-keys',
        expect.objectContaining({ method: 'POST' }),
      );
    });
    const body = JSON.parse(String(fetchMock.mock.calls[0]?.[1]?.body)) as Record<string, string>;
    expect(body['environment_id']).toBe('env_default');
    expect(body['name']).toBe('support-ai key');
  });

  test('sanitizes free-form agent names before they reach snippets', async () => {
    fetchMock.mockResolvedValueOnce(jsonResponse(CREATED));
    renderStep();

    const input = screen.getByLabelText(/name your agent/i);
    await userEvent.clear(input);
    await userEvent.type(input, "billing bot'; drop");
    expect(input).toHaveProperty('value', 'billing-bot-drop');

    await userEvent.click(screen.getByRole('button', { name: /create my api key/i }));
    await screen.findByDisplayValue(CREATED.plaintext_key);
    for (const pre of Array.from(document.querySelectorAll('pre'))) {
      expect(pre.textContent).toContain('billing-bot-drop');
      expect(pre.textContent).not.toContain("'; drop");
    }
  });

  test('reveals the key once and shows both integration snippets', async () => {
    fetchMock.mockResolvedValueOnce(jsonResponse(CREATED));
    renderStep();

    await userEvent.click(screen.getByRole('button', { name: /create my api key/i }));

    expect(await screen.findByDisplayValue(CREATED.plaintext_key)).toBeDefined();
    expect(screen.getByText(/shown only once/i)).toBeDefined();
    expect(screen.getByText(/add the sdk yourself/i)).toBeDefined();
    expect(screen.getByText(/paste this into claude code/i)).toBeDefined();
    // The plaintext secret must never leak into the snippet bodies.
    for (const pre of Array.from(document.querySelectorAll('pre'))) {
      expect(pre.textContent).not.toContain(CREATED.plaintext_key);
    }
  });

  test('tailors the assistant prompt to the selected coding assistant', async () => {
    fetchMock.mockResolvedValueOnce(jsonResponse(CREATED));
    renderStep();

    await userEvent.click(screen.getByRole('button', { name: /create my api key/i }));
    expect(await screen.findByText(/paste this into claude code/i)).toBeDefined();

    await userEvent.click(screen.getByRole('button', { name: 'Hermes' }));

    expect(screen.getByText(/paste this into hermes/i)).toBeDefined();
    expect(screen.getByText(/Open Hermes in this project/i)).toBeDefined();
  });

  test('trims long copy blocks until the user asks to show all', async () => {
    fetchMock.mockResolvedValueOnce(jsonResponse(CREATED));
    renderStep();

    await userEvent.click(screen.getByRole('button', { name: /create my api key/i }));

    const sdkBlock = screen.getByText(/import \{ Client, guard \}/i).closest('div');
    expect(sdkBlock?.textContent).not.toContain('onEscalate');

    await userEvent.click(screen.getAllByRole('button', { name: /show all/i })[0]!);

    expect(sdkBlock?.textContent).toContain('onEscalate');
  });

  test('links carry workspace and the requested environment', async () => {
    fetchMock.mockResolvedValueOnce(jsonResponse(CREATED));
    renderStep('env_staging');

    await userEvent.click(screen.getByRole('button', { name: /create my api key/i }));

    const continueLink = await screen.findByRole('link', { name: /watch for my first event/i });
    expect(continueLink.getAttribute('href')).toBe(
      '/onboarding/verify?workspace=acme&environment=env_staging',
    );
    const skipLink = screen.getByRole('link', { name: /skip setup/i });
    expect(skipLink.getAttribute('href')).toBe('/?workspace=acme&environment=env_staging');
  });

  test('links omit environment when none was requested', async () => {
    fetchMock.mockResolvedValueOnce(jsonResponse(CREATED));
    renderStep();

    await userEvent.click(screen.getByRole('button', { name: /create my api key/i }));

    const continueLink = await screen.findByRole('link', { name: /watch for my first event/i });
    expect(continueLink.getAttribute('href')).toBe('/onboarding/verify?workspace=acme');
  });

  test('stays on the form and keeps input when creation fails', async () => {
    fetchMock.mockResolvedValueOnce(jsonResponse({ error: 'nope' }, 500));
    renderStep();

    const input = screen.getByLabelText(/name your agent/i);
    await userEvent.clear(input);
    await userEvent.type(input, 'billing-bot');
    await userEvent.click(screen.getByRole('button', { name: /create my api key/i }));

    await waitFor(() => {
      expect(screen.getByLabelText(/name your agent/i)).toHaveProperty('value', 'billing-bot');
    });
    expect(screen.queryByDisplayValue(CREATED.plaintext_key)).toBeNull();
  });
});
