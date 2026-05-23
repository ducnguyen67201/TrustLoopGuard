"use client";

import { useEffect, useMemo, useState, type HTMLAttributes } from 'react';
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
import { Bar, BarChart, CartesianGrid, Cell, Line, LineChart, Pie, PieChart, XAxis, YAxis } from 'recharts';
import { Eye, EyeOff, GripVertical, Save } from 'lucide-react';

import { Button } from '@/components/ui/button';
import { Card, CardAction, CardContent, CardDescription, CardHeader, CardTitle } from '@/components/ui/card';
import {
  ChartContainer,
  ChartTooltip,
  ChartTooltipContent,
  type ChartConfig,
} from '@/components/ui/chart';
import {
  DropdownMenu,
  DropdownMenuCheckboxItem,
  DropdownMenuContent,
  DropdownMenuTrigger,
} from '@/components/ui/dropdown-menu';
import { Input } from '@/components/ui/input';
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from '@/components/ui/select';
import type {
  AnalyticsCatalog,
  AnalyticsChartType,
  AnalyticsDashboardView,
  AnalyticsDashboardViewConfig,
  AnalyticsDashboardWidget,
  AnalyticsDimension,
  AnalyticsFilter,
  AnalyticsMetric,
  AnalyticsQueryRequest,
  AnalyticsQueryResponse,
  AnalyticsWidgetLayout,
} from '@/lib/server/dashboard-data';

type AnalyticsChartGridProps = {
  workspaceSlug: string;
  catalog: AnalyticsCatalog;
  savedViews: AnalyticsDashboardView[];
};

const WIDGET_LIBRARY: AnalyticsDashboardWidget[] = [
  {
    id: 'trace-volume',
    title: 'Trace volume',
    metric: 'trace_count',
    chart_type: 'bar',
    group_by: 'decision',
    layout: { x: 0, y: 0, w: 6, h: 1 },
  },
  {
    id: 'intervention-rate',
    title: 'Intervention rate',
    metric: 'intervention_rate',
    chart_type: 'big_number',
    group_by: null,
    layout: { x: 6, y: 0, w: 3, h: 1 },
  },
  {
    id: 'p95-latency',
    title: 'p95 latency',
    metric: 'p95_latency_ms',
    chart_type: 'big_number',
    group_by: null,
    layout: { x: 9, y: 0, w: 3, h: 1 },
  },
  {
    id: 'agent-volume',
    title: 'Agent activity',
    metric: 'trace_count',
    chart_type: 'bar',
    group_by: 'agent_id',
    layout: { x: 0, y: 1, w: 6, h: 1 },
  },
  {
    id: 'policy-interventions',
    title: 'Policy interventions',
    metric: 'trace_count',
    chart_type: 'bar',
    group_by: 'policy_id',
    layout: { x: 6, y: 1, w: 6, h: 1 },
  },
  {
    id: 'review-outcomes',
    title: 'Review outcomes',
    metric: 'human_review_count',
    chart_type: 'donut',
    group_by: 'review_outcome',
    layout: { x: 0, y: 2, w: 6, h: 1 },
  },
  {
    id: 'false-positive-rate',
    title: 'False positive rate',
    metric: 'false_positive_rate',
    chart_type: 'big_number',
    group_by: null,
    layout: { x: 6, y: 2, w: 3, h: 1 },
  },
];

const DEFAULT_VIEW: AnalyticsDashboardView = {
  id: 'local-default',
  name: 'Default analytics',
  is_default: true,
  config: {
    filters: [],
    widgets: WIDGET_LIBRARY.slice(0, 4),
  },
  created_at: '',
  updated_at: '',
};

const chartConfig = {
  value: { label: 'Value', color: 'var(--chart-1)' },
} satisfies ChartConfig;

const DEFAULT_LAYOUT: AnalyticsWidgetLayout = { x: 0, y: 0, w: 6, h: 1 };
const WIDGET_SIZE_PRESETS = [
  { value: '3x1', label: 'Small', layout: { w: 3, h: 1 } },
  { value: '6x1', label: 'Medium', layout: { w: 6, h: 1 } },
  { value: '12x1', label: 'Wide', layout: { w: 12, h: 1 } },
  { value: '6x2', label: 'Tall', layout: { w: 6, h: 2 } },
  { value: '12x2', label: 'Large', layout: { w: 12, h: 2 } },
];

export function AnalyticsChartGrid({ workspaceSlug, catalog, savedViews }: AnalyticsChartGridProps) {
  const initialView = savedViews.find((view) => view.is_default) ?? savedViews[0] ?? DEFAULT_VIEW;
  const [views, setViews] = useState<AnalyticsDashboardView[]>(savedViews);
  const [selectedViewId, setSelectedViewId] = useState(initialView.id);
  const [viewName, setViewName] = useState(initialView.name);
  const [config, setConfig] = useState<AnalyticsDashboardViewConfig>(initialView.config);
  const [saveState, setSaveState] = useState<'idle' | 'saving' | 'saved' | 'error'>('idle');
  const sensors = useSensors(useSensor(PointerSensor, { activationConstraint: { distance: 6 } }));

  const selectedView = useMemo(
    () => views.find((view) => view.id === selectedViewId) ?? DEFAULT_VIEW,
    [selectedViewId, views],
  );

  function applyView(viewId: string) {
    const next = views.find((view) => view.id === viewId) ?? DEFAULT_VIEW;
    setSelectedViewId(next.id);
    setViewName(next.name);
    setConfig(next.config);
    setSaveState('idle');
  }

  function setFilter(dimension: AnalyticsDimension, value: string) {
    setConfig((current) => ({
      ...current,
      filters:
        value === 'all'
          ? current.filters.filter((filter) => filter.dimension !== dimension)
          : [
              ...current.filters.filter((filter) => filter.dimension !== dimension),
              { dimension, values: [value] },
            ],
    }));
    setSaveState('idle');
  }

  function setWidgetEnabled(widget: AnalyticsDashboardWidget, enabled: boolean) {
    setConfig((current) => ({
      ...current,
      widgets: enabled
        ? [...current.widgets, withLayout(widget, current.widgets.length)]
        : current.widgets.filter((existing) => existing.id !== widget.id),
    }));
    setSaveState('idle');
  }

  function reorderWidgets(event: DragEndEvent) {
    const { active, over } = event;
    if (!over || active.id === over.id) return;
    setConfig((current) => {
      const oldIndex = current.widgets.findIndex((widget) => widget.id === active.id);
      const newIndex = current.widgets.findIndex((widget) => widget.id === over.id);
      if (oldIndex < 0 || newIndex < 0) return current;
      return {
        ...current,
        widgets: applyGridOrder(arrayMove(current.widgets, oldIndex, newIndex)),
      };
    });
    setSaveState('idle');
  }

  function setWidgetLayout(widgetId: string, layout: AnalyticsWidgetLayout) {
    setConfig((current) => ({
      ...current,
      widgets: current.widgets.map((widget) =>
        widget.id === widgetId ? { ...widget, layout } : widget,
      ),
    }));
    setSaveState('idle');
  }

  async function saveView() {
    setSaveState('saving');
    const existing = views.find((view) => view.id === selectedViewId);
    const nextConfig = {
      ...config,
      widgets: applyGridOrder(config.widgets.map((widget, index) => withLayout(widget, index))),
    };
    const body = JSON.stringify({ name: viewName, config: nextConfig, is_default: existing?.is_default ?? false });
    const response = await fetch(
      existing
        ? `/api/analytics/views/${encodeURIComponent(existing.id)}?workspace=${encodeURIComponent(workspaceSlug)}`
        : `/api/analytics/views?workspace=${encodeURIComponent(workspaceSlug)}`,
      {
        method: existing ? 'PATCH' : 'POST',
        headers: { 'Content-Type': 'application/json' },
        body,
      },
    );
    if (!response.ok) {
      setSaveState('error');
      return;
    }
    const saved = (await response.json()) as AnalyticsDashboardView;
    setViews((current) =>
      current.some((view) => view.id === saved.id)
        ? current.map((view) => (view.id === saved.id ? saved : view))
        : [saved, ...current],
    );
    setSelectedViewId(saved.id);
    setViewName(saved.name);
    setConfig(saved.config);
    setSaveState('saved');
  }

  const visibleWidgets = applyGridOrder(config.widgets.map((widget, index) => withLayout(widget, index)));
  const activeWidgetIds = new Set(visibleWidgets.map((widget) => widget.id));

  return (
    <div className="grid gap-4">
      <Card>
        <CardHeader>
          <CardDescription>{selectedView.is_default ? 'Default view' : 'Saved view'}</CardDescription>
          <CardTitle>Analytics controls</CardTitle>
          <CardAction className="flex items-center gap-2">
            <Button onClick={() => void saveView()} disabled={saveState === 'saving'} size="sm">
              <Save className="size-4" />
              {saveState === 'saving' ? 'Saving' : 'Save'}
            </Button>
          </CardAction>
        </CardHeader>
        <CardContent className="grid gap-4">
          <div className="grid gap-3 md:grid-cols-[minmax(0,1fr)_minmax(0,1fr)_auto]">
            <Select value={selectedViewId} onValueChange={applyView}>
              <SelectTrigger aria-label="Saved analytics view">
                <SelectValue />
              </SelectTrigger>
              <SelectContent>
                <SelectItem value={DEFAULT_VIEW.id}>{DEFAULT_VIEW.name}</SelectItem>
                {views.map((view) => (
                  <SelectItem key={view.id} value={view.id}>
                    {view.name}
                  </SelectItem>
                ))}
              </SelectContent>
            </Select>
            <Input value={viewName} onChange={(event) => setViewName(event.target.value)} />
            <DropdownMenu>
              <DropdownMenuTrigger asChild>
                <Button variant="outline">
                  <Eye className="size-4" />
                  Widgets
                </Button>
              </DropdownMenuTrigger>
              <DropdownMenuContent align="end" className="w-56">
                {WIDGET_LIBRARY.map((widget) => (
                  <DropdownMenuCheckboxItem
                    key={widget.id}
                    checked={activeWidgetIds.has(widget.id)}
                    onCheckedChange={(checked) => setWidgetEnabled(widget, Boolean(checked))}
                  >
                    {widget.title}
                  </DropdownMenuCheckboxItem>
                ))}
              </DropdownMenuContent>
            </DropdownMenu>
          </div>

          <div className="grid gap-3 md:grid-cols-2 xl:grid-cols-4">
            {catalog.facets
              .filter((facet) => facet.values.length > 0)
              .slice(0, 8)
              .map((facet) => {
                const selected =
                  config.filters.find((filter) => filter.dimension === facet.dimension)?.values[0] ??
                  'all';
                return (
                  <div key={facet.dimension} className="grid gap-1.5">
                    <span className="text-xs font-medium text-muted-foreground">{facet.label}</span>
                    <Select
                      value={selected}
                      onValueChange={(value) => setFilter(facet.dimension, value)}
                    >
                      <SelectTrigger aria-label={facet.label}>
                        <SelectValue />
                      </SelectTrigger>
                      <SelectContent>
                        <SelectItem value="all">All</SelectItem>
                        {facet.values.map((value) => (
                          <SelectItem key={value} value={value}>
                            {value}
                          </SelectItem>
                        ))}
                      </SelectContent>
                    </Select>
                  </div>
                );
              })}
          </div>

          {saveState === 'saved' && <p className="text-sm text-muted-foreground">View saved.</p>}
          {saveState === 'error' && (
            <p className="text-sm text-destructive">Could not save this analytics view.</p>
          )}
        </CardContent>
      </Card>

      {visibleWidgets.length === 0 ? (
        <Card>
          <CardContent className="flex min-h-40 items-center justify-center text-sm text-muted-foreground">
            <EyeOff className="mr-2 size-4" />
            No analytics widgets selected.
          </CardContent>
        </Card>
      ) : (
        <DndContext
          sensors={sensors}
          collisionDetection={closestCenter}
          onDragEnd={reorderWidgets}
        >
          <SortableContext items={visibleWidgets.map((widget) => widget.id)} strategy={rectSortingStrategy}>
            <div className="grid gap-4 lg:grid-cols-12">
              {visibleWidgets.map((widget) => (
                <SortableAnalyticsWidget
                  key={widget.id}
                  workspaceSlug={workspaceSlug}
                  widget={widget}
                  filters={config.filters}
                  onLayoutChange={(layout) => setWidgetLayout(widget.id, layout)}
                />
              ))}
            </div>
          </SortableContext>
        </DndContext>
      )}
    </div>
  );
}

function SortableAnalyticsWidget({
  workspaceSlug,
  widget,
  filters,
  onLayoutChange,
}: {
  workspaceSlug: string;
  widget: AnalyticsDashboardWidget;
  filters: AnalyticsFilter[];
  onLayoutChange: (layout: AnalyticsWidgetLayout) => void;
}) {
  const { attributes, listeners, setNodeRef, transform, transition, isDragging } = useSortable({
    id: widget.id,
  });

  return (
    <div
      ref={setNodeRef}
      className={`${layoutSpanClass(widget.layout)} ${isDragging ? 'z-10 opacity-80' : ''}`}
      style={{
        transform: CSS.Transform.toString(transform),
        transition,
      }}
    >
      <AnalyticsWidget
        workspaceSlug={workspaceSlug}
        widget={widget}
        filters={filters}
        dragHandleProps={{ ...attributes, ...listeners }}
        onLayoutChange={onLayoutChange}
      />
    </div>
  );
}

function AnalyticsWidget({
  workspaceSlug,
  widget,
  filters,
  dragHandleProps,
  onLayoutChange,
}: {
  workspaceSlug: string;
  widget: AnalyticsDashboardWidget;
  filters: AnalyticsFilter[];
  dragHandleProps?: HTMLAttributes<HTMLButtonElement>;
  onLayoutChange: (layout: AnalyticsWidgetLayout) => void;
}) {
  const [data, setData] = useState<AnalyticsQueryResponse | null>(null);
  const [status, setStatus] = useState<'loading' | 'ready' | 'error'>('loading');
  const layout = normalizeLayout(widget.layout);
  const heightClass = layout.h > 1 ? 'h-[380px]' : 'h-[260px]';
  const sizeValue = `${layout.w}x${layout.h}`;

  useEffect(() => {
    let canceled = false;
    async function runQuery() {
      setStatus('loading');
      const request: AnalyticsQueryRequest = {
        metric: widget.metric,
        group_by: widget.group_by ?? null,
        filters,
        limit: 12,
      };
      const response = await fetch(
        `/api/analytics/query?workspace=${encodeURIComponent(workspaceSlug)}`,
        {
          method: 'POST',
          headers: { 'Content-Type': 'application/json' },
          body: JSON.stringify(request),
        },
      );
      if (canceled) return;
      if (!response.ok) {
        setStatus('error');
        return;
      }
      setData((await response.json()) as AnalyticsQueryResponse);
      setStatus('ready');
    }
    void runQuery();
    return () => {
      canceled = true;
    };
  }, [filters, widget.group_by, widget.metric, workspaceSlug]);

  return (
    <Card>
      <CardHeader>
        <CardDescription>{widget.group_by ? dimensionLabel(widget.group_by) : metricLabel(widget.metric)}</CardDescription>
        <CardTitle>{widget.title}</CardTitle>
        <CardAction className="flex items-center gap-2">
          <Select
            value={sizeValue}
            onValueChange={(value) => {
              const preset = WIDGET_SIZE_PRESETS.find((item) => item.value === value);
              if (!preset) return;
              onLayoutChange(normalizeLayout({ ...layout, ...preset.layout }));
            }}
          >
            <SelectTrigger size="sm" aria-label={`${widget.title} size`} className="w-28">
              <SelectValue />
            </SelectTrigger>
            <SelectContent>
              {WIDGET_SIZE_PRESETS.map((preset) => (
                <SelectItem key={preset.value} value={preset.value}>
                  {preset.label}
                </SelectItem>
              ))}
            </SelectContent>
          </Select>
          <Button
            type="button"
            variant="ghost"
            size="icon"
            className="size-8 text-muted-foreground"
            {...dragHandleProps}
          >
            <GripVertical className="size-4" aria-hidden="true" />
            <span className="sr-only">Drag to reorder {widget.title}</span>
          </Button>
        </CardAction>
      </CardHeader>
      <CardContent>
        {status === 'loading' && (
          <div className={`flex ${heightClass} items-center justify-center border text-sm text-muted-foreground`}>
            Loading analytics...
          </div>
        )}
        {status === 'error' && (
          <div className={`flex ${heightClass} items-center justify-center border text-sm text-destructive`}>
            Could not load analytics.
          </div>
        )}
        {status === 'ready' && data && <WidgetBody data={data} chartType={widget.chart_type} metric={widget.metric} heightClass={heightClass} />}
      </CardContent>
    </Card>
  );
}

function WidgetBody({
  data,
  chartType,
  metric,
  heightClass,
}: {
  data: AnalyticsQueryResponse;
  chartType: AnalyticsChartType;
  metric: AnalyticsMetric;
  heightClass: string;
}) {
  if (data.points.length === 0) {
    return (
      <div className={`flex ${heightClass} items-center justify-center border text-sm text-muted-foreground`}>
        No data for this selection.
      </div>
    );
  }
  if (chartType === 'big_number' || !data.group_by) {
    return (
      <div className={`flex ${heightClass} flex-col justify-center border p-6`}>
        <div className="text-4xl font-semibold tabular-nums">{formatMetricValue(metric, data.total)}</div>
        <div className="mt-2 text-sm text-muted-foreground">{metricLabel(metric)}</div>
      </div>
    );
  }
  if (chartType === 'donut') {
    return (
      <ChartContainer config={chartConfig} className={`aspect-auto ${heightClass} w-full`}>
        <PieChart>
          <ChartTooltip content={<ChartTooltipContent />} />
          <Pie data={data.points} dataKey="value" nameKey="label" innerRadius={58} outerRadius={92}>
            {data.points.map((point, index) => (
              <Cell key={point.label} fill={`var(--chart-${(index % 5) + 1})`} />
            ))}
          </Pie>
        </PieChart>
      </ChartContainer>
    );
  }
  if (chartType === 'line' || chartType === 'area') {
    return (
      <ChartContainer config={chartConfig} className={`aspect-auto ${heightClass} w-full`}>
        <LineChart data={data.points} margin={{ left: 0, right: 12 }}>
          <CartesianGrid vertical={false} />
          <XAxis dataKey="label" tickLine={false} axisLine={false} tickMargin={8} minTickGap={18} />
          <YAxis tickLine={false} axisLine={false} tickMargin={8} width={48} />
          <ChartTooltip content={<ChartTooltipContent />} />
          <Line dataKey="value" stroke="var(--color-value)" strokeWidth={2} dot={false} />
        </LineChart>
      </ChartContainer>
    );
  }
  if (chartType === 'table') {
    return (
      <div className={`grid ${heightClass} content-start gap-2 overflow-auto border p-4`}>
        {data.points.map((point) => (
          <div key={point.label} className="flex items-center justify-between gap-4 text-sm">
            <span className="text-muted-foreground">{point.label}</span>
            <span className="font-medium tabular-nums">{formatMetricValue(metric, point.value)}</span>
          </div>
        ))}
      </div>
    );
  }
  return (
    <ChartContainer config={chartConfig} className={`aspect-auto ${heightClass} w-full`}>
      <BarChart data={data.points} margin={{ left: 0, right: 12 }}>
        <CartesianGrid vertical={false} />
        <XAxis dataKey="label" tickLine={false} axisLine={false} tickMargin={8} minTickGap={18} />
        <YAxis tickLine={false} axisLine={false} tickMargin={8} width={48} />
        <ChartTooltip content={<ChartTooltipContent />} />
        <Bar dataKey="value" fill="var(--color-value)" radius={[4, 4, 0, 0]} />
      </BarChart>
    </ChartContainer>
  );
}

function metricLabel(metric: AnalyticsMetric): string {
  return metric
    .split('_')
    .map((part) => part.slice(0, 1).toUpperCase() + part.slice(1))
    .join(' ');
}

function dimensionLabel(dimension: AnalyticsDimension): string {
  return dimension
    .split('_')
    .map((part) => part.slice(0, 1).toUpperCase() + part.slice(1))
    .join(' ');
}

function formatMetricValue(metric: AnalyticsMetric, value: number): string {
  if (metric.endsWith('_rate')) return `${value.toFixed(value % 1 === 0 ? 0 : 1)}%`;
  if (metric === 'p95_latency_ms') return `${Math.round(value)}ms`;
  return String(Math.round(value));
}

function withLayout(widget: AnalyticsDashboardWidget, index: number): AnalyticsDashboardWidget {
  return {
    ...widget,
    layout: normalizeLayout(widget.layout ?? { ...DEFAULT_LAYOUT, y: index }),
  };
}

function normalizeLayout(layout?: AnalyticsWidgetLayout | null): AnalyticsWidgetLayout {
  const source = layout ?? DEFAULT_LAYOUT;
  const w = Math.min(Math.max(Math.round(source.w || DEFAULT_LAYOUT.w), 1), 12);
  const h = Math.min(Math.max(Math.round(source.h || DEFAULT_LAYOUT.h), 1), 4);
  const x = Math.min(Math.max(Math.round(source.x || 0), 0), 12 - w);
  const y = Math.max(Math.round(source.y || 0), 0);
  return { x, y, w, h };
}

function applyGridOrder(widgets: AnalyticsDashboardWidget[]): AnalyticsDashboardWidget[] {
  return widgets.map((widget, index) => ({
    ...widget,
    layout: {
      ...normalizeLayout(widget.layout),
      y: index,
    },
  }));
}

function layoutSpanClass(layout?: AnalyticsWidgetLayout): string {
  const width = normalizeLayout(layout).w;
  if (width <= 3) return 'lg:col-span-3';
  if (width <= 6) return 'lg:col-span-6';
  return 'lg:col-span-12';
}
