import { cleanup, fireEvent, render, screen, waitFor, within } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';

import { http } from '@/lib/http';
import { ProductionLoopSetup } from './ProductionLoopSetup';

vi.mock('./useActivationRun', () => ({
  useActivationRun: () => ({
    run: null,
    detail: null,
    evaluationComplete: false,
    errors: 0,
  }),
}));

const fallback = {
  id: 'fallback-provider',
  display_name: 'OpenAI fallback',
  kind: 'openai_compatible' as const,
  base_url: 'https://fallback.example.com',
  default_model: 'fallback-model',
  credential_status: 'configured' as const,
  created_at: '2026-08-11T00:00:00Z',
  updated_at: '2026-08-11T00:00:00Z',
};

function activationResponse(notificationRule = true) {
  return {
    route: {
      id: 'production-route',
      display_name: 'Production route',
      provider_connection_id: 'primary-provider',
      agent_id: 'agent-1',
      reliability_mode: 'standard',
      fallback_provider_connection_ids: ['fallback-provider'],
      created_at: '2026-08-11T00:00:00Z',
      updated_at: '2026-08-11T00:00:00Z',
    },
    agent_id: 'agent-1',
    evaluation_profile: {},
    ...(notificationRule ? { notification_rule: { id: 'rule-1' } } : {}),
    alerts_deferred: !notificationRule,
    verification_session_id: 'verify-fixed',
    data_handling_mode: 'no_body_retention',
    readiness: {
      status: 'needs_attention',
      checks: [
        {
          id: 'traffic_seen',
          label: 'Exact test traffic seen',
          ready: false,
          detail: 'Send the generated request.',
        },
      ],
    },
  };
}

describe('ProductionLoopSetup', () => {
  beforeEach(() => {
    vi.stubGlobal(
      'ResizeObserver',
      class {
        observe() {}
        unobserve() {}
        disconnect() {}
      },
    );
    vi.stubGlobal('crypto', { randomUUID: () => 'verify-fixed' });
  });

  afterEach(() => {
    cleanup();
    vi.restoreAllMocks();
    vi.unstubAllGlobals();
  });

  it('submits one production activation with base URL, fallback, privacy, and exact session', async () => {
    const user = userEvent.setup();
    const fetchMock = vi
      .fn<typeof fetch>()
      .mockResolvedValue(new Response(JSON.stringify(activationResponse()), { status: 201 }));
    vi.stubGlobal('fetch', fetchMock);
    const { container } = render(
      <ProductionLoopSetup
        workspaceSlug="workspace-1"
        environmentId="production"
        apiBaseUrl="https://guard.example.com"
        agents={[{ id: 'agent-1', name: 'Production agent' }]}
        providerConnections={[fallback]}
        activeRuntimeKeyCount={1}
      />,
    );

    expect(screen.getByText('Provider base URL')).toBeInTheDocument();
    expect(screen.getByText('Fallback provider')).toBeInTheDocument();
    const password = container.querySelector('input[type="password"]');
    const email = container.querySelector('input[type="email"]');
    if (password === null || email === null) throw new Error('activation inputs missing');
    await user.type(password, 'provider-secret');
    await user.type(email, 'ops@example.com');

    const selects = container.querySelectorAll('select');
    const fallbackSelect = selects.item(1);
    if (fallbackSelect === null) throw new Error('fallback select missing');
    fireEvent.change(fallbackSelect, { target: { value: 'fallback-provider' } });
    const privacyConfirmation = screen.getAllByRole('checkbox')[1];
    if (privacyConfirmation === undefined) throw new Error('privacy confirmation missing');
    await user.click(privacyConfirmation);
    await user.click(screen.getByRole('button', { name: 'Activate' }));

    await waitFor(() => expect(fetchMock).toHaveBeenCalledTimes(1));
    const activationCall = fetchMock.mock.calls[0];
    if (activationCall === undefined) throw new Error('activation request missing');
    const [url, init] = activationCall;
    expect(url).toBe('/api/gateway/activations?workspace=workspace-1&environment=production');
    expect(init?.method).toBe('POST');
    const body = JSON.parse(String(init?.body));
    expect(body.provider.base_url).toBe('https://api.openai.com');
    expect(body.provider.provider_api_key).toBe('provider-secret');
    expect(body.fallback_provider_connection_ids).toEqual(['fallback-provider']);
    expect(body.verification_session_id).toBe('verify-fixed');
    expect(body.alerts_deferred).toBe(false);
    expect(body.confirm_workspace_privacy_change).toBe(true);

    expect(await screen.findByText('Send exact verification traffic')).toBeInTheDocument();
    const snippet = screen.getByText(/X-Featherlane-Session-Id.*verify-fixed/);
    expect(snippet).toHaveTextContent(/X-Featherlane-Session-End.*true/);
    expect(snippet).not.toHaveTextContent('provider-secret');
    await user.click(screen.getByRole('button', { name: 'Python' }));
    expect(screen.getByText(/from openai import OpenAI/)).toHaveTextContent(
      '"X-Featherlane-Session-Id": "verify-fixed"',
    );
  });

  it('requires an explicit defer choice and keeps the resulting state visibly not ready', async () => {
    const user = userEvent.setup();
    const fetchMock = vi
      .fn<typeof fetch>()
      .mockResolvedValue(new Response(JSON.stringify(activationResponse(false)), { status: 201 }));
    vi.stubGlobal('fetch', fetchMock);
    const { container } = render(
      <ProductionLoopSetup
        workspaceSlug="workspace-1"
        environmentId="production"
        apiBaseUrl="https://guard.example.com"
        agents={[{ id: 'agent-1', name: 'Production agent' }]}
        providerConnections={[]}
        activeRuntimeKeyCount={1}
      />,
    );
    const password = container.querySelector('input[type="password"]');
    if (password === null) throw new Error('provider key input missing');
    await user.type(password, 'provider-secret');
    const checkboxes = screen.getAllByRole('checkbox');
    const defer = checkboxes[0];
    const privacyConfirmation = checkboxes[1];
    if (defer === undefined || privacyConfirmation === undefined) {
      throw new Error('activation confirmations missing');
    }
    await user.click(defer);
    await user.click(privacyConfirmation);
    const form = screen.getByRole('button', { name: 'Activate' }).closest('form');
    if (form === null) throw new Error('activation form missing');
    fireEvent.submit(form);

    await waitFor(() => expect(fetchMock).toHaveBeenCalledTimes(1));
    const activationCall = fetchMock.mock.calls[0];
    if (activationCall === undefined) throw new Error('activation request missing');
    const body = JSON.parse(String(activationCall[1]?.body));
    expect(body.alert_email).toBe('');
    expect(body.alerts_deferred).toBe(true);
    const alert = await screen.findByRole('alert');
    expect(within(alert).getByText('Email alerts deferred')).toBeInTheDocument();
    expect(within(alert).getByText(/production ready/i)).toBeInTheDocument();
    await user.click(within(alert).getByRole('button', { name: 'Resume activation' }));
    const resumedEmail = container.querySelector('input[type="email"]');
    expect(resumedEmail).toBeEnabled();
    expect(screen.getByRole('button', { name: 'Activate' })).toBeInTheDocument();
  });

  it('resumes a partial activation with the same verification session and completed resources', async () => {
    const user = userEvent.setup();
    const fetchMock = vi
      .fn<typeof fetch>()
      .mockResolvedValueOnce(
        new Response(
          JSON.stringify({
            message: 'notification transport is unavailable',
            details: {
              activation_step: 'notification_rule',
              ready_resource_ids: ['primary-provider', 'production-route'],
              retriable: true,
            },
          }),
          { status: 503 },
        ),
      )
      .mockResolvedValueOnce(new Response(JSON.stringify(activationResponse()), { status: 201 }));
    vi.stubGlobal('fetch', fetchMock);
    const { container } = render(
      <ProductionLoopSetup
        workspaceSlug="workspace-1"
        environmentId="production"
        apiBaseUrl="https://guard.example.com"
        agents={[{ id: 'agent-1', name: 'Production agent' }]}
        providerConnections={[]}
        activeRuntimeKeyCount={1}
      />,
    );
    const password = container.querySelector('input[type="password"]');
    const email = container.querySelector('input[type="email"]');
    if (password === null || email === null) throw new Error('activation inputs missing');
    await user.type(password, 'provider-secret');
    await user.type(email, 'ops@example.com');
    const privacyConfirmation = screen.getAllByRole('checkbox')[1];
    if (privacyConfirmation === undefined) throw new Error('privacy confirmation missing');
    await user.click(privacyConfirmation);
    await user.click(screen.getByRole('button', { name: 'Activate' }));

    const pausedTitle = await screen.findByText('Activation paused');
    const paused = pausedTitle.closest('[role="alert"]');
    if (paused === null) throw new Error('activation error alert missing');
    expect(paused).toHaveTextContent('notification_rule');
    expect(paused).toHaveTextContent('primary-provider, production-route');
    await user.click(screen.getByRole('button', { name: 'Resume activation' }));

    await screen.findByText('Send exact verification traffic');
    const requestBodies = fetchMock.mock.calls.map(([, init]) => JSON.parse(String(init?.body)));
    expect(requestBodies).toHaveLength(2);
    expect(requestBodies[0]?.verification_session_id).toBe('verify-fixed');
    expect(requestBodies[1]?.verification_session_id).toBe('verify-fixed');
  });

  it('creates the runtime key in the activation workspace and environment', async () => {
    const user = userEvent.setup();
    const fetchMock = vi
      .fn<typeof fetch>()
      .mockResolvedValueOnce(new Response(JSON.stringify(activationResponse()), { status: 201 }))
      .mockResolvedValueOnce(
        new Response(JSON.stringify(activationResponse().readiness), { status: 200 }),
      );
    vi.stubGlobal('fetch', fetchMock);
    const keyPost = vi.spyOn(http.withoutWorkspace, 'post').mockResolvedValue({
      api_key: { id: 'key-1', name: 'Gateway key', prefix: 'tl_live_' },
      plaintext_key: 'one-time-key',
    });
    const { container } = render(
      <ProductionLoopSetup
        workspaceSlug="workspace-1"
        environmentId="production"
        apiBaseUrl="https://guard.example.com"
        agents={[{ id: 'agent-1', name: 'Production agent' }]}
        providerConnections={[]}
        activeRuntimeKeyCount={0}
      />,
    );
    const password = container.querySelector('input[type="password"]');
    const email = container.querySelector('input[type="email"]');
    if (password === null || email === null) throw new Error('activation inputs missing');
    await user.type(password, 'provider-secret');
    await user.type(email, 'ops@example.com');
    const privacyConfirmation = screen.getAllByRole('checkbox')[1];
    if (privacyConfirmation === undefined) throw new Error('privacy confirmation missing');
    await user.click(privacyConfirmation);
    await user.click(screen.getByRole('button', { name: 'Activate' }));

    await waitFor(() => expect(keyPost).toHaveBeenCalledTimes(1));
    expect(keyPost.mock.calls[0]?.[0]).toBe(
      '/api/api-keys?workspace=workspace-1&environment=production',
    );
    expect(await screen.findByText('one-time-key')).toBeInTheDocument();
    expect(screen.queryByText('provider-secret')).not.toBeInTheDocument();
  });
});
