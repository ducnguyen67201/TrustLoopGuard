import { cleanup, render, screen, waitFor } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { afterEach, describe, expect, it, vi } from 'vitest';

import { ConsentForm } from './consent-form';

const workspaces = [
  {
    id: 'ws-1',
    name: 'Workspace',
    slug: 'workspace',
    role: 'admin',
  },
];

describe('hosted MCP consent', () => {
  afterEach(() => {
    cleanup();
    vi.unstubAllGlobals();
  });

  it('binds authorization to the selected registered agent', async () => {
    const fetchMock = vi.fn<typeof fetch>().mockImplementation(async (_input, init) => {
      if (init?.method === 'POST') {
        return new Response(JSON.stringify({ error: 'test stop' }), { status: 400 });
      }
      return new Response(
        JSON.stringify({
          agents: [
            {
              agent_id: 'agent-customer',
              display_name: 'Customer agent',
              scope: { in_scope: [], out_of_scope: [] },
              authority: {},
              tone: {},
              knowledge_sources: [],
              escalation_triggers: [],
              workflow_requirements: [],
            },
          ],
        }),
        { status: 200 },
      );
    });
    vi.stubGlobal('fetch', fetchMock);
    render(
      <ConsentForm
        clientId="client"
        redirectUri="https://client.example/callback"
        state="state"
        codeChallenge="challenge"
        workspaces={workspaces}
        userLabel="member@example.com"
      />,
    );

    expect(await screen.findByRole('option', { name: 'Customer agent' })).toBeInTheDocument();
    await userEvent.click(screen.getByRole('button', { name: 'Authorize' }));

    await waitFor(() => expect(fetchMock).toHaveBeenCalledTimes(2));
    const authorizationCall = fetchMock.mock.calls[1];
    expect(authorizationCall).toBeDefined();
    const [, init] = authorizationCall!;
    expect(JSON.parse(String(init?.body))).toMatchObject({
      workspaceId: 'ws-1',
      agentId: 'agent-customer',
    });
  });

  it('does not authorize a workspace without a registered agent', async () => {
    vi.stubGlobal(
      'fetch',
      vi
        .fn<typeof fetch>()
        .mockResolvedValue(new Response(JSON.stringify({ agents: [] }), { status: 200 })),
    );
    render(
      <ConsentForm
        clientId="client"
        redirectUri="https://client.example/callback"
        state="state"
        codeChallenge="challenge"
        workspaces={workspaces}
        userLabel="member@example.com"
      />,
    );

    expect(await screen.findByText(/register an agent/i)).toBeInTheDocument();
    expect(screen.getByRole('button', { name: 'Authorize' })).toBeDisabled();
  });
});
