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
import {
  CheckCircle2,
  Eye,
  GripVertical,
  LayoutGrid,
  ListFilter,
  Save,
  SlidersHorizontal,
  X,
} from 'lucide-react';

import { analyticsDashboardViewSchema, analyticsQueryResponseSchema } from '@/lib/analytics-schemas';
import { http } from '@/lib/http';
import { Button } from '@/components/ui/button';
import { Card, CardAction, CardContent, CardDescription, CardHeader, CardTitle } from '@/components/ui/card';
import { Badge } from '@/components/ui/badge';
import { EmptyState } from '@/components/ui/empty-state';
import { InfoHint } from '@/components/ui/info-hint';
import { VerdictLegend } from '@/components/ui/verdict-legend';
import { Label } from '@/components/ui/label';
import { Separator } from '@/components/ui/separator';
import { Skeleton } from '@/components/ui/skeleton';
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

// Plain-language help. Keep these readable to a non-technical teammate: a clear
// chart title, a one-line "what this tells you" caption, and—where a chart leans
// on jargon—an optional InfoHint tooltip. Keyed by widget id so copy stays with
// the widget it describes, not buried in render logic.
type WidgetCopy = {
  title: string;
  caption: string;
  hint?: string;
};

const WIDGET_COPY: Record<string, WidgetCopy> = {
  'trace-volume': {
    title: 'Requests by outcome',
    caption: 'How many requests were allowed, rewritten, flagged for review, or blocked.',
  },
  'intervention-rate': {
    title: 'How often we stepped in',
    caption: 'The share of requests the guardrail rewrote, flagged, or blocked instead of letting through.',
    hint: 'Out of every 100 requests, this many were changed or stopped. A higher number means the guardrail acted more often.',
  },
  'p95-latency': {
    title: 'Safety check speed',
    caption: 'How long the safety check took on a slow request. Lower is faster.',
    hint: 'p95 means 95 out of every 100 checks finished at least this fast — measured in milliseconds (1000 ms = 1 second).',
  },
  'agent-volume': {
    title: 'Requests by agent',
    caption: 'Which of your AI apps sent the most requests through the guardrail.',
  },
  'policy-interventions': {
    title: 'Which rules fired most',
    caption: 'How many requests each of your rules acted on.',
  },
  'review-outcomes': {
    title: 'What reviewers decided',
    caption: 'For requests a person looked at, how often they approved or rejected them.',
  },
  'false-positive-rate': {
    title: 'False alarms',
    caption: 'The share of flagged requests a reviewer later judged to be safe after all.',
    hint: 'A false alarm is when the guardrail stopped a request that turned out to be fine. Lower is better.',
  },
};

// Friendly names for the raw metric/dimension/enum strings the API returns, so a
// non-technical viewer never sees a snake_case token on an axis, legend, or label.
const METRIC_LABELS: Record<string, string> = {
  trace_count: 'Number of requests',
  allow_count: 'Allowed',
  block_count: 'Blocked',
  rewrite_count: 'Rewritten',
  escalate_count: 'Flagged for review',
  intervention_rate: 'Step-in rate',
  p95_latency_ms: 'Check speed (p95)',
  human_review_count: 'Reviewed by a person',
  human_intervention_rate: 'Reviewer step-in rate',
  false_positive_rate: 'False-alarm rate',
};

const DIMENSION_LABELS: Record<string, string> = {
  agent_id: 'Agent',
  environment: 'Environment',
  run_kind: 'Request type',
  run_status: 'Status',
  decision: 'Outcome',
  policy_id: 'Rule',
  workflow_step: 'Step',
  review_outcome: 'Review decision',
  external_id: 'Reference',
};

// Raw enum values that show up on axes and in filters. Map the ones a person
// would otherwise misread; anything not listed falls back to title-casing.
const VALUE_LABELS: Record<string, string> = {
  allow: 'Allowed',
  block: 'Blocked',
  rewrite: 'Rewritten',
  escalate: 'Flagged for review',
  approved: 'Approved',
  rejected: 'Rejected',
  pending: 'Awaiting review',
};

// Charts grouped by a verdict-style dimension get a VerdictLegend so the colors
// and outcome words are explained right next to them.
const VERDICT_DIMENSIONS = new Set<string>(['decision', 'review_outcome']);

function widgetTitle(widget: AnalyticsDashboardWidget): string {
  return WIDGET_COPY[widget.id]?.title ?? widget.title;
}

const DEFAULT_LAYOUT: AnalyticsWidgetLayout = { x: 0, y: 0, w: 6, h: 1 };
const WIDGET_SIZE_PRESETS = [
  { value: '3x1', label: 'Small', layout: { w: 3, h: 1 } },
  { value: '6x1', label: 'Medium', layout: { w: 6, h: 1 } },
  { value: '12x1', label: 'Wide', layout: { w: 12, h: 1 } },
  { value: '6x2', label: 'Tall', layout: { w: 6, h: 2 } },
  { value: '12x2', label: 'Large', layout: { w: 12, h: 2 } },
];

export function AnalyticsChartGrid({ catalog, savedViews }: AnalyticsChartGridProps) {
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
    const body = { name: viewName, config: nextConfig, is_default: existing?.is_default ?? false };
    let saved: AnalyticsDashboardView;
    try {
      saved = existing
        ? await http.patch(
            `/api/analytics/views/${encodeURIComponent(existing.id)}`,
            body,
            analyticsDashboardViewSchema,
          )
        : await http.post('/api/analytics/views', body, analyticsDashboardViewSchema);
    } catch {
      setSaveState('error');
      return;
    }
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

  const facets = catalog.facets.filter((facet) => facet.values.length > 0).slice(0, 8);
  const activeFilters = config.filters.filter((filter) => (filter.values[0] ?? '') !== '');
  const facetLabel = (dimension: AnalyticsDimension) =>
    facets.find((facet) => facet.dimension === dimension)?.label ?? dimensionLabel(dimension);

  return (
    <div className="grid gap-4">
      <Card>
        <CardHeader className="border-b border-border/60">
          <CardDescription className="flex items-center gap-2">
            <SlidersHorizontal className="size-3.5" aria-hidden="true" />
            {selectedView.is_default ? 'Default view' : 'Saved view'}
          </CardDescription>
          <CardTitle>Analytics controls</CardTitle>
          <CardAction className="flex items-center gap-3">
            {saveState === 'saved' ? (
              <span className="hidden items-center gap-1.5 text-xs font-medium text-[var(--color-allow)] sm:flex">
                <CheckCircle2 className="size-3.5" aria-hidden="true" />
                View saved
              </span>
            ) : null}
            <Button onClick={() => void saveView()} disabled={saveState === 'saving'} size="sm">
              <Save className="size-4" aria-hidden="true" />
              {saveState === 'saving' ? 'Saving' : 'Save view'}
            </Button>
          </CardAction>
        </CardHeader>
        <CardContent className="grid gap-5">
          <div className="grid gap-4 lg:grid-cols-[minmax(0,2fr)_minmax(0,1fr)_auto] lg:items-end">
            <div className="grid gap-1.5">
              <Label htmlFor="analytics-view" className="text-xs text-muted-foreground">
                Saved view
              </Label>
              <Select value={selectedViewId} onValueChange={applyView}>
                <SelectTrigger id="analytics-view" aria-label="Saved analytics view">
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
            </div>
            <div className="grid gap-1.5">
              <Label htmlFor="analytics-view-name" className="text-xs text-muted-foreground">
                View name
              </Label>
              <Input
                id="analytics-view-name"
                value={viewName}
                onChange={(event) => setViewName(event.target.value)}
                placeholder="Name this view"
              />
            </div>
            <div className="grid gap-1.5">
              <span className="text-xs font-medium text-muted-foreground">
                Widgets · {activeWidgetIds.size}/{WIDGET_LIBRARY.length}
              </span>
              <DropdownMenu>
                <DropdownMenuTrigger asChild>
                  <Button variant="outline" className="justify-start">
                    <Eye className="size-4" aria-hidden="true" />
                    Choose widgets
                  </Button>
                </DropdownMenuTrigger>
                <DropdownMenuContent align="end" className="w-56">
                  {WIDGET_LIBRARY.map((widget) => (
                    <DropdownMenuCheckboxItem
                      key={widget.id}
                      checked={activeWidgetIds.has(widget.id)}
                      onCheckedChange={(checked) => setWidgetEnabled(widget, Boolean(checked))}
                    >
                      {widgetTitle(widget)}
                    </DropdownMenuCheckboxItem>
                  ))}
                </DropdownMenuContent>
              </DropdownMenu>
            </div>
          </div>

          {facets.length > 0 ? (
            <>
              <Separator />
              <div className="grid gap-3">
                <div className="flex flex-wrap items-center justify-between gap-2">
                  <span className="flex items-center gap-2 text-xs font-medium text-muted-foreground">
                    <ListFilter className="size-3.5" aria-hidden="true" />
                    Filters
                  </span>
                  {activeFilters.length > 0 ? (
                    <div className="flex flex-wrap items-center gap-1.5">
                      {activeFilters.map((filter) => (
                        <Badge key={filter.dimension} variant="secondary" className="gap-1.5 pr-1.5">
                          <span className="text-muted-foreground">{facetLabel(filter.dimension)}:</span>
                          <span>{valueLabel(filter.values[0] ?? '')}</span>
                          <button
                            type="button"
                            onClick={() => setFilter(filter.dimension, 'all')}
                            aria-label={`Clear ${facetLabel(filter.dimension)} filter`}
                            className="rounded-sm text-muted-foreground transition-colors hover:text-foreground focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring"
                          >
                            <X className="size-3" aria-hidden="true" />
                          </button>
                        </Badge>
                      ))}
                    </div>
                  ) : (
                    <span className="text-xs text-muted-foreground">No filters applied</span>
                  )}
                </div>
                <div className="grid gap-3 md:grid-cols-2 xl:grid-cols-4">
                  {facets.map((facet) => {
                    const selected =
                      config.filters.find((filter) => filter.dimension === facet.dimension)?.values[0] ??
                      'all';
                    return (
                      <div key={facet.dimension} className="grid gap-1.5">
                        <Label
                          htmlFor={`analytics-filter-${facet.dimension}`}
                          className="text-xs text-muted-foreground"
                        >
                          {facet.label}
                        </Label>
                        <Select
                          value={selected}
                          onValueChange={(value) => setFilter(facet.dimension, value)}
                        >
                          <SelectTrigger id={`analytics-filter-${facet.dimension}`} aria-label={facet.label}>
                            <SelectValue />
                          </SelectTrigger>
                          <SelectContent>
                            <SelectItem value="all">All</SelectItem>
                            {facet.values.map((value) => (
                              <SelectItem key={value} value={value}>
                                {valueLabel(value)}
                              </SelectItem>
                            ))}
                          </SelectContent>
                        </Select>
                      </div>
                    );
                  })}
                </div>
              </div>
            </>
          ) : null}

          {saveState === 'error' && (
            <p className="text-sm text-destructive" role="alert">
              Could not save this analytics view. Check your connection and try again.
            </p>
          )}
        </CardContent>
      </Card>

      {visibleWidgets.length === 0 ? (
        <Card>
          <CardContent className="py-2">
            <EmptyState
              icon={<LayoutGrid />}
              title="No widgets on this view"
              description="Pick the charts and metrics you want to track, then save the view to keep this layout."
              action={
                <DropdownMenu>
                  <DropdownMenuTrigger asChild>
                    <Button>
                      <Eye className="size-4" aria-hidden="true" />
                      Choose widgets
                    </Button>
                  </DropdownMenuTrigger>
                  <DropdownMenuContent align="center" className="w-56">
                    {WIDGET_LIBRARY.map((widget) => (
                      <DropdownMenuCheckboxItem
                        key={widget.id}
                        checked={activeWidgetIds.has(widget.id)}
                        onCheckedChange={(checked) => setWidgetEnabled(widget, Boolean(checked))}
                      >
                        {widgetTitle(widget)}
                      </DropdownMenuCheckboxItem>
                    ))}
                  </DropdownMenuContent>
                </DropdownMenu>
              }
            />
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
  widget,
  filters,
  onLayoutChange,
}: {
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
        widget={widget}
        filters={filters}
        dragHandleProps={{ ...attributes, ...listeners }}
        onLayoutChange={onLayoutChange}
      />
    </div>
  );
}

function AnalyticsWidget({
  widget,
  filters,
  dragHandleProps,
  onLayoutChange,
}: {
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
  const copy = WIDGET_COPY[widget.id];
  const heading = copy?.title ?? widget.title;
  const caption = copy?.caption ?? metricLabel(widget.metric);
  const showVerdictLegend = widget.group_by != null && VERDICT_DIMENSIONS.has(widget.group_by);

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
      if (canceled) return;
      try {
        const result = await http.post('/api/analytics/query', request, analyticsQueryResponseSchema);
        if (canceled) return;
        setData(result);
        setStatus('ready');
      } catch {
        if (!canceled) setStatus('error');
      }
    }
    void runQuery();
    return () => {
      canceled = true;
    };
  }, [filters, widget.group_by, widget.metric]);

  return (
    <Card className="h-full transition-shadow duration-200 hover:shadow-sm">
      <CardHeader>
        <CardDescription>
          {widget.group_by ? dimensionLabel(widget.group_by) : metricLabel(widget.metric)}
        </CardDescription>
        <CardTitle className="flex items-center gap-1.5">
          {heading}
          {copy?.hint ? <InfoHint side="top">{copy.hint}</InfoHint> : null}
        </CardTitle>
        <p className="text-sm text-muted-foreground [text-wrap:pretty]">{caption}</p>
        <CardAction className="flex items-center gap-2">
          <Select
            value={sizeValue}
            onValueChange={(value) => {
              const preset = WIDGET_SIZE_PRESETS.find((item) => item.value === value);
              if (!preset) return;
              onLayoutChange(normalizeLayout({ ...layout, ...preset.layout }));
            }}
          >
            <SelectTrigger size="sm" aria-label={`${heading} size`} className="w-28">
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
            className="size-8 cursor-grab text-muted-foreground active:cursor-grabbing"
            {...dragHandleProps}
          >
            <GripVertical className="size-4" aria-hidden="true" />
            <span className="sr-only">Drag to reorder {heading}</span>
          </Button>
        </CardAction>
      </CardHeader>
      <CardContent>
        {status === 'loading' && <WidgetSkeleton chartType={widget.chart_type} heightClass={heightClass} />}
        {status === 'error' && (
          <div
            className={`flex ${heightClass} flex-col items-center justify-center gap-2 rounded-md border border-dashed bg-card/40 px-6 text-center`}
            role="alert"
          >
            <p className="text-sm font-medium text-destructive">Could not load this chart.</p>
            <p className="text-xs text-muted-foreground">Adjust the filters or reload the page to retry.</p>
          </div>
        )}
        {status === 'ready' && data && (
          <>
            <WidgetBody
              data={data}
              chartType={widget.chart_type}
              metric={widget.metric}
              heightClass={heightClass}
            />
            {showVerdictLegend && data.points.length > 0 ? (
              <VerdictLegend className="mt-4 border-t border-border/60 pt-4" />
            ) : null}
          </>
        )}
      </CardContent>
    </Card>
  );
}

function WidgetSkeleton({ chartType, heightClass }: { chartType: AnalyticsChartType; heightClass: string }) {
  if (chartType === 'big_number') {
    return (
      <div className={`flex ${heightClass} flex-col justify-center gap-3`}>
        <Skeleton className="h-12 w-32" />
        <Skeleton className="h-4 w-24" />
      </div>
    );
  }
  if (chartType === 'donut') {
    return (
      <div className={`flex ${heightClass} items-center justify-center`}>
        <Skeleton className="size-40 rounded-full" />
      </div>
    );
  }
  if (chartType === 'table') {
    return (
      <div className={`grid ${heightClass} content-start gap-2`}>
        {Array.from({ length: 6 }).map((_, index) => (
          <div key={index} className="flex items-center justify-between gap-4">
            <Skeleton className="h-4 w-32" />
            <Skeleton className="h-4 w-12" />
          </div>
        ))}
      </div>
    );
  }
  return (
    <div className={`flex ${heightClass} items-end gap-2 pb-2`}>
      {[64, 88, 52, 96, 72, 40, 80, 60].map((height, index) => (
        <Skeleton key={index} className="w-full rounded-sm" style={{ height: `${height}%` }} />
      ))}
    </div>
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
  if (data.points.length === 0 && data.total === 0) {
    return (
      <div
        className={`flex ${heightClass} flex-col items-center justify-center gap-1.5 rounded-md border border-dashed bg-card/40 px-6 text-center`}
      >
        <p className="text-sm font-medium text-foreground">No data yet</p>
        <p className="text-xs text-muted-foreground [text-wrap:pretty]">
          Once your agents send requests through the guardrail, this chart fills in automatically.
        </p>
      </div>
    );
  }
  if (data.points.length === 0) {
    return (
      <div
        className={`flex ${heightClass} flex-col items-center justify-center gap-1.5 rounded-md border border-dashed bg-card/40 px-6 text-center`}
      >
        <p className="text-sm font-medium text-foreground">Nothing matches these filters</p>
        <p className="text-xs text-muted-foreground [text-wrap:pretty]">
          Try clearing a filter above to see more of your data.
        </p>
      </div>
    );
  }
  if (chartType === 'big_number' || !data.group_by) {
    return (
      <div className={`flex ${heightClass} flex-col justify-center`}>
        <div className="font-mono text-4xl font-semibold tabular-nums text-foreground">
          {formatMetricValue(metric, data.total)}
        </div>
        <div className="mt-2 text-sm text-muted-foreground">{metricLabel(metric)}</div>
      </div>
    );
  }
  if (chartType === 'donut') {
    return (
      <ChartContainer config={chartConfig} className={`aspect-auto ${heightClass} w-full`}>
        <PieChart>
          <ChartTooltip content={<ChartTooltipContent labelFormatter={valueLabel} />} />
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
          <XAxis
            dataKey="label"
            tickLine={false}
            axisLine={false}
            tickMargin={8}
            minTickGap={18}
            tickFormatter={valueLabel}
          />
          <YAxis tickLine={false} axisLine={false} tickMargin={8} width={48} />
          <ChartTooltip content={<ChartTooltipContent labelFormatter={valueLabel} />} />
          <Line dataKey="value" stroke="var(--color-value)" strokeWidth={2} dot={false} />
        </LineChart>
      </ChartContainer>
    );
  }
  if (chartType === 'table') {
    return (
      <div className={`grid ${heightClass} content-start gap-1 overflow-auto`}>
        {data.points.map((point) => (
          <div
            key={point.label}
            className="flex items-center justify-between gap-4 rounded-md px-2 py-1.5 text-sm transition-colors hover:bg-muted/60"
          >
            <span className="truncate text-muted-foreground">{valueLabel(point.label)}</span>
            <span className="font-mono font-medium tabular-nums text-foreground">
              {formatMetricValue(metric, point.value)}
            </span>
          </div>
        ))}
      </div>
    );
  }
  return (
    <ChartContainer config={chartConfig} className={`aspect-auto ${heightClass} w-full`}>
      <BarChart data={data.points} margin={{ left: 0, right: 12 }}>
        <CartesianGrid vertical={false} />
        <XAxis
          dataKey="label"
          tickLine={false}
          axisLine={false}
          tickMargin={8}
          minTickGap={18}
          tickFormatter={valueLabel}
        />
        <YAxis tickLine={false} axisLine={false} tickMargin={8} width={48} />
        <ChartTooltip content={<ChartTooltipContent labelFormatter={valueLabel} />} />
        <Bar dataKey="value" fill="var(--color-value)" radius={[4, 4, 0, 0]} />
      </BarChart>
    </ChartContainer>
  );
}

function titleCase(value: string): string {
  return value
    .split('_')
    .map((part) => part.slice(0, 1).toUpperCase() + part.slice(1))
    .join(' ');
}

function metricLabel(metric: AnalyticsMetric): string {
  return METRIC_LABELS[metric] ?? titleCase(metric);
}

function dimensionLabel(dimension: AnalyticsDimension): string {
  return DIMENSION_LABELS[dimension] ?? titleCase(dimension);
}

// Turns a raw axis/legend/filter value into something a non-technical viewer
// reads cleanly. Leaves ids (agents, policies, references) as-is. Accepts the
// loosely-typed value Recharts hands its formatters and only maps strings.
function valueLabel(raw: unknown): string {
  if (typeof raw !== 'string') return raw == null ? '' : String(raw);
  return VALUE_LABELS[raw.toLowerCase()] ?? raw;
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
