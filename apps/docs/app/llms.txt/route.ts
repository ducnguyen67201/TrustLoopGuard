import { listDocPages, textResponse } from '@/lib/llm-docs';

export async function GET() {
  const pages = await listDocPages();
  const lines = [
    '# Featherlane AI Docs',
    '',
    '> Open-source policy enforcement runtime for agents, services, and humans in the loop.',
    '',
    '## Documentation',
    '',
    ...pages.map((page) => {
      const suffix = page.description ? ` - ${page.description}` : '';
      return `- [${page.title}](${page.url})${suffix}`;
    }),
    '',
    '## Agent-readable endpoints',
    '',
    '- [/llms.txt](/llms.txt) - curated docs index',
    '- [/llms-full.txt](/llms-full.txt) - concatenated docs markdown',
    '- `/docs/<path>.md` - raw markdown source for a rendered docs page',
    '',
  ];

  return textResponse(lines.join('\n'));
}
