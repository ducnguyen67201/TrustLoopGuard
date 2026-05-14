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
    <div className="surface overflow-hidden">
      <div className="flex items-center justify-between border-b border-[var(--color-border)] px-4 py-2.5">
        <span className="font-mono text-[11px] text-[var(--color-ink-mute)]">
          quickstart.{lang === 'ts' ? 'ts' : lang === 'python' ? 'py' : 'rs'}
        </span>
        <div
          role="tablist"
          aria-label="SDK language"
          className="flex gap-0.5 rounded-md border border-[var(--color-border)] bg-[var(--color-canvas-soft)] p-0.5 text-xs"
        >
          {(Object.keys(samples) as Lang[]).map((l) => (
            <button
              key={l}
              role="tab"
              aria-selected={lang === l}
              onClick={() => setLang(l)}
              className={`rounded-[5px] px-2.5 py-1 transition-colors ${
                lang === l
                  ? 'bg-[var(--color-surface)] text-[var(--color-ink)] shadow-sm'
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
      <div className="flex items-center justify-between border-t border-[var(--color-border)] px-4 py-2.5 font-mono text-[11px] text-[var(--color-ink-mute)]">
        <span>POST /v1/check</span>
        <span>
          <span
            aria-hidden
            className="mr-1.5 inline-block h-1.5 w-1.5 rounded-full align-middle"
            style={{ background: 'var(--color-allow)' }}
          />
          200 OK · 3.4 ms
        </span>
      </div>
    </div>
  );
}

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
        <span key={key++} className="text-[var(--color-accent)]">
          {tok}
        </span>,
      );
    } else if (/^[A-Z][A-Za-z0-9_]*$/.test(tok)) {
      out.push(
        <span key={key++} className="text-[var(--color-ink)]">
          {tok}
        </span>,
      );
    } else {
      out.push(
        <span key={key++} className="text-[var(--color-ink-dim)]">
          {tok}
        </span>,
      );
    }
  }
  return out;
}
