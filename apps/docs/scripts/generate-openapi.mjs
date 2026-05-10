// Regenerates apps/docs/content/docs/reference/api/*.mdx from
// docs/openapi.yaml. Run via `pnpm --filter docs gen:openapi`. Safe to
// run repeatedly — the output dir is wiped first.
//
// The generated MDX is checked in (small, deterministic, easy to review).
// Regenerate after editing docs/openapi.yaml and commit the diff.

import { generateFiles } from 'fumadocs-openapi';
import { createOpenAPI } from 'fumadocs-openapi/server';
import { rm } from 'node:fs/promises';
import { resolve } from 'node:path';

const apiDir = resolve('content/docs/reference/api');

await rm(apiDir, { recursive: true, force: true });

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
