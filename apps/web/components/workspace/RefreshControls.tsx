'use client';

import { useEffect } from 'react';
import { IconAlertTriangle, IconChevronDown, IconRefresh } from '@tabler/icons-react';

import { Badge } from '@/components/ui/badge';
import { Button } from '@/components/ui/button';
import { ButtonGroup, ButtonGroupSeparator } from '@/components/ui/button-group';
import { InfoHint } from '@/components/ui/info-hint';
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

const REFRESH_MODE_OPTIONS: ReadonlyArray<{ value: RefreshMode; label: string; hint: string }> = [
  { value: 'manual', label: 'Manual', hint: 'Refresh on demand' },
  { value: 'live', label: 'Live', hint: 'Every 2 seconds' },
  { value: '1m', label: '1 min', hint: 'Every minute' },
  { value: '5m', label: '5 min', hint: 'Every 5 minutes' },
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
  const isLive = mode !== 'manual';

  return (
    <div className="flex flex-wrap items-center justify-end gap-x-3 gap-y-2">
      <div className="flex items-center gap-2 text-xs">
        {isLive ? (
          <Badge variant="secondary" className="gap-1.5">
            <LiveDot />
            {mode === 'live' ? 'Live' : `Every ${mode}`}
          </Badge>
        ) : (
          <Badge variant="outline" className="gap-1.5 text-muted-foreground">
            <span className="size-1.5 rounded-full bg-muted-foreground/60" />
            Paused
          </Badge>
        )}
        <InfoHint label="What does the live indicator mean?">
          {isLive
            ? 'New activity loads on its own — you don’t need to do anything. Use the menu to change how often, or pause it.'
            : 'Auto-updating is off. Press Refresh to load the latest, or pick a live option from the menu.'}
        </InfoHint>
        {error ? (
          <span
            className="inline-flex items-center gap-1 text-destructive"
            role="status"
            aria-live="polite"
          >
            <IconAlertTriangle className="size-3.5" aria-hidden />
            <span aria-hidden>Sync failed</span>
            <span className="sr-only">Sync failed{error ? `: ${error}` : ''}</span>
          </span>
        ) : (
          <span className="text-muted-foreground">
            Updated{' '}
            <time dateTime={lastSync.toISOString()} className="font-data text-foreground/80">
              {relativeSync(lastSync)}
            </time>
          </span>
        )}
      </div>
      <ButtonGroup>
        <DropdownMenu>
          <DropdownMenuTrigger asChild>
            <Button type="button" variant="outline" size="sm" className="gap-1.5">
              {REFRESH_MODE_LABELS[mode]}
              <IconChevronDown className="size-3.5 opacity-60" />
            </Button>
          </DropdownMenuTrigger>
          <DropdownMenuContent align="end" className="min-w-44">
            <DropdownMenuLabel>How often to update</DropdownMenuLabel>
            <DropdownMenuRadioGroup
              value={mode}
              onValueChange={(value) => {
                if (isRefreshMode(value)) onModeChange(value);
              }}
            >
              {REFRESH_MODE_OPTIONS.map((option) => (
                <DropdownMenuRadioItem key={option.value} value={option.value}>
                  <span className="flex flex-col">
                    <span>{option.label}</span>
                    <span className="text-xs text-muted-foreground">{option.hint}</span>
                  </span>
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
          <IconRefresh
            className={isRefreshing ? 'size-4 animate-spin motion-reduce:animate-none' : 'size-4'}
          />
          Refresh
        </Button>
      </ButtonGroup>
    </div>
  );
}

/**
 * Pulsing live indicator. Inherits the host badge's text color via
 * `currentColor` so connection status never borrows a verdict token.
 * Pulse halts under prefers-reduced-motion.
 */
function LiveDot() {
  return (
    <span className="relative flex size-2 text-foreground">
      <span className="absolute inline-flex h-full w-full animate-ping rounded-full bg-current opacity-60 motion-reduce:hidden" />
      <span className="relative inline-flex size-2 rounded-full bg-current" />
    </span>
  );
}

function relativeSync(date: Date): string {
  const seconds = Math.max(0, Math.round((Date.now() - date.getTime()) / 1000));
  if (seconds < 2) return 'just now';
  if (seconds < 60) return `${seconds}s ago`;
  return `${Math.round(seconds / 60)}m ago`;
}
