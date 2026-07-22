import assert from 'node:assert/strict';
import test from 'node:test';

import { DEFAULT_SAFE_REPLY, createGuardedSupportAgent, draftSupportReply } from './agent';

type SubmittedEvent = {
  kind: string;
  principal: { agent_id: string };
  action: { operation: string; parameters: { text: string } };
};

type AuthorizationEffect = 'permit' | 'deny';

function decisionFetch(
  effect: AuthorizationEffect,
  submittedEvents: SubmittedEvent[],
): typeof fetch {
  return async (input, init) => {
    assert.equal(String(input), 'https://guard.test/v1/events');
    assert.equal(init?.method, 'POST');
    assert.equal(typeof init?.body, 'string');
    submittedEvents.push(JSON.parse(String(init?.body)));

    return new Response(
      JSON.stringify({
        trace_id: `trace-${effect}`,
        domain: 'customer_support',
        effect,
        reason: effect === 'permit' ? 'No policy matched.' : 'PII policy matched.',
        findings: [],
        latency_ms: 1,
      }),
      { status: 200, headers: { 'content-type': 'application/json' } },
    );
  };
}

test('submits the agent draft to /v1/events and delivers a permitted reply', async () => {
  const submittedEvents: SubmittedEvent[] = [];
  const reply = createGuardedSupportAgent({
    baseUrl: 'https://guard.test',
    fetchImpl: decisionFetch('permit', submittedEvents),
  });

  const result = await reply('What are your support hours?');

  assert.equal(result, 'Support is available from 9:00 to 17:00 UTC.');
  assert.deepEqual(submittedEvents, [
    {
      kind: 'output.proposed',
      principal: {
        workspace_id: '',
        environment_id: '',
        agent_id: 'cookbook-support-agent',
      },
      action: {
        operation: 'output',
        parameters: { text: 'Support is available from 9:00 to 17:00 UTC.' },
        side_effect: 'none',
      },
      sources: [
        {
          id: 'input',
          origin: 'user',
          labels: {
            trust: 'unknown',
            confidentiality: 'unknown',
            integrity: 'unknown',
          },
        },
      ],
      provenance: { text: ['input'] },
      context: { channel: 'chat', domain: 'customer_support' },
    },
  ]);
});

test('replaces a denied PII draft before the caller can deliver it', async () => {
  const submittedEvents: SubmittedEvent[] = [];
  const reply = createGuardedSupportAgent({
    baseUrl: 'https://guard.test',
    fetchImpl: decisionFetch('deny', submittedEvents),
  });

  const result = await reply('What is the customer SSN?');

  assert.match(draftSupportReply('What is the customer SSN?'), /123-45-6789/);
  assert.equal(result, DEFAULT_SAFE_REPLY);
  assert.doesNotMatch(result, /123-45-6789/);
});

test('fails closed when the guard service cannot be reached', async () => {
  const unavailableFetch: typeof fetch = async () => {
    throw new TypeError('network unavailable');
  };
  const reply = createGuardedSupportAgent({
    baseUrl: 'https://guard.test',
    fetchImpl: unavailableFetch,
    retry: {
      maxAttempts: 1,
      totalBudgetS: 0,
      baseDelayS: 0,
      maxDelayS: 0,
    },
  });

  const result = await reply('What is the customer SSN?');

  assert.equal(result, DEFAULT_SAFE_REPLY);
  assert.doesNotMatch(result, /123-45-6789/);
});

test('attaches local workspace context when the quickstart explicitly supplies it', async () => {
  let submittedHeaders = new Headers();
  const fetchImpl: typeof fetch = async (_input, init) => {
    submittedHeaders = new Headers(init?.headers);
    return new Response(
      JSON.stringify({
        trace_id: 'trace-workspace',
        domain: 'customer_support',
        effect: 'permit',
        reason: 'No policy matched.',
        findings: [],
        latency_ms: 1,
      }),
      { status: 200, headers: { 'content-type': 'application/json' } },
    );
  };
  const reply = createGuardedSupportAgent({
    baseUrl: 'https://guard.test',
    fetchImpl,
    workspaceId: 'cookbook-local',
  });

  await reply('What are your support hours?');

  assert.equal(submittedHeaders.get('x-tlg-workspace-id'), 'cookbook-local');
});
