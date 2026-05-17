'use client';

import { useRef, useState } from 'react';
import dynamic from 'next/dynamic';
import { Button } from '@/components/ui/button';
import { Input } from '@/components/ui/input';
import { VersionPicker, type VersionEntry } from './VersionPicker';

const MonacoDiffEditor = dynamic(
  () => import('@monaco-editor/react').then((m) => m.DiffEditor),
  { ssr: false },
);

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
    <div className="flex flex-col gap-2">
      {/* Version picker — shown only when version data is provided */}
      {onVersionSelect && (
        <div className="flex items-center gap-2 px-0.5">
          <span className="text-xs text-muted-foreground shrink-0">Compare with:</span>
          <VersionPicker
            versions={versions}
            selectedVersion={selectedVersion}
            onSelect={onVersionSelect}
            loading={versionsLoading}
          />
        </div>
      )}

      {/* Editor block: pane labels + Monaco */}
      <div className="overflow-hidden rounded-md border border-border">
        {/* Pane label bar — dark to blend with vs-dark Monaco */}
        <div className="grid grid-cols-2 bg-[#1e1e1e] border-b border-[#3c3c3c] select-none">
          {/* Left: baseline / selected version */}
          <div className="flex items-center gap-2 px-3 py-1.5 border-r border-[#3c3c3c]">
            <span className="text-[10px] font-sans uppercase tracking-widest text-[#6a6a6a]">
              Baseline
            </span>
            <span className="text-[11px] font-mono text-[#c8c8c8]">{leftLabel}</span>
            <span className="ml-auto text-[10px] text-[#4a4a4a] font-sans italic">read-only</span>
          </div>
          {/* Right: current / editing */}
          <div className="flex items-center gap-2 px-3 py-1.5">
            <span className="text-[10px] font-sans uppercase tracking-widest text-[#6a6a6a]">
              Editing
            </span>
            <span className="text-[11px] font-mono text-[#c8c8c8]">Current</span>
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
        <div className="flex items-center gap-2">
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
            placeholder="Ask AI to edit… (e.g. change severity to critical)"
            disabled={aiLoading || disabled}
            className="font-mono text-xs h-8"
          />
          <Button
            type="button"
            size="sm"
            variant="secondary"
            onClick={() => void handleAiApply()}
            disabled={aiLoading || disabled || aiPrompt.trim() === ''}
            className="shrink-0"
          >
            {aiLoading ? 'Thinking…' : 'Apply'}
          </Button>
        </div>
      )}
    </div>
  );
}
