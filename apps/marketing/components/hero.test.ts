import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import test from 'node:test';

test('the homepage hero leads with a concrete outcome and the live demo', () => {
  const hero = readFileSync(new URL('./hero.tsx', import.meta.url), 'utf8');

  assert.match(hero, /send, spend, or execute/i);
  assert.match(hero, /href="\/demo"/);
  assert.match(hero, /event="demo_click"/);
  assert.match(hero, /Try the live refund demo/i);
  assert.match(hero, /href="#how"/);
  assert.match(hero, /execution not started/i);
  assert.match(hero, /require_approval/i);
});

test('the hero keeps source inspection as tertiary proof', () => {
  const hero = readFileSync(new URL('./hero.tsx', import.meta.url), 'utf8');

  assert.match(hero, /Inspect the source/i);
  assert.match(hero, /hero-source-link/);
});

test('the homepage keeps the acquisition viewport free of the back-to-top control', () => {
  const page = readFileSync(new URL('../app/page.tsx', import.meta.url), 'utf8');

  assert.doesNotMatch(page, /ScrollTopButton/);
});
