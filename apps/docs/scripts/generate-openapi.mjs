// Regenerates apps/docs/content/docs/reference/api/*.mdx from
// docs/openapi.yaml. Run via `pnpm --filter docs gen:openapi`.
//
// Safe to run repeatedly — wipes only generated .mdx files so a stale
// endpoint disappears when removed from the spec, but leaves
// hand-curated meta.json (sidebar order) intact.
//
// The generated MDX is checked in (small, deterministic, diff-able).
// Regenerate after editing docs/openapi.yaml and commit the result.
// CI fails if regen produces a diff (.github/workflows/docs-ci.yml).

import { generateFiles } from 'fumadocs-openapi';
import { createOpenAPI } from 'fumadocs-openapi/server';
import { readdir, rm } from 'node:fs/promises';
import { join, resolve } from 'node:path';

const apiDir = resolve('content/docs/reference/api');

const existing = await readdir(apiDir).catch(() => []);
await Promise.all(
  existing
    .filter((name) => name.endsWith('.mdx'))
    .map((name) => rm(join(apiDir, name))),
);

// Named input — the key (`trustloopguard`) becomes the schemaId in
// generated MDX (`<APIPage document="trustloopguard" .../>`), which keeps
// the output portable across machines and CI. The path is only used at
// generation time.
const server = createOpenAPI({
  input: () => ({
    trustloopguard: resolve('../../docs/openapi.yaml'),
  }),
});

await generateFiles({
  input: server,
  output: apiDir,
});

console.log(`[openapi] wrote API reference to ${apiDir}`);
