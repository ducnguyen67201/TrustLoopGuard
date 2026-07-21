import { cleanup, fireEvent, render, screen, waitFor } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { useState } from 'react';
import { afterEach, describe, expect, it, vi } from 'vitest';

import { PolicyBuilderEditor } from './PolicyBuilderEditor';

afterEach(cleanup);

const CHAT_POLICY = `id: deny-northwind-disclosure
description: Prevent disclosure of Northwind Labs records.
when:
  channels: [chat]
match:
  regex: Northwind Labs
action: deny
severity: medium
owner_agent_id: policy-test-agent
`;

function Harness({ onYamlChange }: { onYamlChange: (yaml: string) => void }) {
  const [yaml, setYaml] = useState(CHAT_POLICY);
  return (
    <PolicyBuilderEditor
      yaml={yaml}
      onYamlChange={(next) => {
        setYaml(next);
        onYamlChange(next);
      }}
    />
  );
}

describe('PolicyBuilderEditor', () => {
  it('makes a chat policy cover hosted MCP without losing its assistant scope', async () => {
    const user = userEvent.setup();
    const onYamlChange = vi.fn();
    render(<Harness onYamlChange={onYamlChange} />);

    const hostedMcp = screen.getByRole('switch', {
      name: 'Include hosted MCP tool calls',
    });
    expect(hostedMcp).not.toBeChecked();

    await user.click(hostedMcp);

    await waitFor(() => expect(hostedMcp).toBeChecked());
    const yaml = onYamlChange.mock.lastCall?.[0];
    expect(yaml).toContain('when:\n  agents: [policy-test-agent]');
    expect(yaml).not.toContain('channels:');
    expect(yaml).toContain('owner_agent_id: policy-test-agent');
  });

  it('edits the runtime assistant scope from the guided field', () => {
    const onYamlChange = vi.fn();
    render(<PolicyBuilderEditor yaml={CHAT_POLICY} onYamlChange={onYamlChange} />);

    const assistant = screen.getByLabelText('Applies to one assistant');
    expect(assistant).toHaveValue('policy-test-agent');
    expect(assistant).toBeEnabled();

    fireEvent.change(assistant, { target: { value: 'replacement-agent' } });

    expect(onYamlChange).toHaveBeenCalledOnce();
    const yaml = onYamlChange.mock.lastCall?.[0];
    expect(yaml).toContain('agents: [replacement-agent]');
    expect(yaml).toContain('owner_agent_id: replacement-agent');
  });
});
