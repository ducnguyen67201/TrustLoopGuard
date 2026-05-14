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
}

export function CodeBlock({ samples }: CodeBlockProps) {
  const [lang, setLang] = useState<Lang>('ts');

  return (
    <div className="glass relative overflow-hidden rounded-2xl">
      <div className="flex items-center justify-between border-b border-[var(--color-hairline)] px-4 py-3">
        <div className="flex items-center gap-1.5" aria-hidden>
          <span className="h-2.5 w-2.5 rounded-full bg-[var(--color-block)]/70" />
          <span className="h-2.5 w-2.5 rounded-full bg-[var(--color-rewrite)]/70" />
          <span className="h-2.5 w-2.5 rounded-full bg-[var(--color-allow)]/70" />
        </div>
        <div
          role="tablist"
          aria-label="SDK language"
          className="flex rounded-full bg-[var(--color-canvas-tint)] p-0.5 text-xs"
        >
          {(Object.keys(samples) as Lang[]).map((l) => (
            <button
              key={l}
              role="tab"
              aria-selected={lang === l}
              onClick={() => setLang(l)}
              className={`rounded-full px-3 py-1 transition-colors ${
                lang === l
                  ? 'bg-white text-[var(--color-ink)] shadow-sm'
                  : 'text-[var(--color-ink-mute)] hover:text-[var(--color-ink-dim)]'
              }`}
            >
              {LABELS[l]}
            </button>
          ))}
        </div>
      </div>
      <pre className="overflow-x-auto px-6 py-6 font-mono text-[13px] leading-[1.7] text-[var(--color-ink)]">
        <code>{highlight(samples[lang])}</code>
      </pre>
      <div className="flex items-center justify-between border-t border-[var(--color-hairline)] px-4 py-3 text-xs text-[var(--color-ink-mute)]">
        <span>POST /v1/check → Decision</span>
        <span className="font-mono">
          <span className="text-[var(--color-allow)]">●</span> 200 OK · 3.4ms
        </span>
      </div>
    </div>
  );
}

// Lightweight token highlighter — keywords + strings + comments. No deps.
const KEYWORDS = new Set([
  'import', 'from', 'const', 'let', 'await', 'return', 'if', 'new',
  'use', 'async', 'fn', 'pub', 'self', 'as', 'in', 'Ok', 'Err',
  'def', 'class', 'False', 'True', 'None', 'or', 'and', 'not',
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
    } else if (
      tok.startsWith('"') ||
      tok.startsWith("'") ||
      tok.startsWith('`')
    ) {
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
