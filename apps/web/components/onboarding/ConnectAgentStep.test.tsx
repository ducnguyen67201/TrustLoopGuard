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

  // The integration surface is now a tabbed control (SDK · AI assistant ·
  // Guard coding agents); only the active panel's <pre> is mounted at a time, so
  // these tests walk the tabs and assert each panel independently.
  test('sanitizes free-form agent names before they reach snippets', async () => {
    fetchMock.mockResolvedValueOnce(jsonResponse(CREATED));
    renderStep();

    const input = screen.getByLabelText(/name your agent/i);
    await userEvent.clear(input);
    await userEvent.type(input, "billing bot'; drop");
    expect(input).toHaveProperty('value', 'billing-bot-drop');

    await userEvent.click(screen.getByRole('button', { name: /create my api key/i }));
    await screen.findByDisplayValue(CREATED.plaintext_key);

    // Every integration panel's full payload must carry the sanitized id and
    // never the raw input. Previews are shortened, so expand each block first.
    for (const tab of [/^SDK$/i, /AI assistant/i, /Guard coding agents/i]) {
      await userEvent.click(screen.getByRole('tab', { name: tab }));
      for (const showAll of screen.queryAllByRole('button', { name: /show all/i })) {
        await userEvent.click(showAll);
      }
      const pres = Array.from(document.querySelectorAll('pre'));
      expect(pres.length).toBeGreaterThan(0);
      for (const pre of pres) {
        expect(pre.textContent).toContain('billing-bot-drop');
        expect(pre.textContent).not.toContain("'; drop");
      }
    }
  });

  test('reveals the key once and shows every integration snippet', async () => {
    fetchMock.mockResolvedValueOnce(jsonResponse(CREATED));
    renderStep();

    await userEvent.click(screen.getByRole('button', { name: /create my api key/i }));

    expect(await screen.findByDisplayValue(CREATED.plaintext_key)).toBeDefined();
    expect(screen.getByText(/shown only once/i)).toBeDefined();

    // SDK is the default panel.
    expect(screen.getByText(/add the sdk yourself/i)).toBeDefined();

    // AI assistant panel: default assistant is Claude Code.
    await userEvent.click(screen.getByRole('tab', { name: /AI assistant/i }));
    expect(screen.getByText(/paste this into claude code/i)).toBeDefined();

    // Coding-agent tool-gate panel.
    await userEvent.click(screen.getByRole('tab', { name: /Guard coding agents/i }));
    expect(screen.getByText(/install the claude code gate/i)).toBeDefined();
    expect(screen.getByText(/authorizes every emitted tool call/i)).toBeDefined();
    expect(screen.getByText(/managed projects fail closed/i)).toBeDefined();

    // The plaintext secret must never leak into any snippet body, on any tab.
    for (const tab of [/^SDK$/i, /AI assistant/i, /Guard coding agents/i]) {
      await userEvent.click(screen.getByRole('tab', { name: tab }));
      for (const pre of Array.from(document.querySelectorAll('pre'))) {
        expect(pre.textContent).not.toContain(CREATED.plaintext_key);
      }
    }
  });

  test('changes only the target in the coding-agent install command', async () => {
    fetchMock.mockResolvedValueOnce(jsonResponse(CREATED));
    renderStep();

    await userEvent.click(screen.getByRole('button', { name: /create my api key/i }));
    await screen.findByDisplayValue(CREATED.plaintext_key);
    await userEvent.click(screen.getByRole('tab', { name: /Guard coding agents/i }));

    expect(screen.getByText(/--target claude/i)).toBeDefined();
    await userEvent.click(screen.getByRole('button', { name: 'Codex' }));
    expect(screen.getByText(/--target codex/i)).toBeDefined();
    await userEvent.click(screen.getByRole('button', { name: 'OpenCode' }));
    expect(screen.getByText(/--target opencode/i)).toBeDefined();

    for (const pre of Array.from(document.querySelectorAll('pre'))) {
      expect(pre.textContent).not.toContain(CREATED.plaintext_key);
      expect(pre.textContent).toContain('npx @trustloopguard/cli install');
      expect(pre.textContent).toContain('https://api.example.test');
      expect(pre.textContent).toContain('support-ai');
    }
  });

  test('tailors the assistant prompt to the selected coding assistant', async () => {
    fetchMock.mockResolvedValueOnce(jsonResponse(CREATED));
    renderStep();

    await userEvent.click(screen.getByRole('button', { name: /create my api key/i }));
    await screen.findByDisplayValue(CREATED.plaintext_key);

    await userEvent.click(screen.getByRole('tab', { name: /AI assistant/i }));
    expect(screen.getByText(/paste this into claude code/i)).toBeDefined();

    await userEvent.click(screen.getByRole('button', { name: 'Hermes' }));

    expect(screen.getByText(/paste this into hermes/i)).toBeDefined();

    // The tailored "Assistant workflow:" sentence now trails the numbered
    // actions (so the 5-line preview leads with the compact steps), so expand
    // the block before asserting the assistant-specific guidance is present.
    for (const showAll of screen.queryAllByRole('button', { name: /show all/i })) {
      await userEvent.click(showAll);
    }
    expect(screen.getByText(/Open Hermes in this project/i)).toBeDefined();
  });

  test('trims long copy blocks until the user asks to show all', async () => {
    fetchMock.mockResolvedValueOnce(jsonResponse(CREATED));
    renderStep();

    await userEvent.click(screen.getByRole('button', { name: /create my api key/i }));

    // SDK panel is active by default; its preview stays short.
    const sdkBlock = screen.getByText(/import \{ guardAgent \}/i).closest('div');
    expect(sdkBlock?.textContent).not.toContain('agent.reply(userMessage)');

    await userEvent.click(screen.getAllByRole('button', { name: /show all/i })[0]!);

    expect(sdkBlock?.textContent).toContain('agent.reply(userMessage)');
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

  test('listens after key creation and flips to connected on the first event', async () => {
    const trace = {
      trace_id: 'tr_first_1',
      decision: 'permit',
      elapsed_ms: 12,
      created_at: '2026-07-03T00:00:00Z',
    };
    fetchMock.mockResolvedValueOnce(jsonResponse(CREATED));
    // Every subsequent call is the /api/traces poll.
    fetchMock.mockImplementation(() => Promise.resolve(jsonResponse({ traces: [trace] }, 200)));
    renderStep();

    await userEvent.click(screen.getByRole('button', { name: /create my api key/i }));

    expect(await screen.findByText(/connected — we received your request/i)).toBeInTheDocument();
    const dashboardLink = screen.getByRole('link', { name: /continue to your dashboard/i });
    expect(dashboardLink.getAttribute('href')).toBe('/?workspace=acme');
    expect(screen.getByRole('link', { name: /see the event details/i }).getAttribute('href')).toBe(
      '/onboarding/verify?workspace=acme',
    );
  });

  test('shows the listening state while no event has arrived', async () => {
    fetchMock.mockResolvedValueOnce(jsonResponse(CREATED));
    fetchMock.mockImplementation(() => Promise.resolve(jsonResponse({ traces: [] }, 200)));
    renderStep();

    await userEvent.click(screen.getByRole('button', { name: /create my api key/i }));

    expect(await screen.findByText(/listening — run your agent once/i)).toBeInTheDocument();
    expect(screen.queryByText(/connected — we received your request/i)).toBeNull();
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
