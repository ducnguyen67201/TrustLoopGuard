import { render, screen } from '@testing-library/react';
import { describe, expect, it, vi } from 'vitest';

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
});
