'use client';

import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from '@/components/ui/select';

export interface VersionEntry {
  version: number;
  created_at: string;
}

interface VersionPickerProps {
  versions: VersionEntry[];
  selectedVersion: number | null;
  onSelect: (version: number) => void;
  loading?: boolean;
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

export function VersionPicker({ versions, selectedVersion, onSelect, loading }: VersionPickerProps) {
  if (loading) {
    return <span className="text-xs text-muted-foreground">Loading…</span>;
  }

  if (versions.length === 0) {
    return <span className="text-xs text-muted-foreground">No history yet</span>;
  }

  const latestVersion = versions[0]?.version;
  const selected = versions.find((v) => v.version === selectedVersion);

  const triggerLabel = selected
    ? `v${selected.version}${selected.version === latestVersion ? ' · latest' : ''} · ${relativeTime(selected.created_at)}`
    : 'Select version';

  return (
    <Select
      value={selectedVersion?.toString() ?? ''}
      onValueChange={(val) => onSelect(Number(val))}
    >
      <SelectTrigger className="h-7 w-52 text-xs font-mono gap-1.5 px-2">
        <SelectValue placeholder="Select version">
          {triggerLabel}
        </SelectValue>
      </SelectTrigger>
      <SelectContent className="font-mono text-xs max-h-80">
        {versions.map((v) => {
          const isLatest = v.version === latestVersion;
          return (
            <SelectItem
              key={v.version}
              value={v.version.toString()}
              className="text-xs"
            >
              <span className="flex items-center gap-2 w-full">
                <span className="tabular-nums">v{v.version}</span>
                {isLatest && (
                  <span className="rounded px-1 py-0 text-[10px] bg-primary/15 text-primary font-sans">
                    latest
                  </span>
                )}
                <span className="ml-auto text-muted-foreground font-sans">
                  {relativeTime(v.created_at)}
                </span>
              </span>
            </SelectItem>
          );
        })}
      </SelectContent>
    </Select>
  );
}
