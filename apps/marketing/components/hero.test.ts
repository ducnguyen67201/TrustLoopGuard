import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import test from 'node:test';

test('the homepage hero leads with agent underwriting and the live demo', () => {
  const hero = readFileSync(new URL('./hero.tsx', import.meta.url), 'utf8');

  assert.match(hero, /We underwrite/i);
  assert.match(hero, /Every consequential action gets a price before it runs/i);
  assert.match(hero, /eligible losses are paid under the agreed terms/i);
  assert.match(hero, /href=\{locale === 'vi' \? '\/vi\/demo' : '\/demo'\}/);
  assert.match(hero, /event="demo_click"/);
  assert.match(hero, /Try the live refund demo/i);
  assert.match(hero, /href="#how"/);
  assert.match(hero, /Action value/i);
  assert.match(hero, /Risk price/i);
  assert.match(hero, /Coverage limit/i);
  assert.match(hero, /Authorized to execute/i);
  assert.match(hero, /Coverage is available only under separate agreed terms/i);
});

test('the underwriting preview is one continuous risk slip, not nested cards', () => {
  const hero = readFileSync(new URL('./hero.tsx', import.meta.url), 'utf8');
  const styles = readFileSync(new URL('../app/globals.css', import.meta.url), 'utf8');

  assert.match(hero, /quote-rate-primary/);
  assert.match(hero, /quote-terms/);
  assert.match(hero, /quote-flow/);
  assert.doesNotMatch(hero, /control-proposal|control-pricing|control-decision/);
  assert.doesNotMatch(styles, /\.control-proposal|\.control-pricing|\.control-decision/);
});

test('the hero keeps source inspection as tertiary proof', () => {
  const hero = readFileSync(new URL('./hero.tsx', import.meta.url), 'utf8');

  assert.match(hero, /Inspect the source/i);
  assert.match(hero, /hero-source-link/);
});

test('the hero puts the tracked app entry between the demo and control-flow actions', () => {
  const hero = readFileSync(new URL('./hero.tsx', import.meta.url), 'utf8');
  const demoIndex = hero.indexOf('event="demo_click"');
  const appIndex = hero.indexOf('event="app_click"');
  const controlFlowIndex = hero.indexOf('label: copy.controlFlow');

  assert.match(hero, /href=\{APP_URL\}/);
  assert.match(hero, /hero-app-link/);
  assert.match(hero, /Go to the app/);
  assert.ok(demoIndex >= 0 && demoIndex < appIndex);
  assert.ok(appIndex < controlFlowIndex);
});

test('the hero states the founder credibility precisely', () => {
  const hero = readFileSync(new URL('./hero.tsx', import.meta.url), 'utf8');
  const styles = readFileSync(new URL('../app/globals.css', import.meta.url), 'utf8');

  assert.match(hero, /former engineer at a company backed by/i);
  assert.match(hero, /hero-backing-logos/);
  assert.match(hero, /hero-backing-chip hero-backing-chip-yc/);
  assert.match(hero, /hero-backing-chip hero-backing-chip-a16z/);
  assert.match(hero, /src="\/yc-logo\.svg"/);
  assert.match(hero, /alt="Y Combinator"/);
  assert.match(hero, /aria-label="a16z"/);
  assert.match(styles, /\.hero-backing-chip\s*\{[^}]*border:/s);
  assert.doesNotMatch(styles, /\.hero-backing-logos\s*\{[^}]*border:/s);
  assert.doesNotMatch(hero, /hero-backing-logo-plus/);
  assert.doesNotMatch(hero, /hero-founder-proof/);
  assert.doesNotMatch(hero, /former (YC|a16z) engineer/i);
});

test('the homepage keeps the acquisition viewport free of the back-to-top control', () => {
  const page = readFileSync(new URL('../app/page.tsx', import.meta.url), 'utf8');

  assert.doesNotMatch(page, /ScrollTopButton/);
});
