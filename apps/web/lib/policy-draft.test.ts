import { describe, expect, it } from 'vitest';

import { draftToYaml, yamlToDraft } from './policy-draft';

describe('policy draft YAML mapping', () => {
  it('parses common policy YAML into builder fields', () => {
    const result = yamlToDraft(`
id: demo-proxy-rude-output-0968f70b
description: "Block rude assistant replies."
severity: medium
when:
  channels: [chat]
  domains: [gateway_output_check]
match:
  regex: "(?i)\\\\b(stupid question|figure it out yourself)\\\\b"
action: block
owner_agent_id: demo-proxy-agent-0968f70b
`);

    expect(result.ok).toBe(true);
    if (!result.ok) return;
    expect(result.draft).toMatchObject({
      id: 'demo-proxy-rude-output-0968f70b',
      description: 'Block rude assistant replies.',
      severity: 'medium',
      channels: ['chat'],
      domains: ['gateway_output_check'],
      matchType: 'regex',
      matchValue: '(?i)\\b(stupid question|figure it out yourself)\\b',
      action: 'block',
      ownerAgentId: 'demo-proxy-agent-0968f70b',
    });
  });

  it('round-trips builder-supported fields to YAML', () => {
    const yaml = draftToYaml({
      id: 'no-sensitive-id-output',
      description: 'Block sensitive identifiers in assistant replies.',
      severity: 'critical',
      channels: ['chat'],
      domains: ['gateway_output_check'],
      matchType: 'regex',
      matchValue: '\\b\\d{3}-\\d{2}-\\d{4}\\b',
      action: 'block',
      ownerAgentId: 'agent-1',
    });

    expect(yaml).toContain('when:\n  channels: [chat]\n  domains: [gateway_output_check]');
    expect(yaml).toContain('match:\n  regex: "\\\\b\\\\d{3}-\\\\d{2}-\\\\d{4}\\\\b"');
    expect(yaml).toContain('owner_agent_id: agent-1');
  });

  it('marks unsupported match shapes as advanced YAML only', () => {
    const result = yamlToDraft(`
id: complex-policy
description: Uses nested matching.
match:
  any:
    - literal: one
    - literal: two
action: block
`);

    expect(result).toMatchObject({
      ok: false,
      reason: expect.stringContaining('match.literal or match.regex'),
    });
  });
});
