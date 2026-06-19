'use client';

import { History } from 'lucide-react';

import { Badge } from '@/components/ui/badge';
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
    return (
      <span className="flex h-8 items-center gap-1.5 px-2 text-xs text-muted-foreground">
        <History className="size-3.5 animate-pulse" aria-hidden />
        Loading history…
      </span>
    );
  }

  if (versions.length === 0) {
    return (
      <span className="flex h-8 items-center gap-1.5 px-2 text-xs text-muted-foreground">
        <History className="size-3.5" aria-hidden />
        No history yet
      </span>
    );
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
      <SelectTrigger className="h-8 w-56 gap-1.5 px-2 font-mono text-xs tabular-nums">
        <History className="size-3.5 text-muted-foreground" aria-hidden />
        <SelectValue placeholder="Select version">{triggerLabel}</SelectValue>
      </SelectTrigger>
      <SelectContent className="max-h-80 font-mono text-xs">
        {versions.map((v) => {
          const isLatest = v.version === latestVersion;
          return (
            <SelectItem key={v.version} value={v.version.toString()} className="text-xs">
              <span className="flex w-full items-center gap-2">
                <span className="tabular-nums">v{v.version}</span>
                {isLatest ? (
                  <Badge variant="secondary" className="px-1.5 py-0 font-sans text-[10px]">
                    latest
                  </Badge>
                ) : null}
                <span className="ml-auto font-sans text-muted-foreground">
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
