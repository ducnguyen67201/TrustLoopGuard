import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import test from 'node:test';

test('the homepage hero is concise and leads directly to install and demo', () => {
  const hero = readFileSync(new URL('./hero.tsx', import.meta.url), 'utf8');
  const home = readFileSync(new URL('./marketing-home.tsx', import.meta.url), 'utf8');

  assert.match(hero, /titleBefore: 'Policy'/);
  assert.match(hero, /titleAccent: 'approvals'/);
  assert.match(hero, /titleAfter: 'for AI agents\.'/);
  assert.match(hero, /returns a decision before anything happens/i);
  assert.match(hero, /former engineer at YC \/ a16z-backed companies/i);
  assert.match(hero, /<QuickInstall locale=\{locale\} \/>/);
  assert.match(hero, /href="#demo"/);
  assert.match(hero, /Try the demo/i);
  assert.match(hero, /Get started/i);
  assert.doesNotMatch(hero, /ApprovalPreview|proof-strip|hero-signal|See the control flow/i);
  assert.ok(home.indexOf('<Hero') < home.indexOf('<HomeDemo'));
  assert.ok(home.indexOf('<HomeDemo') < home.indexOf('<ControlLoop'));
  assert.doesNotMatch(home, /<Sdk/);
});

test('the hero keeps precise founder credibility marks', () => {
  const hero = readFileSync(new URL('./hero.tsx', import.meta.url), 'utf8');
  const styles = readFileSync(new URL('../app/globals.css', import.meta.url), 'utf8');

  assert.match(hero, /hero-backing-logos/);
  assert.match(hero, /src="\/yc-logo\.svg"/);
  assert.match(hero, /alt="Y Combinator"/);
  assert.match(hero, /aria-label="a16z"/);
  assert.match(styles, /\.hero-backing-chip\s*\{[^}]*border:/s);
});

test('the homepage keeps the acquisition viewport free of the back-to-top control', () => {
  const page = readFileSync(new URL('../app/page.tsx', import.meta.url), 'utf8');

  assert.doesNotMatch(page, /ScrollTopButton/);
});
