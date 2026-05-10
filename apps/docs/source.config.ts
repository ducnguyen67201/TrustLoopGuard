import { defineDocs, defineConfig } from 'fumadocs-mdx/config';

export const docs = defineDocs({
  dir: 'content/docs',
  // Surface processed (LLM-friendly) markdown on each page via
  // page.data.getText('processed'). Used by the agent-readable endpoints
  // (/llms.txt, /llms-full.txt, /docs/<path>.md).
  docs: {
    postprocess: {
      includeProcessedMarkdown: true,
    },
  },
});

export default defineConfig();
