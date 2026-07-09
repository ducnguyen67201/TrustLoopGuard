import { promises as fs } from 'node:fs';
import path from 'node:path';

const DOCS_ROOT = path.join(process.cwd(), 'content/docs');
const FRONTMATTER_DELIMITER = '---';

export type LlmDocPage = {
  title: string;
  description?: string;
  url: string;
  rawUrl: string;
  raw: string;
  body: string;
};

export function textResponse(body: string, init?: ResponseInit): Response {
  const headers = new Headers(init?.headers);
  headers.set('content-type', 'text/plain; charset=utf-8');
  return new Response(body, {
    ...init,
    headers,
  });
}

export async function listDocPages(): Promise<LlmDocPage[]> {
  const files = await listContentFiles('');
  const pages = await Promise.all(files.map(readPageFromRelativePath));
  return pages.sort((a, b) => a.url.localeCompare(b.url));
}

export async function getRawDocBySlug(slug: string[] | undefined): Promise<LlmDocPage | null> {
  const segments = normalizeSlugSegments(slug);
  if (!segments) return null;

  for (const relativePath of candidateRelativePaths(segments)) {
    try {
      return await readPageFromRelativePath(relativePath);
    } catch (error) {
      if (!isNotFound(error)) throw error;
    }
  }

  return null;
}

async function listContentFiles(relativeDir: string): Promise<string[]> {
  const absoluteDir = safeContentPath(relativeDir);
  if (!absoluteDir) return [];

  const entries = await fs.readdir(absoluteDir, { withFileTypes: true });
  const files = await Promise.all(
    entries.map(async (entry) => {
      const childRelativePath = relativeDir ? `${relativeDir}/${entry.name}` : entry.name;
      if (entry.isDirectory()) {
        return listContentFiles(childRelativePath);
      }
      if (entry.isFile() && isMarkdownFile(entry.name)) {
        return [childRelativePath];
      }
      return [];
    }),
  );

  return files.flat().sort();
}

async function readPageFromRelativePath(relativePath: string): Promise<LlmDocPage> {
  const absolutePath = safeContentPath(relativePath);
  if (!absolutePath) {
    throw new Error(`invalid docs path: ${relativePath}`);
  }

  const raw = await fs.readFile(absolutePath, 'utf8');
  const { metadata, body } = splitFrontmatter(raw);
  const routeSlug = routeSlugForRelativePath(relativePath);
  const title = metadata['title'] ?? titleFromSlug(routeSlug);
  const description = metadata['description'];

  return {
    title,
    description,
    url: routeSlug ? `/docs/${routeSlug}` : '/docs',
    rawUrl: routeSlug ? `/docs/${routeSlug}.md` : '/docs/index.md',
    raw,
    body,
  };
}

function candidateRelativePaths(segments: string[]): string[] {
  if (segments.length === 0 || (segments.length === 1 && segments[0] === 'index')) {
    return ['index.mdx', 'index.md'];
  }

  const slug = segments.join('/');
  const withoutIndex = segments.at(-1) === 'index' ? segments.slice(0, -1).join('/') : slug;
  const candidates = [
    `${slug}.mdx`,
    `${slug}.md`,
    `${slug}/index.mdx`,
    `${slug}/index.md`,
  ];

  if (withoutIndex && withoutIndex !== slug) {
    candidates.push(`${withoutIndex}/index.mdx`, `${withoutIndex}/index.md`);
  }

  return candidates;
}

function normalizeSlugSegments(slug: string[] | undefined): string[] | null {
  const segments = [...(slug ?? [])];
  if (segments.length > 0) {
    const last = segments.at(-1);
    if (last?.endsWith('.md')) {
      segments[segments.length - 1] = last.slice(0, -3);
    } else if (last?.endsWith('.mdx')) {
      segments[segments.length - 1] = last.slice(0, -4);
    }
  }

  const normalized = segments.filter(Boolean).map((segment) => decodeURIComponent(segment));
  if (
    normalized.some(
      (segment) =>
        segment === '.' ||
        segment === '..' ||
        segment.includes('\\') ||
        segment.split('/').some((part) => part === '..'),
    )
  ) {
    return null;
  }

  return normalized;
}

function routeSlugForRelativePath(relativePath: string): string {
  const withoutExtension = relativePath.replace(/\.(mdx|md)$/u, '');
  const segments = withoutExtension.split('/');
  if (segments.at(-1) === 'index') {
    segments.pop();
  }
  return segments.join('/');
}

function titleFromSlug(slug: string): string {
  const value = slug.split('/').at(-1) || 'Introduction';
  return value
    .split('-')
    .filter(Boolean)
    .map((part) => part[0]?.toUpperCase() + part.slice(1))
    .join(' ');
}

function splitFrontmatter(raw: string): { metadata: Record<string, string>; body: string } {
  if (!raw.startsWith(`${FRONTMATTER_DELIMITER}\n`)) {
    return { metadata: {}, body: raw.trimStart() };
  }

  const end = raw.indexOf(`\n${FRONTMATTER_DELIMITER}\n`, FRONTMATTER_DELIMITER.length + 1);
  if (end === -1) {
    return { metadata: {}, body: raw.trimStart() };
  }

  const frontmatter = raw.slice(FRONTMATTER_DELIMITER.length + 1, end);
  const body = raw.slice(end + FRONTMATTER_DELIMITER.length + 2).trimStart();
  const metadata = Object.fromEntries(
    frontmatter
      .split('\n')
      .map((line) => line.match(/^([A-Za-z0-9_-]+):\s*(.*)$/u))
      .filter((match): match is RegExpMatchArray => Boolean(match))
      .map((match) => [match[1], stripQuotes(match[2] ?? '')]),
  );

  return { metadata, body };
}

function stripQuotes(value: string): string {
  const trimmed = value.trim();
  if (
    (trimmed.startsWith('"') && trimmed.endsWith('"')) ||
    (trimmed.startsWith("'") && trimmed.endsWith("'"))
  ) {
    return trimmed.slice(1, -1);
  }
  return trimmed;
}

function safeContentPath(relativePath: string): string | null {
  const absolutePath = path.resolve(DOCS_ROOT, relativePath);
  const rootWithSeparator = DOCS_ROOT.endsWith(path.sep) ? DOCS_ROOT : `${DOCS_ROOT}${path.sep}`;
  if (absolutePath !== DOCS_ROOT && !absolutePath.startsWith(rootWithSeparator)) {
    return null;
  }
  return absolutePath;
}

function isMarkdownFile(fileName: string): boolean {
  return fileName.endsWith('.mdx') || fileName.endsWith('.md');
}

function isNotFound(error: unknown): boolean {
  return (
    typeof error === 'object' &&
    error !== null &&
    'code' in error &&
    (error as NodeJS.ErrnoException).code === 'ENOENT'
  );
}
