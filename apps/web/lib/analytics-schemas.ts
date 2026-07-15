import { z } from 'zod';

const analyticsMetricSchema = z.enum([
  'trace_count',
  'permit_count',
  'deny_count',
  'transform_count',
  'require_approval_count',
  'defer_count',
  'intervention_rate',
  'p95_latency_ms',
  'human_review_count',
  'human_intervention_rate',
  'false_positive_rate',
]);

const analyticsDimensionSchema = z.enum([
  'agent_id',
  'environment',
  'run_kind',
  'run_status',
  'authorization_effect',
  'policy_id',
  'workflow_step',
  'review_outcome',
  'external_id',
]);

const analyticsChartTypeSchema = z.enum(['big_number', 'bar', 'line', 'area', 'donut', 'table']);

const analyticsFilterSchema = z.object({
  dimension: analyticsDimensionSchema,
  values: z.array(z.string()),
});

const analyticsWidgetLayoutSchema = z.object({
  x: z.number(),
  y: z.number(),
  w: z.number(),
  h: z.number(),
});

const analyticsDashboardWidgetSchema = z.object({
  id: z.string(),
  title: z.string(),
  metric: analyticsMetricSchema,
  chart_type: analyticsChartTypeSchema,
  group_by: analyticsDimensionSchema.nullable().optional(),
  layout: analyticsWidgetLayoutSchema.optional(),
});

const analyticsDashboardViewConfigSchema = z.object({
  filters: z.array(analyticsFilterSchema),
  widgets: z.array(analyticsDashboardWidgetSchema),
});

export const analyticsDashboardViewSchema = z.object({
  id: z.string(),
  name: z.string(),
  is_default: z.boolean(),
  config: analyticsDashboardViewConfigSchema,
  created_at: z.string(),
  updated_at: z.string(),
});

export const analyticsDashboardViewListSchema = z.object({
  views: z.array(analyticsDashboardViewSchema),
});

export const analyticsCatalogSchema = z.object({
  metrics: z.array(
    z.object({
      metric: analyticsMetricSchema,
      label: z.string(),
      default_chart_type: analyticsChartTypeSchema,
    }),
  ),
  dimensions: z.array(z.object({ dimension: analyticsDimensionSchema, label: z.string() })),
  chart_types: z.array(analyticsChartTypeSchema),
  facets: z.array(
    z.object({
      dimension: analyticsDimensionSchema,
      label: z.string(),
      values: z.array(z.string()),
    }),
  ),
});

const analyticsQueryPointSchema = z.object({
  label: z.string(),
  value: z.number(),
});

export const analyticsQueryResponseSchema = z.object({
  metric: analyticsMetricSchema,
  group_by: analyticsDimensionSchema.nullable().optional(),
  total: z.number(),
  points: z.array(analyticsQueryPointSchema),
});
