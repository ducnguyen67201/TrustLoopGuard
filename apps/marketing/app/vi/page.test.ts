import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import test from 'node:test';

test('the Vietnamese homepage has localized content and search metadata', () => {
  const page = readFileSync(new URL('./page.tsx', import.meta.url), 'utf8');
  const home = readFileSync(
    new URL('../../components/marketing-home.tsx', import.meta.url),
    'utf8',
  );
  const sitemap = readFileSync(new URL('../sitemap.ts', import.meta.url), 'utf8');

  assert.match(page, /MarketingHome locale="vi"/);
  assert.match(page, /canonical: '\/vi'/);
  assert.match(page, /locale: 'vi_VN'/);
  assert.match(home, /<div lang={locale}>/);
  assert.match(sitemap, /url: absoluteUrl\('\/vi'\)/);
});
