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

    expect(screen.getByRole('row', { name: /default profile unknown allow/i })).toBeInTheDocument();
  });
});
