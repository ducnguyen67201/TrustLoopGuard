import { render, screen } from '@testing-library/react';
import { describe, expect, it, vi } from 'vitest';
import type { EnforcementProfile } from '@trustloopguard/sdk';

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
  it('renders partial gateway profile data without crashing', () => {
    const profile = {
      id: 'profile_1',
      display_name: 'Default profile',
      output_action: 'allow',
      fail_mode: 'closed',
      retention_mode: 'metadata_only',
      response_mode: 'regular',
      fallback_message: 'Blocked',
      max_regenerations: 1,
      created_at: '2026-05-30T00:00:00Z',
      updated_at: '2026-05-30T00:00:00Z',
    } as EnforcementProfile;

    render(
      <GatewayPageContent
        apiBaseUrl="http://localhost:3001"
        data={{
          ...shell,
          providerConnections: [],
          enforcementProfiles: [profile],
          gatewayRoutes: [],
          activeRuntimeKeyCount: 1,
        }}
      />,
    );

    // The row shows friendly, non-technical wording: a missing input action reads
    // "Unknown", `allow` reads "Allow", and `closed` becomes "Block when unsure".
    expect(
      screen.getByRole('row', { name: /default profile unknown allow block when unsure/i }),
    ).toBeInTheDocument();

    // The page header carries the plain-language gateway explanation.
    expect(
      screen.getByRole('heading', { level: 1, name: /gateway/i }),
    ).toBeInTheDocument();
  });

  it('guides a non-technical user with plain-language empty and warning states', () => {
    render(
      <GatewayPageContent
        apiBaseUrl="http://localhost:3001"
        data={{
          ...shell,
          providerConnections: [],
          enforcementProfiles: [],
          gatewayRoutes: [],
          activeRuntimeKeyCount: 0,
        }}
      />,
    );

    // With nothing set up, the default Routes tab explains the next step in plain words.
    expect(screen.getByText(/no routes set up yet/i)).toBeInTheDocument();

    // Missing API key warning is phrased for a non-technical reader.
    expect(screen.getByText(/you need an api key first/i)).toBeInTheDocument();
  });
});
