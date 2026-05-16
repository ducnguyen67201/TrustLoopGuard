'use client';

import type { ComponentProps, ComponentType, ReactNode } from 'react';

import { Button } from '@/components/ui/button';
import { cn } from '@/lib/utils';

type ButtonVariant = ComponentProps<typeof Button>['variant'];

export interface BatchAction {
  id: string;
  label: string;
  icon?: ComponentType<{ className?: string }>;
  variant?: ButtonVariant;
  disabled?: boolean;
  onSelect: () => void;
}

export function BatchActionBar({
  selectedCount,
  actions,
  onClear,
  className,
  children,
}: {
  selectedCount: number;
  actions: BatchAction[];
  onClear: () => void;
  className?: string;
  children?: ReactNode;
}) {
  if (selectedCount === 0) return null;

  return (
    <div
      className={cn(
        'flex flex-col gap-2 rounded-md border bg-background p-3 shadow-sm sm:flex-row sm:items-center sm:justify-between',
        className,
      )}
    >
      <div className="text-sm text-muted-foreground">
        <span className="font-medium text-foreground">{selectedCount}</span> selected
      </div>
      <div className="flex flex-wrap items-center gap-2">
        {children}
        {actions.map((action) => {
          const Icon = action.icon;
          return (
            <Button
              key={action.id}
              type="button"
              size="sm"
              variant={action.variant ?? 'outline'}
              disabled={action.disabled}
              onClick={action.onSelect}
            >
              {Icon ? <Icon className="size-4" /> : null}
              {action.label}
            </Button>
          );
        })}
        <Button type="button" size="sm" variant="ghost" onClick={onClear}>
          Clear
        </Button>
      </div>
    </div>
  );
}
