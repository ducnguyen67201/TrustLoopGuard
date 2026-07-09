'use client';

import type { ReactNode } from 'react';
import Link from 'next/link';
import {
  closestCenter,
  DndContext,
  PointerSensor,
  useSensor,
  useSensors,
  type DragEndEvent,
} from '@dnd-kit/core';
import {
  arrayMove,
  rectSortingStrategy,
  SortableContext,
  useSortable,
} from '@dnd-kit/sortable';
import { CSS } from '@dnd-kit/utilities';
import { GripVertical } from 'lucide-react';
import { IconArrowRight, IconLayoutGrid } from '@tabler/icons-react';

import { Button } from '@/components/ui/button';
import {
  DropdownMenu,
  DropdownMenuCheckboxItem,
  DropdownMenuContent,
  DropdownMenuItem,
  DropdownMenuLabel,
  DropdownMenuSeparator,
  DropdownMenuTrigger,
} from '@/components/ui/dropdown-menu';
import { InfoHint } from '@/components/ui/info-hint';
import { PageHeader } from '@/components/ui/page-header';
import { cn } from '@/lib/utils';
import type { WorkspaceDashboardData } from '@/lib/server/dashboard-data';
import {
  DASHBOARD_WIDGETS,
  type DashboardUsageData,
  type DashboardWidget,
  type WidgetKey,
  type WidgetProps,
} from './dashboard-widgets';
import { useDashboardLayout } from './use-dashboard-layout';
import type { UsagePeriod } from './usage-utils';

export type { DashboardUsageData };

const WIDGETS_BY_KEY = new Map<WidgetKey, DashboardWidget>(
  DASHBOARD_WIDGETS.map((widget) => [widget.key, widget]),
);

export function WorkspaceDashboard({
  data,
  usage,
  usagePeriod,
}: {
  data: WorkspaceDashboardData;
  usage: DashboardUsageData;
  usagePeriod: UsagePeriod;
}) {
  const { visibleKeys, isHidden, toggle, move, reset, isCustomizing, setCustomizing } =
    useDashboardLayout(data.activeWorkspace.slug);

  const widgetProps: WidgetProps = { data, usage, usagePeriod };

  const sensors = useSensors(useSensor(PointerSensor, { activationConstraint: { distance: 6 } }));

  function handleDragEnd(event: DragEndEvent) {
    const { active, over } = event;
    if (over && active.id !== over.id) {
      move(active.id as WidgetKey, over.id as WidgetKey);
    }
  }

  return (
    <div className="grid gap-6 px-4 lg:px-6">
      <PageHeader
        eyebrow={data.organization.name}
        title={data.activeWorkspace.name}
        description={`This is your safety overview for ${data.activeEnvironment.name}. Every time one of your AI apps makes a request, the guardrail checks it and the result shows up below.`}
        help={<InfoHint term="workspace" />}
        actions={
          <>
            <DropdownMenu>
              <DropdownMenuTrigger asChild>
                <Button variant="outline">
                  <IconLayoutGrid />
                  Customize
                </Button>
              </DropdownMenuTrigger>
              <DropdownMenuContent align="end" className="w-60">
                <DropdownMenuLabel>Show on dashboard</DropdownMenuLabel>
                {DASHBOARD_WIDGETS.map((widget) => (
                  <DropdownMenuCheckboxItem
                    key={widget.key}
                    checked={!isHidden(widget.key)}
                    onCheckedChange={() => toggle(widget.key)}
                    onSelect={(event) => event.preventDefault()}
                  >
                    {widget.title}
                  </DropdownMenuCheckboxItem>
                ))}
                <DropdownMenuSeparator />
                <DropdownMenuCheckboxItem
                  checked={isCustomizing}
                  onCheckedChange={setCustomizing}
                  onSelect={(event) => event.preventDefault()}
                >
                  Reorder widgets
                </DropdownMenuCheckboxItem>
                <DropdownMenuItem onSelect={() => reset()}>Reset to default</DropdownMenuItem>
              </DropdownMenuContent>
            </DropdownMenu>
            <Button asChild variant="outline">
              <Link href="/settings">Settings</Link>
            </Button>
            <Button asChild>
              <Link href="/policies">
                Review protection rules
                <IconArrowRight />
              </Link>
            </Button>
          </>
        }
      />

      <DndContext sensors={sensors} collisionDetection={closestCenter} onDragEnd={handleDragEnd}>
        <SortableContext items={visibleKeys} strategy={rectSortingStrategy}>
          <div className="grid gap-6 xl:grid-cols-2">
            {visibleKeys.map((key) => {
              const widget = WIDGETS_BY_KEY.get(key);
              if (!widget) return null;
              return (
                <SortableWidget
                  key={key}
                  id={key}
                  span={widget.span}
                  isCustomizing={isCustomizing}
                >
                  {widget.render(widgetProps)}
                </SortableWidget>
              );
            })}
          </div>
        </SortableContext>
      </DndContext>
    </div>
  );
}

function SortableWidget({
  id,
  span,
  isCustomizing,
  children,
}: {
  id: WidgetKey;
  span: 'full' | 'half';
  isCustomizing: boolean;
  children: ReactNode;
}) {
  const { attributes, listeners, setNodeRef, transform, transition, isDragging } = useSortable({
    id,
    disabled: !isCustomizing,
  });

  return (
    <div
      ref={setNodeRef}
      // Translate only — never scale. Widgets have mixed spans/heights, and
      // CSS.Transform would apply rectSortingStrategy's scale ratio, ballooning
      // a card when it's dragged over a differently-sized slot.
      style={{ transform: CSS.Translate.toString(transform), transition }}
      className={cn(
        'relative min-w-0',
        span === 'full' && 'xl:col-span-2',
        isDragging && 'z-10 opacity-80',
      )}
    >
      {isCustomizing ? (
        <button
          type="button"
          aria-label="Drag to reorder"
          className="absolute -top-2 -left-2 z-10 inline-flex size-7 cursor-grab items-center justify-center rounded-md border bg-card text-muted-foreground shadow-sm hover:text-foreground focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring/50 active:cursor-grabbing"
          {...attributes}
          {...listeners}
        >
          <GripVertical className="size-4" />
        </button>
      ) : null}
      {children}
    </div>
  );
}
