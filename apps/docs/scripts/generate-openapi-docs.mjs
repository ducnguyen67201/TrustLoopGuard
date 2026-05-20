import { readdir, rm, readFile, writeFile } from 'node:fs/promises';
import { generateFiles } from 'fumadocs-openapi';
import { createOpenAPI } from 'fumadocs-openapi/server';

const output = './content/docs/reference/api';
const openapi = createOpenAPI({
  input: ['../../docs/openapi.yaml'],
});

await rm(output, { recursive: true, force: true });

await generateFiles({
  input: openapi,
  output,
  per: 'tag',
  meta: true,
  includeDescription: true,
  addGeneratedComment: true,
  beforeWrite(files) {
    for (let index = files.length - 1; index >= 0; index -= 1) {
      const file = files[index];

      if (file?.content.includes('operations={[]}')) {
        files.splice(index, 1);
      }
    }
  },
});

const metaPath = `${output}/meta.json`;
const meta = JSON.parse(await readFile(metaPath, 'utf8'));
const generatedPages = new Set(
  (await readdir(output))
    .filter((fileName) => fileName.endsWith('.mdx'))
    .map((fileName) => fileName.replace(/\.mdx$/, '')),
);

await writeFile(
  metaPath,
  `${JSON.stringify(
    {
      title: 'HTTP API',
      ...meta,
      pages: meta.pages.filter((page) => generatedPages.has(page)),
    },
    null,
    2,
  )}\n`,
);
