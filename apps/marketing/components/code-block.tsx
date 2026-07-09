'use client';

import { useState } from 'react';

type Lang = 'ts' | 'python' | 'rust';

const LABELS: Record<Lang, string> = {
  ts: 'TypeScript',
  python: 'Python',
  rust: 'Rust',
};

interface CodeBlockProps {
  samples: Record<Lang, string>;
  footerLabel?: string;
}

export function CodeBlock({ samples, footerLabel = 'POST /v1/events - Decision' }: CodeBlockProps) {
  const [lang, setLang] = useState<Lang>('ts');

  return (
    <div className="code-panel">
      <div className="flex flex-col gap-3 border-b border-[var(--color-hairline)] px-4 py-3 sm:flex-row sm:items-center sm:justify-between">
        <span className="code-panel-label">Integration example</span>
        <div
          role="tablist"
          aria-label="SDK language"
            className="code-language-tabs flex p-0.5 text-xs"
        >
          {(Object.keys(samples) as Lang[]).map((l) => (
            <button
              key={l}
              role="tab"
              aria-selected={lang === l}
              onClick={() => setLang(l)}
              className={`flex-1 rounded-sm px-2 py-1 transition-colors sm:flex-none sm:px-3 ${
                lang === l
                  ? 'code-tab-active text-white'
                  : 'text-[var(--color-ink-mute)] hover:text-[var(--color-ink-dim)]'
              }`}
            >
              {LABELS[l]}
            </button>
          ))}
        </div>
      </div>
      <pre className="max-h-[30rem] overflow-auto px-4 py-5 font-mono text-[12px] leading-[1.65] text-[var(--color-ink)] sm:px-6 sm:py-6 sm:text-[13px]">
        <code>{highlight(samples[lang])}</code>
      </pre>
      <div className="flex flex-col gap-1 border-t border-[var(--color-hairline)] px-4 py-3 text-xs text-[var(--color-ink-mute)] sm:flex-row sm:items-center sm:justify-between">
        <span>{footerLabel}</span>
          <span className="font-mono text-[var(--color-allow)]">Decision returned</span>
      </div>
    </div>
  );
}

// Lightweight token highlighter: keywords, strings, and comments. No deps.
const KEYWORDS = new Set([
  'import',
  'from',
  'const',
  'let',
  'await',
  'return',
  'if',
  'new',
  'use',
  'async',
  'fn',
  'pub',
  'self',
  'as',
  'in',
  'Ok',
  'Err',
  'def',
  'class',
  'False',
  'True',
  'None',
  'or',
  'and',
  'not',
]);

function highlight(src: string): React.ReactNode {
  const lines = src.split('\n');
  return lines.map((line, i) => (
    <span key={i} className="block">
      {tokenize(line)}
      {i < lines.length - 1 ? '\n' : ''}
    </span>
  ));
}

function tokenize(line: string): React.ReactNode[] {
  // Strings, comments, identifiers, punctuation.
  const out: React.ReactNode[] = [];
  const re =
    /("[^"]*"|'[^']*'|`[^`]*`|#[^\n]*|\/\/[^\n]*|\b[A-Za-z_][A-Za-z0-9_]*\b|[^A-Za-z_"'`#/]+|.)/g;
  let key = 0;
  for (const m of line.matchAll(re)) {
    const tok = m[0];
    if (tok.startsWith('//') || tok.startsWith('#')) {
      out.push(
        <span key={key++} className="text-[var(--color-ink-mute)]">
          {tok}
        </span>,
      );
    } else if (tok.startsWith('"') || tok.startsWith("'") || tok.startsWith('`')) {
      out.push(
        <span key={key++} className="text-[var(--color-allow)]">
          {tok}
        </span>,
      );
    } else if (KEYWORDS.has(tok)) {
      out.push(
        <span key={key++} className="text-[var(--color-accent-deep)]">
          {tok}
        </span>,
      );
    } else if (/^[A-Z][A-Za-z0-9_]*$/.test(tok)) {
      out.push(
        <span key={key++} className="text-[var(--color-block)]">
          {tok}
        </span>,
      );
    } else {
      out.push(tok);
    }
  }
  return out;
}
