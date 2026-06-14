'use client';

import { useEffect } from 'react';
import { IconActivity, IconChevronDown, IconRefresh } from '@tabler/icons-react';

import { Badge } from '@/components/ui/badge';
import { Button } from '@/components/ui/button';
import { ButtonGroup, ButtonGroupSeparator } from '@/components/ui/button-group';
import {
  DropdownMenu,
  DropdownMenuContent,
  DropdownMenuLabel,
  DropdownMenuRadioGroup,
  DropdownMenuRadioItem,
  DropdownMenuTrigger,
} from '@/components/ui/dropdown-menu';

export type RefreshMode = 'manual' | 'live' | '1m' | '5m';

// null = no auto-refresh (manual button only). Live stays snappy for demos;
// the longer cadences are for leaving the platform open beside an agent.
const REFRESH_INTERVALS: Record<RefreshMode, number | null> = {
  manual: null,
  live: 2000,
  '1m': 60_000,
  '5m': 300_000,
};

const REFRESH_MODE_OPTIONS: ReadonlyArray<{ value: RefreshMode; label: string }> = [
  { value: 'manual', label: 'Manual' },
  { value: 'live', label: 'Live' },
  { value: '1m', label: '1 min' },
  { value: '5m', label: '5 min' },
];

const REFRESH_MODE_LABELS: Record<RefreshMode, string> = {
  manual: 'Manual',
  live: 'Live',
  '1m': '1 min',
  '5m': '5 min',
};

function isRefreshMode(value: string): value is RefreshMode {
  return value in REFRESH_INTERVALS;
}

/**
 * Polls `refresh` on the cadence implied by `mode`, pausing while the tab is
 * hidden so a backgrounded demo does not keep hammering the API.
 */
export function useAutoRefresh(refresh: () => void | Promise<void>, mode: RefreshMode): void {
  useEffect(() => {
    const interval = REFRESH_INTERVALS[mode];
    if (interval === null) return;

    const id = window.setInterval(() => {
      if (document.visibilityState === 'visible') {
        void refresh();
      }
    }, interval);

    return () => window.clearInterval(id);
  }, [refresh, mode]);
}

interface RefreshControlsProps {
  mode: RefreshMode;
  onModeChange: (mode: RefreshMode) => void;
  onRefresh: () => void;
  isRefreshing: boolean;
  lastSync: Date;
  error?: string | null;
}

export function RefreshControls({
  mode,
  onModeChange,
  onRefresh,
  isRefreshing,
  lastSync,
  error,
}: RefreshControlsProps) {
  return (
    <div className="flex flex-wrap items-center justify-end gap-x-3 gap-y-2">
      <div className="flex items-center gap-2 text-xs text-muted-foreground">
        {mode !== 'manual' ? (
          <Badge variant="outline" className="gap-1 rounded-sm">
            <IconActivity className="size-3.5 text-green-600" />
            {mode === 'live' ? 'Live' : `Every ${mode}`}
          </Badge>
        ) : null}
        <span>Updated {relativeSync(lastSync)}</span>
        {error ? <span className="text-destructive">{error}</span> : null}
      </div>
      <ButtonGroup>
        <DropdownMenu>
          <DropdownMenuTrigger asChild>
            <Button type="button" variant="outline" size="sm" className="gap-1.5">
              {REFRESH_MODE_LABELS[mode]}
              <IconChevronDown className="size-3.5 opacity-60" />
            </Button>
          </DropdownMenuTrigger>
          <DropdownMenuContent align="end" className="min-w-36">
            <DropdownMenuLabel>Refresh cadence</DropdownMenuLabel>
            <DropdownMenuRadioGroup
              value={mode}
              onValueChange={(value) => {
                if (isRefreshMode(value)) onModeChange(value);
              }}
            >
              {REFRESH_MODE_OPTIONS.map((option) => (
                <DropdownMenuRadioItem key={option.value} value={option.value}>
                  {option.label}
                </DropdownMenuRadioItem>
              ))}
            </DropdownMenuRadioGroup>
          </DropdownMenuContent>
        </DropdownMenu>
        <ButtonGroupSeparator />
        <Button
          type="button"
          variant="outline"
          size="sm"
          className="gap-2"
          onClick={() => onRefresh()}
          disabled={isRefreshing}
        >
          <IconRefresh className={isRefreshing ? 'size-4 animate-spin' : 'size-4'} />
          Refresh
        </Button>
      </ButtonGroup>
    </div>
  );
}

function relativeSync(date: Date): string {
  const seconds = Math.max(0, Math.round((Date.now() - date.getTime()) / 1000));
  if (seconds < 2) return 'just now';
  if (seconds < 60) return `${seconds}s ago`;
  return `${Math.round(seconds / 60)}m ago`;
}
