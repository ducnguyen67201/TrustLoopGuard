import { cleanup, render, screen, waitFor } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { afterEach, describe, expect, it, vi } from 'vitest';

import { GatewayPageContent } from './GatewayPageContent';

vi.mock('next/navigation', () => ({
  useRouter: () => ({ refresh: vi.fn() }),
}));

type GatewayPageContentData = Parameters<typeof GatewayPageContent>[0]['data'];

const shell: Pick<
  GatewayPageContentData,
  | 'user'
  | 'organization'
  | 'activeWorkspace'
  | 'workspaces'
  | 'activeEnvironment'
  | 'environments'
  | 'agents'
> = {
  user: {
    name: 'Duc',
    email: 'duc@example.com',
    avatar: '',
  },
  organization: {
    id: 'org_1',
    name: 'TrustLoop',
    slug: 'trustloop',
  },
  activeWorkspace: {
    id: 'ws_1',
    name: 'Proxy Demo',
    slug: 'proxy-demo',
    description: '',
    policyCount: 0,
    enabledPolicies: 0,
    agentCount: 0,
    sourceCount: 0,
    apiKeyCount: 0,
    role: 'admin',
    isKnowledgeBaseEnabled: false,
    isAttacksEnabled: false,
  },
  workspaces: [],
  activeEnvironment: {
    id: 'production',
    slug: 'production',
    name: 'Production',
    description: null,
    isDefault: true,
  },
  environments: [],
  agents: [],
};

describe('GatewayPageContent', () => {
  afterEach(() => {
    cleanup();
    vi.unstubAllGlobals();
  });

  it('shows the policy-authoritative two-resource setup', () => {
    render(
      <GatewayPageContent
        apiBaseUrl="http://localhost:3001"
        data={{
          ...shell,
          providerConnections: [],
          gatewayRoutes: [],
          activeRuntimeKeyCount: 1,
        }}
      />,
    );
    expect(screen.getByRole('heading', { level: 1, name: /gateway/i })).toBeInTheDocument();
    expect(screen.getByRole('tab', { name: /routes/i })).toBeInTheDocument();
    expect(screen.getByRole('tab', { name: /providers/i })).toBeInTheDocument();
    expect(screen.queryByRole('tab', { name: /rule sets/i })).not.toBeInTheDocument();
    expect(screen.queryByRole('tab', { name: /connect your app/i })).not.toBeInTheDocument();
    expect(screen.getByText(/enabled policies apply automatically/i)).toBeInTheDocument();
  });

  it('guides a non-technical user with plain-language empty and warning states', () => {
    render(
      <GatewayPageContent
        apiBaseUrl="http://localhost:3001"
        data={{
          ...shell,
          providerConnections: [],
          gatewayRoutes: [],
          activeRuntimeKeyCount: 0,
        }}
      />,
    );

    // With nothing set up, the default Routes tab explains the next step in plain words.
    expect(screen.getAllByText(/no routes set up yet/i).length).toBeGreaterThan(0);

    // Missing API key warning is phrased for a non-technical reader.
    expect(screen.getByText(/you need an api key first/i)).toBeInTheDocument();
  });

  it('links route setup to LLM budget readiness', () => {
    render(
      <GatewayPageContent
        apiBaseUrl="http://localhost:3001"
        data={{
          ...shell,
          providerConnections: [],
          gatewayRoutes: [],
          activeRuntimeKeyCount: 1,
        }}
        budgetReadiness={{ hasPrice: true, hasCap: true, hasAlert: false }}
      />,
    );

    expect(screen.getByText('Model price ready')).toBeInTheDocument();
    expect(screen.getByText('Hard cap ready')).toBeInTheDocument();
    expect(screen.getByText('80% alert not configured')).toBeInTheDocument();
    expect(screen.getByRole('link', { name: /configure usage & budgets/i })).toHaveAttribute(
      'href',
      '/usage?workspace=proxy-demo&environment=production#budgets',
    );
  });

  it('edits a provider without requiring the existing secret', async () => {
    const user = userEvent.setup();
    const fetchMock = vi.fn<typeof fetch>().mockResolvedValue(new Response(null, { status: 200 }));
    vi.stubGlobal('fetch', fetchMock);

    render(
      <GatewayPageContent
        apiBaseUrl="http://localhost:3001"
        data={{
          ...shell,
          providerConnections: [
            {
              id: 'provider-1',
              display_name: 'OpenAI production',
              kind: 'openai_compatible',
              base_url: 'https://api.openai.com',
              default_model: 'gpt-4o-mini',
              credential_status: 'configured',
              created_at: '2026-07-10T00:00:00Z',
              updated_at: '2026-07-10T00:00:00Z',
            },
          ],
          gatewayRoutes: [],
          activeRuntimeKeyCount: 1,
        }}
      />,
    );

    await user.click(screen.getByRole('button', { name: 'Edit OpenAI production' }));
    const name = screen.getByRole('textbox', { name: 'Name' });
    await user.clear(name);
    await user.type(name, 'OpenAI primary');
    await user.click(screen.getByRole('button', { name: 'Save changes' }));

    await waitFor(() => {
      expect(fetchMock).toHaveBeenCalledWith(
        '/api/gateway/provider-connections/provider-1?workspace=proxy-demo',
        expect.objectContaining({
          method: 'PATCH',
          body: JSON.stringify({
            display_name: 'OpenAI primary',
            base_url: 'https://api.openai.com',
            default_model: 'gpt-4o-mini',
          }),
        }),
      );
    });
    await waitFor(() => {
      expect(screen.queryByRole('dialog', { name: 'Edit provider' })).not.toBeInTheDocument();
    });
  });

  it('requires confirmation before permanently deleting a provider', async () => {
    const user = userEvent.setup();
    const fetchMock = vi.fn<typeof fetch>().mockResolvedValue(new Response(null, { status: 204 }));
    vi.stubGlobal('fetch', fetchMock);

    render(
      <GatewayPageContent
        apiBaseUrl="http://localhost:3001"
        data={{
          ...shell,
          providerConnections: [
            {
              id: 'provider-1',
              display_name: 'OpenAI production',
              kind: 'openai_compatible',
              base_url: null,
              default_model: 'gpt-4o-mini',
              credential_status: 'configured',
              created_at: '2026-07-10T00:00:00Z',
              updated_at: '2026-07-10T00:00:00Z',
            },
          ],
          gatewayRoutes: [],
          activeRuntimeKeyCount: 1,
        }}
      />,
    );

    await user.click(screen.getByRole('button', { name: 'Delete OpenAI production' }));
    expect(screen.getByRole('alertdialog')).toHaveTextContent(/permanently delete/i);
    expect(fetchMock).not.toHaveBeenCalled();

    await user.click(screen.getByRole('button', { name: 'Delete provider' }));
    expect(fetchMock).toHaveBeenCalledWith(
      '/api/gateway/provider-connections/provider-1?workspace=proxy-demo',
      { method: 'DELETE' },
    );
  });
});
