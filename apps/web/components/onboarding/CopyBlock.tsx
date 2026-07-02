'use client';

import { IconCheck, IconCopy } from '@tabler/icons-react';
import { useEffect, useRef, useState } from 'react';
import { toast } from 'sonner';

import { Button } from '@/components/ui/button';

const COPIED_RESET_MS = 2000;
const PREVIEW_LINES = 12;

/**
 * A copy-to-clipboard code/prose block: mono content in a quiet bordered
 * panel with an uppercase micro-label and a copy button. See
 * docs/concept/web-ui-conventions.md ("CopyBlock").
 */
export function CopyBlock({ label, content }: { label: string; content: string }) {
  const [copied, setCopied] = useState(false);
  const [expanded, setExpanded] = useState(false);
  const resetTimer = useRef<ReturnType<typeof setTimeout> | null>(null);
  const lines = content.split('\n');
  const canCollapse = lines.length > PREVIEW_LINES;
  const visibleContent =
    canCollapse && !expanded ? `${lines.slice(0, PREVIEW_LINES).join('\n')}\n...` : content;

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
        <div className="border-t px-4 py-2">
          <Button type="button" size="sm" variant="ghost" onClick={() => setExpanded(!expanded)}>
            {expanded ? 'Show less' : 'Show all'}
          </Button>
        </div>
      ) : null}
    </div>
  );
}
