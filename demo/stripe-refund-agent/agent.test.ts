import assert from 'node:assert/strict';
import test from 'node:test';

import { runRefundAgent } from './agent';
import type { RefundAgentClient } from './core';

test('live demo mode never falls back to the scripted agent', async () => {
  const client = {} as RefundAgentClient;
  const options = { useOpenAI: false, requireLiveAgent: true };

  await assert.rejects(
    runRefundAgent('Refund order ord_missing for $75 because it arrived damaged.', client, options),
    /live agent/i,
  );
});
