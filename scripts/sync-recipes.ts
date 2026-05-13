import { readFileSync, readdirSync, writeFileSync } from 'node:fs';
import type { Dirent } from 'node:fs';
import { join, relative } from 'node:path';
import YAML from 'yaml';

interface Recipe {
  id: string;
  snippets: Record<string, Snippet>;
  targets: Target[];
}

interface Snippet {
  language: string;
  code: string;
}

interface Target {
  file: string;
  snippet: string;
}

const mode = process.argv[2] ?? 'check';
if (mode !== 'check' && mode !== 'update') {
  process.stderr.write('usage: tsx scripts/sync-recipes.ts [check|update]\n');
  process.exit(2);
}

const recipesRoot = 'recipes';
const changed = new Set<string>();
const failures: string[] = [];
const recipePaths = listRecipePaths(recipesRoot);

if (recipePaths.length === 0) {
  failures.push(`${recipesRoot}: no recipe YAML files found`);
}

for (const recipePath of recipePaths) {
  const recipe = YAML.parse(readFileSync(recipePath, 'utf8')) as Recipe;

  for (const target of recipe.targets) {
    const snippet = recipe.snippets[target.snippet];
    if (snippet === undefined) {
      failures.push(`${recipePath}: missing snippet "${target.snippet}"`);
      continue;
    }

    let original: string;
    try {
      original = readFileSync(target.file, 'utf8');
    } catch {
      failures.push(`${recipePath}: target file not found: ${target.file}`);
      continue;
    }

    const next = replaceBlock(original, recipe.id, target.snippet, renderSnippet(snippet));

    if (next === original) continue;

    if (mode === 'update') {
      writeFileSync(target.file, next);
      changed.add(target.file);
    } else {
      failures.push(`${target.file}: recipe block ${recipe.id}:${target.snippet} is stale`);
    }
  }
}

if (failures.length > 0) {
  for (const failure of failures) {
    process.stderr.write(`${failure}\n`);
  }
  process.stderr.write('\nrun `pnpm recipes:update` to refresh recipe blocks\n');
  process.exit(1);
}

if (changed.size > 0) {
  for (const file of [...changed].sort()) {
    process.stdout.write(`updated ${relative(process.cwd(), file)}\n`);
  }
}

function listRecipePaths(directory: string): string[] {
  let entries: Dirent[];
  try {
    entries = readdirSync(directory, { withFileTypes: true });
  } catch {
    failures.push(`${directory}: recipe directory not found`);
    return [];
  }

  const paths = entries.flatMap((entry) => {
    const path = join(directory, entry.name);
    if (entry.isDirectory()) return listRecipePaths(path);
    if (entry.isFile() && /\.ya?ml$/u.test(entry.name)) return [path];
    return [];
  });

  return paths.sort();
}

function renderSnippet(snippet: Snippet): string {
  return ['```' + snippet.language, snippet.code.trimEnd(), '```'].join('\n');
}

function replaceBlock(
  source: string,
  recipeId: string,
  snippetId: string,
  renderedSnippet: string,
): string {
  const begin = `<!-- BEGIN recipe:${recipeId}:${snippetId} -->`;
  const end = `<!-- END recipe:${recipeId}:${snippetId} -->`;
  const pattern = new RegExp(
    `${escapeRegExp(begin)}(?:\\n[\\s\\S]*?)?\\n${escapeRegExp(end)}`,
    'm',
  );

  if (!pattern.test(source)) {
    failures.push(`missing recipe block ${recipeId}:${snippetId}`);
    return source;
  }

  return source.replace(pattern, [begin, '', renderedSnippet, '', end].join('\n'));
}

function escapeRegExp(value: string): string {
  return value.replace(/[.*+?^${}()|[\]\\]/g, '\\$&');
}
