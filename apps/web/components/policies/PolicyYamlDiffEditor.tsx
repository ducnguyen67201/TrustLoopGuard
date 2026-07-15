'use client';

import dynamic from 'next/dynamic';
import { Loader2, Sparkles } from 'lucide-react';
import { useRef, useState } from 'react';

import { Button } from '@/components/ui/button';
import { Input } from '@/components/ui/input';
import { VersionPicker, type VersionEntry } from './VersionPicker';

const MonacoDiffEditor = dynamic(() => import('@monaco-editor/react').then((m) => m.DiffEditor), {
  ssr: false,
});

interface Props {
  original: string;
  modified: string;
  onChange: (value: string) => void;
  onAiEdit?: (instruction: string) => Promise<void>;
  // Version chrome — optional; omit to hide version picker entirely
  versions?: VersionEntry[];
  selectedVersion?: number | null;
  onVersionSelect?: (version: number) => void;
  versionsLoading?: boolean;
  height?: string;
  disabled?: boolean;
}

function relativeTime(iso: string): string {
  const diff = Date.now() - new Date(iso).getTime();
  const m = Math.floor(diff / 60_000);
  const h = Math.floor(m / 60);
  const d = Math.floor(h / 24);
  if (d > 0) return `${d}d ago`;
  if (h > 0) return `${h}h ago`;
  if (m > 0) return `${m}m ago`;
  return 'just now';
}

export function PolicyYamlDiffEditor({
  original,
  modified,
  onChange,
  onAiEdit,
  versions = [],
  selectedVersion = null,
  onVersionSelect,
  versionsLoading = false,
  height = '360px',
  disabled = false,
}: Props) {
  const [aiPrompt, setAiPrompt] = useState('');
  const [aiLoading, setAiLoading] = useState(false);
  const inputRef = useRef<HTMLInputElement>(null);

  const selectedEntry = versions.find((v) => v.version === selectedVersion);

  // Left pane label: version number + relative timestamp
  const leftLabel = selectedEntry
    ? `v${selectedEntry.version} · ${relativeTime(selectedEntry.created_at)}`
    : 'Saved';

  async function handleAiApply() {
    if (!onAiEdit || aiPrompt.trim() === '') return;
    setAiLoading(true);
    try {
      await onAiEdit(aiPrompt.trim());
      setAiPrompt('');
    } finally {
      setAiLoading(false);
    }
  }

  return (
    <div className="flex flex-col gap-3">
      {/* Version picker — shown only when version data is provided */}
      {onVersionSelect && (
        <div className="flex flex-wrap items-center gap-2">
          <span className="shrink-0 text-xs font-medium text-muted-foreground">
            Compare your edits with an earlier saved version
          </span>
          <VersionPicker
            versions={versions}
            selectedVersion={selectedVersion}
            onSelect={onVersionSelect}
            loading={versionsLoading}
          />
        </div>
      )}

      <p className="text-xs leading-relaxed text-muted-foreground">
        The left side is the last saved rule; the right side is what you&apos;re editing. Changed
        lines are highlighted. You can also ask AI to make a change for you below.
      </p>

      {/* Editor block: pane labels + Monaco */}
      <div className="overflow-hidden rounded-lg border border-border shadow-sm">
        {/* Pane label bar — dark to blend with vs-dark Monaco */}
        <div className="grid grid-cols-2 select-none border-b border-[#3c3c3c] bg-[#1e1e1e]">
          {/* Left: baseline / selected version */}
          <div className="flex items-center gap-2 border-r border-[#3c3c3c] px-3 py-2">
            <span className="text-[10px] font-sans uppercase tracking-widest text-[#6a6a6a]">
              Baseline
            </span>
            <span className="text-[11px] font-mono tabular-nums text-[#c8c8c8]">{leftLabel}</span>
            <span className="ml-auto text-[10px] font-sans italic text-[#4a4a4a]">read-only</span>
          </div>
          {/* Right: current / editing */}
          <div className="flex items-center gap-2 px-3 py-2">
            <span className="text-[10px] font-sans uppercase tracking-widest text-[#6a6a6a]">
              Editing
            </span>
            <span className="text-[11px] font-mono text-[#c8c8c8]">Current</span>
            <span className="ml-auto size-1.5 rounded-full bg-[var(--color-permit)]" aria-hidden />
          </div>
        </div>

        <MonacoDiffEditor
          height={height}
          language="yaml"
          theme="vs-dark"
          original={original}
          modified={modified}
          options={{
            renderSideBySide: true,
            originalEditable: false,
            readOnly: disabled,
            minimap: { enabled: false },
            fontSize: 13,
            lineNumbers: 'off',
            scrollBeyondLastLine: false,
            wordWrap: 'on',
            padding: { top: 8, bottom: 8 },
          }}
          onMount={(editor) => {
            const modifiedEditor = editor.getModifiedEditor();
            modifiedEditor.onDidChangeModelContent(() => {
              onChange(modifiedEditor.getValue());
            });
          }}
        />
      </div>

      {/* AI edit bar */}
      {onAiEdit && (
        <div className="flex items-center gap-2 rounded-lg border bg-muted/40 p-2">
          <Sparkles className="ml-1 size-4 shrink-0 text-muted-foreground" aria-hidden />
          <Input
            ref={inputRef}
            value={aiPrompt}
            onChange={(e) => setAiPrompt(e.target.value)}
            onKeyDown={(e) => {
              if (e.key === 'Enter' && !e.shiftKey) {
                e.preventDefault();
                void handleAiApply();
              }
            }}
            placeholder="Ask AI to edit… e.g. change severity to critical"
            disabled={aiLoading || disabled}
            className="h-8 border-transparent bg-transparent text-xs shadow-none focus-visible:border-border"
          />
          <Button
            type="button"
            size="sm"
            variant="secondary"
            onClick={() => void handleAiApply()}
            disabled={aiLoading || disabled || aiPrompt.trim() === ''}
            className="shrink-0"
          >
            {aiLoading ? (
              <>
                <Loader2 className="animate-spin" aria-hidden />
                Thinking…
              </>
            ) : (
              'Apply'
            )}
          </Button>
        </div>
      )}
    </div>
  );
}
