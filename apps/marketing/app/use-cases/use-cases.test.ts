import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import test from 'node:test';
import {
  getUseCase,
  USE_CASE_MENU_CLOSE_DELAY_MS,
  USE_CASE_NAV_GROUPS,
  USE_CASE_NAV_ITEMS,
  USE_CASES,
} from './content';

test('the use-cases page presents the six supported customer workflows', () => {
  assert.deepEqual(
    USE_CASES.map((useCase) => useCase.slug),
    [
      'shell-command-safety',
      'email',
      'agent-spending-caps',
      'ai-inference-spend',
      'x402-payments',
      'action-authorization',
    ],
  );
});

test('every use case has its own canonical detail route', () => {
  assert.deepEqual(
    USE_CASES.map((useCase) => useCase.href),
    [
      '/use-cases/shell-command-safety',
      '/use-cases/email',
      '/use-cases/agent-spending-caps',
      '/use-cases/ai-inference-spend',
      '/use-cases/x402-payments',
      '/use-cases/action-authorization',
    ],
  );
});

test('detail routes resolve only supported use-case slugs', () => {
  assert.equal(
    getUseCase('x402-payments')?.title,
    'Authorize the purchase before the agent signs.',
  );
  assert.equal(
    getUseCase('shell-command-safety')?.title,
    'Stop dangerous shell commands before they run.',
  );
  assert.equal(getUseCase('email')?.title, 'Rewrite risky emails before they send.');
  assert.equal(
    getUseCase('agent-spending-caps')?.title,
    'Enforce agent spending caps before payment.',
  );
  assert.equal(getUseCase('not-a-use-case'), undefined);
});

test('the navigation dropdown exposes the overview and every detail page', () => {
  assert.deepEqual(USE_CASE_NAV_ITEMS, [
    { href: '/use-cases', label: 'All use cases', detail: 'Choose a control boundary' },
    {
      href: '/use-cases/shell-command-safety',
      label: 'Shell command safety',
      detail: 'Deny or approve before execution',
    },
    {
      href: '/use-cases/email',
      label: 'Outbound email',
      detail: 'Permit or rewrite before send',
    },
    {
      href: '/use-cases/agent-spending-caps',
      label: 'Agent spending caps',
      detail: 'Permit, hold, or deny payment',
    },
    {
      href: '/use-cases/ai-inference-spend',
      label: 'AI inference spend',
      detail: 'Meter, alert, and hard cap',
    },
    {
      href: '/use-cases/x402-payments',
      label: 'x402 agent payments',
      detail: 'Authorize before wallet signing',
    },
    {
      href: '/use-cases/action-authorization',
      label: 'Action authorization',
      detail: 'Guard the one-way door',
    },
  ]);
});

test('the navigation mega-menu separates the overview from its six use-case cards', () => {
  assert.deepEqual(USE_CASE_NAV_GROUPS.overview, USE_CASE_NAV_ITEMS[0]);
  assert.deepEqual(
    USE_CASE_NAV_GROUPS.details.map((item) => item.href),
    [
      '/use-cases/shell-command-safety',
      '/use-cases/email',
      '/use-cases/agent-spending-caps',
      '/use-cases/ai-inference-spend',
      '/use-cases/x402-payments',
      '/use-cases/action-authorization',
    ],
  );
});

test('email control evaluates the proposed message and leaves delivery to the customer app', () => {
  const email = getUseCase('email');

  assert.equal(email?.href, '/use-cases/email');
  assert.match(email?.control ?? '', /before the customer mailer/i);
  assert.match(JSON.stringify(email?.checks), /channel/i);
  assert.match(JSON.stringify(email?.checks), /wording/i);
  assert.match(email?.resultDetail ?? '', /never sends/i);
});

test('README use cases carry native decision-flow demos into marketing', () => {
  const walkthroughs = [
    getUseCase('shell-command-safety'),
    getUseCase('email'),
    getUseCase('agent-spending-caps'),
  ];

  assert.deepEqual(
    walkthroughs.map((useCase) => useCase?.demo?.kind),
    ['shell', 'email', 'spend'],
  );

  for (const useCase of walkthroughs) {
    assert.equal(useCase?.demo?.proposalFields.length, 2);
    assert.equal(useCase?.demo?.policyFields.length, 3);
    assert.ok((useCase?.demo?.decisions.length ?? 0) >= 2);
    assert.match(useCase?.demo?.boundary ?? '', /never (invokes|executes|sends)/i);
  }
});

test('the landing-page walkthrough is native UI rather than a static image', () => {
  const flow = readFileSync(
    new URL('../../components/use-case-flow-demo.tsx', import.meta.url),
    'utf8',
  );
  const showcase = readFileSync(
    new URL('../../components/use-case-showcase.tsx', import.meta.url),
    'utf8',
  );

  assert.match(flow, /Proposed action/);
  assert.match(flow, /Policy check/);
  assert.match(flow, /Decision/);
  assert.match(flow, /Execution/);
  assert.doesNotMatch(flow, /<img/);
  assert.match(showcase, /role="tablist"/);
  assert.match(showcase, /role="tabpanel"/);
});

test('the use-case trigger spans the header so the pointer can reach the mega-menu', () => {
  const styles = readFileSync(new URL('../globals.css', import.meta.url), 'utf8');
  const triggerRule = styles.match(/\.site-nav-dropdown-trigger\s*\{([^}]+)\}/)?.[1] ?? '';

  assert.match(triggerRule, /height:\s*4\.5rem/);
});

test('the mega-menu keeps a short grace period while the pointer moves into the panel', () => {
  assert.ok(USE_CASE_MENU_CLOSE_DELAY_MS >= 150);
  assert.ok(USE_CASE_MENU_CLOSE_DELAY_MS <= 300);
});

test('the use-case trigger uses a dedicated centered chevron instead of a text glyph', () => {
  const component = readFileSync(
    new URL('../../components/use-case-nav.tsx', import.meta.url),
    'utf8',
  );

  assert.match(component, /className="site-nav-dropdown-chevron"/);
  assert.doesNotMatch(component, /⌄/);
});

test('every use case explains the trigger, control flow, and concrete result', () => {
  for (const useCase of USE_CASES) {
    assert.ok(useCase.trigger.length > 0, `${useCase.slug} needs a trigger`);
    assert.ok(useCase.steps.length >= 3, `${useCase.slug} needs a clear workflow`);
    assert.ok(useCase.checks.length >= 3, `${useCase.slug} needs explicit controls`);
    assert.ok(useCase.result.length > 0, `${useCase.slug} needs a result`);
  }
});

test('product claims stay inside the currently supported control surface', () => {
  const pageCopy = JSON.stringify(USE_CASES).toLowerCase();

  assert.match(pageCopy, /80%/);
  assert.match(pageCopy, /hard cap/);
  assert.match(pageCopy, /x402/);
  assert.match(pageCopy, /shell/);
  assert.match(pageCopy, /spending caps/);
  assert.match(pageCopy, /allow/);
  assert.match(pageCopy, /hold/);
  assert.match(pageCopy, /block/);
  assert.match(pageCopy, /receipt/);
});
