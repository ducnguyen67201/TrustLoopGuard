'use client';

import { IconCheck, IconCopy } from '@tabler/icons-react';
import { useEffect, useRef, useState } from 'react';
import { toast } from 'sonner';

import { Button } from '@/components/ui/button';

const COPIED_RESET_MS = 2000;
const DEFAULT_PREVIEW_LINES = 12;

/**
 * A copy-to-clipboard code/prose block: mono content in a quiet bordered
 * panel with an uppercase micro-label and a copy button. See
 * docs/concept/web-ui-conventions.md ("CopyBlock").
 *
 * `previewLines` sets how many lines show before the block collapses behind
 * "Show all" (default 12). The copy button always copies the full `content`,
 * never the preview — so a short preview never truncates what the user pastes.
 */
export function CopyBlock({
  label,
  content,
  previewLines = DEFAULT_PREVIEW_LINES,
}: {
  label: string;
  content: string;
  previewLines?: number;
}) {
  const [copied, setCopied] = useState(false);
  const [expanded, setExpanded] = useState(false);
  const resetTimer = useRef<ReturnType<typeof setTimeout> | null>(null);
  const lines = content.split('\n');
  const canCollapse = lines.length > previewLines;
  const visibleContent =
    canCollapse && !expanded ? `${lines.slice(0, previewLines).join('\n')}\n…` : content;

  useEffect(() => {
    return () => {
      if (resetTimer.current !== null) clearTimeout(resetTimer.current);
    };
  }, []);

  async function copy() {
    try {
      await navigator.clipboard.writeText(content);
      setCopied(true);
      if (resetTimer.current !== null) clearTimeout(resetTimer.current);
      resetTimer.current = setTimeout(() => setCopied(false), COPIED_RESET_MS);
    } catch {
      toast.error('Copy failed. Select the text and copy it manually.');
    }
  }

  return (
    <div className="min-w-0 rounded-lg border bg-muted/40">
      <div className="flex items-center justify-between gap-2 border-b px-4 py-2">
        <p className="text-3xs font-medium uppercase tracking-label text-muted-foreground">
          {label}
        </p>
        <Button
          type="button"
          size="sm"
          variant="ghost"
          onClick={copy}
          aria-label={`Copy: ${label}`}
        >
          {copied ? <IconCheck aria-hidden /> : <IconCopy aria-hidden />}
          {copied ? 'Copied' : 'Copy'}
        </Button>
      </div>
      <pre className="overflow-x-auto p-4 font-mono text-xs leading-relaxed whitespace-pre-wrap break-words">
        {visibleContent}
      </pre>
      {canCollapse ? (
        <div className="flex items-center justify-between gap-2 border-t px-2 py-1.5">
          <Button type="button" size="sm" variant="ghost" onClick={() => setExpanded(!expanded)}>
            {expanded ? 'Show less' : 'Show all'}
          </Button>
          {!expanded ? (
            <p className="pr-2 text-3xs text-muted-foreground">
              preview · {lines.length} lines copied
            </p>
          ) : null}
        </div>
      ) : null}
    </div>
  );
}
