import type { ChartConfig } from '@/components/ui/chart';

// Shared config so every chart that shows verdict categories uses the same colors.
// Matches the color vars from the existing RunAnalyticsDashboard stacked bar.
export const verdictConfig = {
  allowed:   { label: 'Allowed',   color: 'var(--chart-2)' },
  blocked:   { label: 'Blocked',   color: 'var(--chart-5)' },
  rewritten: { label: 'Rewritten', color: 'var(--chart-4)' },
  escalated: { label: 'Escalated', color: 'var(--chart-1)' },
} satisfies ChartConfig;

export const latencyConfig = {
  p95Ms: { label: 'p95 Latency (ms)', color: 'var(--chart-3)' },
} satisfies ChartConfig;

export const interventionRateConfig = {
  rate: { label: 'Intervention Rate %', color: 'var(--chart-1)' },
} satisfies ChartConfig;
