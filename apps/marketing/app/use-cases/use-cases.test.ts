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

test('the use-cases page presents the four supported customer workflows', () => {
  assert.deepEqual(
    USE_CASES.map((useCase) => useCase.slug),
    ['ai-inference-spend', 'x402-payments', 'action-authorization', 'email'],
  );
});

test('every use case has its own canonical detail route', () => {
  assert.deepEqual(
    USE_CASES.map((useCase) => useCase.href),
    [
      '/use-cases/ai-inference-spend',
      '/use-cases/x402-payments',
      '/use-cases/action-authorization',
      '/use-case/email',
    ],
  );
});

test('detail routes resolve only supported use-case slugs', () => {
  assert.equal(
    getUseCase('x402-payments')?.title,
    'Authorize the purchase before the agent signs.',
  );
  assert.equal(getUseCase('email')?.title, 'Stop the wrong email before it leaves.');
  assert.equal(getUseCase('not-a-use-case'), undefined);
});

test('the navigation dropdown exposes the overview and every detail page', () => {
  assert.deepEqual(USE_CASE_NAV_ITEMS, [
    { href: '/use-cases', label: 'All use cases', detail: 'Choose a control boundary' },
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
    {
      href: '/use-case/email',
      label: 'Email action control',
      detail: 'Authorize before external send',
    },
  ]);
});

test('the navigation mega-menu separates the overview from its four use-case columns', () => {
  assert.deepEqual(USE_CASE_NAV_GROUPS.overview, USE_CASE_NAV_ITEMS[0]);
  assert.deepEqual(
    USE_CASE_NAV_GROUPS.details.map((item) => item.href),
    [
      '/use-cases/ai-inference-spend',
      '/use-cases/x402-payments',
      '/use-cases/action-authorization',
      '/use-case/email',
    ],
  );
});

test('email control binds authorization to the proposed send and its outcome', () => {
  const email = getUseCase('email');

  assert.equal(email?.href, '/use-case/email');
  assert.match(email?.control ?? '', /exact proposed version/i);
  assert.match(JSON.stringify(email?.checks), /recipient/i);
  assert.match(JSON.stringify(email?.checks), /duplicate/i);
  assert.match(JSON.stringify(email?.proof), /provider outcome/i);
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
  assert.match(pageCopy, /allow/);
  assert.match(pageCopy, /hold/);
  assert.match(pageCopy, /block/);
  assert.match(pageCopy, /receipt/);
});
