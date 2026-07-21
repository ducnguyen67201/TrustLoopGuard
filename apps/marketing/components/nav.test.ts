import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import test from 'node:test';

test('the marketing header tracks the live demo on desktop and compact layouts', () => {
  const desktopNav = readFileSync(new URL('./nav.tsx', import.meta.url), 'utf8');
  const compactNav = readFileSync(new URL('./nav-actions.tsx', import.meta.url), 'utf8');
  const styles = readFileSync(new URL('../app/globals.css', import.meta.url), 'utf8');

  assert.match(desktopNav, /locale === 'vi' \? '\/vi\/demo' : '\/demo'/);
  assert.match(desktopNav, /event="demo_click"/);
  assert.match(compactNav, /locale === 'vi' \? '\/vi\/demo' : '\/demo'/);
  assert.match(compactNav, /event="demo_click"/);
  assert.match(compactNav, /nav-demo-compact/);
  assert.match(
    styles,
    /@media\s*\(min-width:\s*1100px\)\s*{[\s\S]*?\.button-secondary\.nav-demo-compact\s*{[^}]*display:\s*none/,
  );
});

test('the marketing header exposes a tracked app entry action', () => {
  const actions = readFileSync(new URL('./nav-actions.tsx', import.meta.url), 'utf8');
  const styles = readFileSync(new URL('../app/globals.css', import.meta.url), 'utf8');

  assert.match(actions, /href=\{APP_URL\}/);
  assert.match(actions, /event="app_click"/);
  assert.match(actions, /Go to the app/);
  assert.match(actions, /nav-talk/);
  assert.match(styles, /\.site-nav \.nav-talk\s*{[^}]*display:\s*none/);
});

test('the marketing header does not expose a manual language switch', () => {
  const compactNav = readFileSync(new URL('./nav-actions.tsx', import.meta.url), 'utf8');

  assert.doesNotMatch(compactNav, /languageCode|hrefLang|localePreferenceHref/);
});
