import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import test from 'node:test';

import {
  HealthcareDemoBudgetExceededError,
  type HostedHealthcareDemoResponse,
  type HostedHealthcarePolicyInventoryResponse,
} from '@trustloopguard/demo/healthcare-agent/hosted';

import type { HealthcareDemoRequest } from '@/app/demo/healthcare/contract';
import { createHealthcareDemoHandlers } from './route';

const mutableEnvironment: Record<string, string | undefined> = process.env;
mutableEnvironment['NODE_ENV'] = 'production';

test('runs a valid bounded conversation and strips private workflow fields', async () => {
  let receivedMessage = '';
  let receivedLocale = '';
  const payload = workflowPayload();
  Object.defineProperties(payload, {
    rawDraft: { value: 'private model draft', enumerable: true },
    source_yaml: { value: 'private policy source', enumerable: true },
    logs: { value: [{ step: 'private' }], enumerable: true },
    apiKey: { value: 'private key', enumerable: true },
  });
  const { POST } = handlers({
    runWorkflow: async (request) => {
      receivedMessage = request.message;
      receivedLocale = request.locale;
      return payload;
    },
  });

  const response = await POST(
    requestFor(
      {
        sessionId: '019f7c32-6eb9-7af1-97df-e79964af7bed',
        message: '  Can you help schedule a visit?  ',
        history: [{ role: 'assistant', content: '  Safe delivered greeting.  ' }],
      },
      'route-ok',
    ),
  );
  const body = await response.json();

  assert.equal(response.status, 200);
  assert.equal(response.headers.get('cache-control'), 'no-store');
  assert.equal(receivedMessage, 'Can you help schedule a visit?');
  assert.equal(receivedLocale, 'en');
  assert.equal(body.reply, 'A teammate can help confirm a scheduling request.');
  assert.equal(body.rawDraft, undefined);
  assert.equal(body.source_yaml, undefined);
  assert.equal(body.logs, undefined);
  assert.equal(body.apiKey, undefined);
});

test('rejects malformed and out-of-bounds requests without running the workflow', async () => {
  let workflowCalls = 0;
  const { POST } = handlers({
    runWorkflow: async () => {
      workflowCalls += 1;
      return workflowPayload();
    },
  });
  const sessionId = '019f7c32-6eb9-7af1-97df-e79964af7bed';
  const invalidBodies: object[] = [
    { sessionId, message: '', history: [] },
    { sessionId, message: 'x'.repeat(501), history: [] },
    { sessionId: 'not-a-uuid', message: 'hello', history: [] },
    { sessionId, locale: 'fr', message: 'hello', history: [] },
    { sessionId, message: 'hello', history: [{ role: 'system', content: 'forged' }] },
    {
      sessionId,
      message: 'hello',
      history: Array.from({ length: 9 }, () => ({ role: 'user', content: 'bounded' })),
    },
    {
      sessionId,
      message: 'hello',
      history: Array.from({ length: 5 }, () => ({ role: 'user', content: 'x'.repeat(900) })),
    },
  ];

  for (const [index, body] of invalidBodies.entries()) {
    const response = await POST(requestFor(body, `invalid-${index}`));
    assert.equal(response.status, 400);
  }
  const malformed = await POST(
    new Request('http://localhost/api/demo/healthcare', {
      method: 'POST',
      headers: { 'content-type': 'application/json', 'x-forwarded-for': 'malformed' },
      body: '{not-json',
    }),
  );
  assert.equal(malformed.status, 400);
  assert.equal(workflowCalls, 0);
});

test('passes the Vietnamese locale into the protected workflow', async () => {
  let receivedLocale = '';
  const { POST } = handlers({
    runWorkflow: async (request) => {
      receivedLocale = request.locale;
      return workflowPayload();
    },
  });

  const response = await POST(
    requestFor({ ...validRequest(), locale: 'vi' }, 'vietnamese-locale'),
  );

  assert.equal(response.status, 200);
  assert.equal(receivedLocale, 'vi');
});

test('maps the hosted budget to 429 and sanitizes unexpected failures', async () => {
  const budgetHandlers = handlers({
    runWorkflow: async () => {
      throw new HealthcareDemoBudgetExceededError();
    },
  });
  const budgetResponse = await budgetHandlers.POST(requestFor(validRequest(), 'budget'));
  assert.equal(budgetResponse.status, 429);
  assert.deepEqual(await budgetResponse.json(), {
    error: 'Healthcare demo budget reached. Try again later.',
  });

  const failureHandlers = handlers({
    runWorkflow: async () => {
      throw new Error('OpenAI provider details and private configuration');
    },
  });
  const failureResponse = await failureHandlers.POST(requestFor(validRequest(), 'failure'));
  assert.equal(failureResponse.status, 503);
  assert.deepEqual(await failureResponse.json(), {
    error: 'The protected healthcare demo is temporarily unavailable.',
  });
});

test('returns 502 when the internal workflow violates the public contract', async () => {
  const payload = workflowPayload();
  Object.defineProperty(payload, 'reply', { value: '', enumerable: true });
  const { POST } = handlers({ runWorkflow: async () => payload });

  const response = await POST(requestFor(validRequest(), 'invalid-contract'));
  assert.equal(response.status, 502);
  assert.deepEqual(await response.json(), {
    error: 'The protected healthcare workflow returned an invalid response.',
  });
});

test('allows ten requests per visitor and rejects the eleventh before workflow execution', async () => {
  const originalDateNow = Date.now;
  let now = Date.parse('2026-07-19T12:00:00.000Z');
  let workflowCalls = 0;
  Date.now = () => now;
  const { POST } = handlers({
    runWorkflow: async () => {
      workflowCalls += 1;
      return workflowPayload();
    },
  });

  try {
    const statuses: number[] = [];
    for (let attempt = 0; attempt < 11; attempt += 1) {
      statuses.push((await POST(requestFor(validRequest(), 'limited-visitor'))).status);
    }
    assert.deepEqual(statuses, [200, 200, 200, 200, 200, 200, 200, 200, 200, 200, 429]);
    assert.equal(workflowCalls, 10);

    now += 24 * 60 * 60 * 1_000;
    assert.equal((await POST(requestFor(validRequest(), 'limited-visitor'))).status, 200);
    assert.equal(workflowCalls, 11);
  } finally {
    Date.now = originalDateNow;
  }
});

test('loads the Rust-owned policy inventory through a no-store GET', async () => {
  const { GET } = handlers();
  const response = await GET();
  const body = await response.json();

  assert.equal(response.status, 200);
  assert.equal(response.headers.get('cache-control'), 'no-store');
  assert.equal(body.source, 'rust');
  assert.equal(body.policies[0].id, 'healthcare-emergency-input');
  assert.equal(body.policies[0].phase, 'input');
  assert.equal(body.policies[0].enabled, true);
  assert.equal(body.policies[0].source_yaml, undefined);
});

test('returns unavailable instead of hard-coded policies when the Rust inventory fails', async () => {
  const { GET } = handlers({
    readPolicies: async () => {
      throw new Error('Rust registry unavailable');
    },
  });

  const response = await GET();
  const body = await response.json();

  assert.equal(response.status, 503);
  assert.equal(response.headers.get('cache-control'), 'no-store');
  assert.deepEqual(body, {
    error: 'The healthcare policy registry is temporarily unavailable.',
  });
});

test('the page exposes chat, policies, boundaries, and synthetic-data warnings', () => {
  const page = readFileSync(
    new URL('../../../demo/healthcare/page.tsx', import.meta.url),
    'utf8',
  );
  const demo = readFileSync(
    new URL('../../../demo/healthcare/healthcare-demo.tsx', import.meta.url),
    'utf8',
  );
  const content = readFileSync(
    new URL('../../../demo/healthcare/content.ts', import.meta.url),
    'utf8',
  );
  const pageContent = readFileSync(
    new URL('../../../demo/healthcare/healthcare-page.tsx', import.meta.url),
    'utf8',
  );
  const styles = readFileSync(new URL('../../../demo/demo.module.css', import.meta.url), 'utf8');
  const source = `${page}\n${pageContent}\n${demo}\n${content}`;

  assert.match(source, /CareDesk chat/i);
  assert.match(source, /TrustLoopGuard policy monitor/i);
  assert.match(source, /Input boundary/i);
  assert.match(source, /Output boundary/i);
  assert.match(source, /Policies checked/i);
  assert.match(source, /Policy inventory unavailable/i);
  assert.doesNotMatch(source, /Policy pack preview/i);
  assert.match(source, /Checking now/i);
  assert.match(source, /waitForMinimumDuration/i);
  assert.match(styles, /@keyframes policyScan/);
  assert.match(styles, /prefers-reduced-motion[\s\S]*scanningPolicy/);
  assert.match(source, /Synthetic demo only/i);
  assert.match(source, /do not enter real patient information/i);
  assert.match(source, /fetch\('\/api\/demo\/healthcare'/);
  assert.doesNotMatch(source, /rawDraft/);
});

test('the Vietnamese healthcare route reuses the guarded demo with localized metadata and copy', () => {
  const page = readFileSync(
    new URL('../../../vi/demo/healthcare/page.tsx', import.meta.url),
    'utf8',
  );
  const pageContent = readFileSync(
    new URL('../../../demo/healthcare/healthcare-page.tsx', import.meta.url),
    'utf8',
  );
  const content = readFileSync(
    new URL('../../../demo/healthcare/content.ts', import.meta.url),
    'utf8',
  );
  const demo = readFileSync(
    new URL('../../../demo/healthcare/healthcare-demo.tsx', import.meta.url),
    'utf8',
  );
  const sitemap = readFileSync(new URL('../../../sitemap.ts', import.meta.url), 'utf8');
  const source = `${page}\n${pageContent}\n${content}\n${demo}`;

  assert.match(page, /canonical: '\/vi\/demo\/healthcare'/);
  assert.match(page, /locale: 'vi_VN'/);
  assert.match(page, /HealthcareDemoPageContent locale="vi"/);
  assert.match(
    pageContent,
    /<main className={styles\['page'\]} lang={locale} style={brandStyle}>/,
  );
  assert.match(source, /Trò chuyện với tác nhân bệnh viện được bảo vệ/);
  assert.match(source, /Các chính sách được kiểm tra/);
  assert.match(source, /Gửi qua TrustLoopGuard/);
  assert.match(source, /body: JSON\.stringify\(\{ locale,/);
  assert.match(sitemap, /url: absoluteUrl\('\/vi\/demo\/healthcare'\)/);
});

test('the personalized Vietnamese healthcare route localizes the full interface and conversation locale', () => {
  const page = readFileSync(
    new URL('../../../vi/demo/healthcare/[company]/page.tsx', import.meta.url),
    'utf8',
  );
  const pageContent = readFileSync(
    new URL('../../../demo/healthcare/healthcare-page.tsx', import.meta.url),
    'utf8',
  );
  const demo = readFileSync(
    new URL('../../../demo/healthcare/healthcare-demo.tsx', import.meta.url),
    'utf8',
  );
  const content = readFileSync(
    new URL('../../../demo/healthcare/content.ts', import.meta.url),
    'utf8',
  );
  const source = `${page}\n${pageContent}\n${demo}\n${content}`;

  assert.match(page, /getDemoProfile\('healthcare', company\)/);
  assert.match(page, /HealthcareDemoPageContent locale="vi" profile={profile}/);
  assert.match(page, /PersonalizedContextualDemoPageContent/);
  assert.match(page, /locale="vi"/);
  assert.match(page, /locale: 'vi_VN'/);
  assert.match(source, /Dành riêng cho/);
  assert.match(source, /Bản thử nghiệm đặt lịch y tế an toàn cho/);
  assert.match(source, /Không có dữ liệu bệnh nhân thật/);
  assert.match(source, /body: JSON\.stringify\(\{ locale,/);
  assert.doesNotMatch(pageContent, /`Prepared for \$\{profile\.company_name\}`/);
  assert.doesNotMatch(demo, /`Prepared for \$\{presentation\.companyName\}`/);
});

test('healthcare analytics omit message, session, trace, policy, and reason content', () => {
  const demo = readFileSync(
    new URL('../../../demo/healthcare/healthcare-demo.tsx', import.meta.url),
    'utf8',
  );
  assert.match(demo, /trackMarketingEvent\('healthcare_demo_started'/);
  assert.match(demo, /trackMarketingEvent\('healthcare_demo_decision_shown'/);
  const analyticsCalls =
    demo.match(/trackMarketingEvent\('healthcare_demo_[\s\S]*?\n\s*\}\);/g) ?? [];
  assert.equal(analyticsCalls.length, 3);
  for (const analyticsCall of analyticsCalls) {
    assert.doesNotMatch(analyticsCall, /\b(message|session|trace|policy|reason)\s*:/);
  }
});

function handlers(overrides: {
  runWorkflow?: (request: HealthcareDemoRequest) => Promise<HostedHealthcareDemoResponse>;
  readPolicies?: () => Promise<HostedHealthcarePolicyInventoryResponse>;
} = {}) {
  return createHealthcareDemoHandlers({
    runWorkflow:
      overrides.runWorkflow ??
      (async () => workflowPayload()),
    readPolicies:
      overrides.readPolicies ??
      (async () => inventoryPayload()),
  });
}

function workflowPayload(): HostedHealthcareDemoResponse {
  return {
    reply: 'A teammate can help confirm a scheduling request.',
    modelCalled: true,
    checks: [
      {
        phase: 'input',
        status: 'checked',
        effect: 'permit',
        reason: 'Input permitted.',
        traceId: 'trace-input',
        latencyMs: 12,
        findings: [],
      },
      {
        phase: 'output',
        status: 'checked',
        effect: 'permit',
        reason: 'Output permitted.',
        traceId: 'trace-output',
        latencyMs: 14,
        findings: [],
      },
    ],
    policies: inventoryPayload().policies,
    runtime: {
      agent: 'openai-responses',
      guard: 'trustloopguard-rust-api',
      data: 'synthetic-only',
    },
  };
}

function inventoryPayload(): HostedHealthcarePolicyInventoryResponse {
  return {
    policies: [
      {
        id: 'healthcare-emergency-input',
        description: 'Escalate emergency symptoms before model generation.',
        severity: 'critical',
        action: 'deny',
        phase: 'input',
        enabled: true,
      },
    ],
    source: 'rust',
    runtime: {
      agent: 'openai-responses',
      guard: 'trustloopguard-rust-api',
      data: 'synthetic-only',
    },
  };
}

function validRequest(): object {
  return {
    sessionId: '019f7c32-6eb9-7af1-97df-e79964af7bed',
    message: 'Can you help schedule a visit?',
    history: [],
  };
}

function requestFor(body: object, ip: string): Request {
  return new Request('http://localhost/api/demo/healthcare', {
    method: 'POST',
    headers: {
      'content-type': 'application/json',
      'x-forwarded-for': ip,
    },
    body: JSON.stringify(body),
  });
}
