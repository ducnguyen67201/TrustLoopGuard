import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import test from 'node:test';

const repositoryRoot = new URL('../../', import.meta.url);

function read(path: string): string {
  return readFileSync(new URL(path, repositoryRoot), 'utf8');
}

test('marketing and app load one font and expose it through every type token', () => {
  const marketingLayout = read('apps/marketing/app/layout.tsx');
  const marketingStyles = read('apps/marketing/app/globals.css');
  const appLayout = read('apps/web/app/layout.tsx');
  const appStyles = read('apps/web/app/globals.css');

  assert.match(marketingLayout, /DepartureMono-Regular\.woff2/);
  assert.match(appLayout, /DepartureMono-Regular\.woff2/);
  assert.doesNotMatch(marketingLayout, /next\/font\/google|GeistMono/);
  assert.doesNotMatch(appLayout, /next\/font\/google|IBM_Plex_Mono/);
  assert.equal(marketingLayout.match(/localFont\(/g)?.length, 1);
  assert.equal(appLayout.match(/localFont\(/g)?.length, 1);

  const primaryStack =
    /--font-sans:\s*var\(--font-primary\), ui-monospace, ['"]SFMono-Regular['"], monospace;/;
  for (const styles of [marketingStyles, appStyles]) {
    assert.match(styles, primaryStack);
    assert.match(styles, /--font-mono:\s*var\(--font-sans\);/);
    assert.match(styles, /--default-font-family:\s*var\(--font-sans\);/);
    assert.match(styles, /--default-mono-font-family:\s*var\(--font-sans\);/);
  }

  for (const path of [
    'apps/marketing/app/globals.css',
    'apps/marketing/app/demo/demo.module.css',
    'apps/marketing/app/demo/procurement/procurement.module.css',
  ]) {
    assert.doesNotMatch(read(path), /var\(--font-(?:display|inter|pixel)\)/);
    assert.doesNotMatch(read(path), /font-family:\s*var\(--font-mono\),\s*monospace;/);
  }
});

test('the Featherlane AI wordmark stays pixel-set from marketing into the app', () => {
  const marketingStyles = read('apps/marketing/app/globals.css');
  const demoStyles = read('apps/marketing/app/demo/demo.module.css');

  const wordmarkRule =
    /\.wordmark\s*{[^}]*font-family: var\(--font-mono\);[^}]*font-weight: 400;[^}]*letter-spacing: 0\.025em;[^}]*white-space: nowrap;/s;
  assert.match(marketingStyles, wordmarkRule);
  assert.match(demoStyles, wordmarkRule);

  for (const path of [
    'apps/web/components/app-sidebar.tsx',
    'apps/web/app/(auth)/auth-screen.tsx',
    'apps/web/app/(auth)/signout/page.tsx',
    'apps/web/app/onboarding/workspace/SetupBrandHeader.tsx',
    'apps/web/app/welcome/WelcomeBrandHeader.tsx',
  ]) {
    assert.match(
      read(path),
      /<span className="[^"]*font-mono[^"]*">\s*Featherlane AI\s*<\/span>/,
      `${path} must use the canonical pixel wordmark`,
    );
  }
});

test('the marketing hero uses the same pixel-font token as install commands', () => {
  const marketingStyles = read('apps/marketing/app/globals.css');

  assert.match(
    marketingStyles,
    /\.hero-title\s*{[^}]*font-family: var\(--font-mono\);[^}]*font-weight: 400;/s,
  );
});

test('translated demo safety notes wrap within the mobile viewport', () => {
  const demoStyles = read('apps/marketing/app/demo/demo.module.css');

  assert.match(
    demoStyles,
    /@media \(max-width: 42rem\)\s*{[\s\S]*?\.safetyNote\s*{[^}]*max-width: 100%;[^}]*white-space: normal;/,
  );
});
