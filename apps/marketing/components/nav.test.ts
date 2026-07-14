import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import test from 'node:test';

test('the marketing header links to the live demo on desktop and compact layouts', () => {
  const desktopNav = readFileSync(new URL('./nav.tsx', import.meta.url), 'utf8');
  const compactNav = readFileSync(new URL('./nav-actions.tsx', import.meta.url), 'utf8');
  const styles = readFileSync(new URL('../app/globals.css', import.meta.url), 'utf8');

  assert.match(desktopNav, /href="\/demo"[^>]*>Demo</);
  assert.match(compactNav, /href="\/demo"/);
  assert.match(compactNav, /nav-demo-compact/);
  assert.match(styles, /\.nav-demo-compact\s*{[^}]*display:\s*flex/s);
  assert.match(
    styles,
    /@media\s*\(min-width:\s*940px\)\s*{[\s\S]*?\.nav-demo-compact\s*{[^}]*display:\s*none/,
  );
});
