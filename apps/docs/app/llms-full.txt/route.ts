import { listDocPages, textResponse } from '@/lib/llm-docs';

export async function GET() {
  const pages = await listDocPages();
  const body = pages
    .map((page) => {
      const parts = [
        `# ${page.title}`,
        '',
        `URL: ${page.url}`,
        `Raw: ${page.rawUrl}`,
      ];
      if (page.description) {
        parts.push(`Description: ${page.description}`);
      }
      parts.push('', page.body.trim(), '');
      return parts.join('\n');
    })
    .join('\n---\n\n');

  return textResponse(`# Featherlane AI Docs Full\n\n${body}`);
}
