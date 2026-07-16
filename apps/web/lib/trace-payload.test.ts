import { describe, expect, it } from 'vitest';

import { readTraceAgent } from './trace-payload';

describe('readTraceAgent', () => {
  it('reads legacy top-level agent ids', () => {
    expect(readTraceAgent({ agent_id: 'legacy-agent' })).toBe('legacy-agent');
  });

  it('reads canonical event principal agent ids', () => {
    expect(
      readTraceAgent({
        event: {
          principal: {
            agent_id: 'decorated-agent',
          },
        },
      }),
    ).toBe('decorated-agent');
  });

  it('uses the runtime fallback when no agent id is recorded', () => {
    expect(readTraceAgent({})).toBe('Runtime agent');
  });
});
