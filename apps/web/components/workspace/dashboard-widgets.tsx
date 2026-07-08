'use client';

import type { ComponentType, ReactNode } from 'react';
import { useEffect, useRef, useState } from 'react';
import Link from 'next/link';
import { toast } from 'sonner';
import type { LlmUsageBucket } from '@trustloopguard/sdk';
import {
  IconActivity,
  IconArrowRight,
  IconArrowUpRight,
  IconBook2,
  IconCheck,
  IconCopy,
  IconKey,
  IconPlugConnected,
  IconRobot,
  IconShieldCheck,
} from '@tabler/icons-react';

import { Badge, badgeVariants } from '@/components/ui/badge';
import { Button } from '@/components/ui/button';
import {
  Card,
  CardAction,
  CardContent,
  CardDescription,
  CardHeader,
  CardTitle,
} from '@/components/ui/card';
import { DataTable, type DataTableColumn } from '@/components/ui/data-table';
import { EmptyState } from '@/components/ui/empty-state';
import { InfoHint } from '@/components/ui/info-hint';
import { Separator } from '@/components/ui/separator';
import { VerdictLegend } from '@/components/ui/verdict-legend';
import { GLOSSARY } from '@/lib/glossary';
import type { VariantProps } from 'class-variance-authority';
import type { WorkspaceDashboardData } from '@/lib/server/dashboard-data';
import { UsageContent } from './UsageContent';
import type { UsagePeriod } from './usage-utils';

type BadgeVariant = NonNullable<VariantProps<typeof badgeVariants>['variant']>;
type VerdictVariant = Extract<BadgeVariant, 'allow' | 'rewrite' | 'block' | 'escalate'>;

const VERDICT_ORDER: VerdictVariant[] = ['allow', 'rewrite', 'escalate', 'block'];

/** Map a raw verdict string to its Badge verdict variant; `allow` is the safe default. */
function verdictVariant(value: string): VerdictVariant {
  const key = value.trim().toLowerCase();
  return (VERDICT_ORDER as string[]).includes(key) ? (key as VerdictVariant) : 'allow';
}

/** Friendly, capitalized label for a verdict instead of the raw lowercase enum. */
function verdictLabel(value: string): string {
  return GLOSSARY[verdictVariant(value)].label;
}

/** Last 8 characters of a trace UUID — enough to tell two requests apart at a glance. */
function shortTraceId(id: string): string {
  return id.length <= 8 ? id : `#${id.slice(-8)}`;
}

type DecisionRow = WorkspaceDashboardData['recentDecisions'][number];

export type DashboardUsageData = {
  dayBuckets: LlmUsageBucket[];
  principalBuckets: LlmUsageBucket[];
  modelBuckets: LlmUsageBucket[];
};

/** Props every dashboard widget receives — the full server-fetched dashboard payload. */
export type WidgetProps = {
  data: WorkspaceDashboardData;
  usage: DashboardUsageData;
  usagePeriod: UsagePeriod;
};

/**
 * The "Request" cell: a human-friendly short label for a trace, a one-tap copy
 * affordance for the full ID, and the relative time underneath — so a
 * non-technical reader never sees a bare UUID with no context.
 */
function TraceCell({ row }: { row: DecisionRow }) {
  const [copied, setCopied] = useState(false);
  const resetTimer = useRef<ReturnType<typeof setTimeout> | null>(null);

  useEffect(
    () => () => {
      if (resetTimer.current) clearTimeout(resetTimer.current);
    },
    [],
  );

  async function copyId() {
    try {
      await navigator.clipboard.writeText(row.id);
    } catch {
      toast.error("Couldn't copy — select the text and copy it manually.");
      return;
    }
    setCopied(true);
    if (resetTimer.current) clearTimeout(resetTimer.current);
    resetTimer.current = setTimeout(() => setCopied(false), 1600);
  }

  return (
    <div className="flex items-center gap-1.5">
      <span className="font-mono text-xs text-foreground" title={row.id}>
        {shortTraceId(row.id)}
      </span>
      <button
        type="button"
        onClick={copyId}
        aria-label={copied ? 'Request ID copied' : `Copy full request ID ${row.id}`}
        className="inline-flex size-6 shrink-0 items-center justify-center rounded-md text-muted-foreground/70 transition-colors hover:bg-accent hover:text-foreground focus-visible:text-foreground focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring/50 [&_svg]:size-3.5"
      >
        {copied ? <IconCheck className="text-[var(--color-allow)]" /> : <IconCopy />}
      </button>
    </div>
  );
}

const decisionColumns: DataTableColumn<DecisionRow>[] = [
  {
    id: 'verdict',
    header: (
      <span className="inline-flex items-center gap-1.5">
        Decision
        <InfoHint term="verdict" />
      </span>
    ),
    cell: (row) => <Badge variant={verdictVariant(row.verdict)}>{verdictLabel(row.verdict)}</Badge>,
  },
  {
    id: 'id',
    header: (
      <span className="inline-flex items-center gap-1.5">
        Request
        <InfoHint term="trace" />
      </span>
    ),
    cell: (row) => <TraceCell row={row} />,
  },
  { id: 'agent', header: 'Agent', cell: (row) => row.agent },
  {
    id: 'policy',
    header: (
      <span className="inline-flex items-center gap-1.5">
        Rule
        <InfoHint term="policy" />
      </span>
    ),
    cell: (row) => row.policy,
    cellClassName: 'font-mono text-xs',
  },
  { id: 'environment', header: 'Environment', cell: (row) => row.environment },
  {
    id: 'latency',
    header: (
      <span className="inline-flex items-center justify-end gap-1.5">
        Speed
        <InfoHint term="latency" />
      </span>
    ),
    align: 'right',
    cell: (row) => row.latency,
    cellClassName: 'font-data text-xs',
  },
  {
    id: 'time',
    header: 'When',
    align: 'right',
    cell: (row) => row.time,
    cellClassName: 'text-muted-foreground',
  },
];

type SetupItem = {
  label: string;
  why: string;
  value: string;
  href: string;
  cta: string;
  icon: ComponentType<{ className?: string }>;
};

function ConfigRow({
  label,
  hint,
  children,
}: {
  label: string;
  hint?: string;
  children: ReactNode;
}) {
  return (
    <div className="flex items-center justify-between gap-3">
      <span className="flex items-center gap-1.5 text-muted-foreground">
        {label}
        {hint ? <InfoHint>{hint}</InfoHint> : null}
      </span>
      {children}
    </div>
  );
}

function ShortcutTile({ label, why, value, href, cta, icon: Icon }: SetupItem) {
  return (
    <Link
      href={href}
      className="group grid gap-3 rounded-lg border bg-card p-4 shadow-sm transition-colors hover:bg-accent/50 focus-visible:ring-[3px] focus-visible:ring-ring/50 focus-visible:outline-none"
    >
      <div className="flex items-center justify-between gap-3">
        <span className="flex size-8 items-center justify-center rounded-md bg-muted text-muted-foreground">
          <Icon className="size-4" />
        </span>
        <IconArrowUpRight className="size-4 text-muted-foreground transition-transform group-hover:-translate-y-0.5 group-hover:translate-x-0.5 motion-reduce:transform-none" />
      </div>
      <div className="grid gap-1">
        <div className="flex items-baseline gap-2">
          <span className="font-data text-2xl tabular-nums text-foreground">{value}</span>
          <span className="text-sm font-medium text-foreground">{label}</span>
        </div>
        <p className="text-xs leading-relaxed text-muted-foreground [text-wrap:pretty]">{why}</p>
      </div>
      <span className="inline-flex items-center gap-1 text-xs font-medium text-primary">
        {cta}
        <IconArrowRight className="size-3.5 transition-transform group-hover:translate-x-0.5 motion-reduce:transform-none" />
      </span>
    </Link>
  );
}

type VerdictCounts = Record<VerdictVariant, number>;

function countVerdicts(decisions: DecisionRow[]): VerdictCounts {
  const counts: VerdictCounts = { allow: 0, rewrite: 0, escalate: 0, block: 0 };
  for (const decision of decisions) {
    counts[verdictVariant(decision.verdict)] += 1;
  }
  return counts;
}

function VerdictBar({ counts, total }: { counts: VerdictCounts; total: number }) {
  if (total === 0) {
    return (
      <div
        className="h-2 w-full rounded-full border border-dashed bg-muted/40"
        role="img"
        aria-label="No decisions in this sample yet"
      />
    );
  }

  return (
    <div
      className="flex h-2 w-full overflow-hidden rounded-full bg-muted"
      role="img"
      aria-label={VERDICT_ORDER.map((v) => `${counts[v]} ${GLOSSARY[v].label}`).join(', ')}
    >
      {VERDICT_ORDER.map((verdict) => {
        const share = counts[verdict] / total;
        if (share === 0) return null;
        return (
          <span
            key={verdict}
            className="h-full first:rounded-l-full last:rounded-r-full"
            style={{
              width: `${share * 100}%`,
              backgroundColor: `var(--color-${verdict})`,
            }}
          />
        );
      })}
    </div>
  );
}

// --- Widget render functions -------------------------------------------------

function RecentDecisionsWidget({ data }: WidgetProps): ReactNode {
  const sampleSize = data.recentDecisions.length;
  return (
    <Card className="overflow-hidden">
      <CardHeader className="border-b pb-6">
        <CardDescription>What just happened</CardDescription>
        <CardTitle className="flex items-center gap-2">
          <IconActivity className="size-5 text-primary" />
          Recent decisions
        </CardTitle>
        {sampleSize > 0 ? (
          <CardAction>
            <Button asChild size="sm" variant="ghost">
              <Link href="/runs">
                See all activity
                <IconArrowRight />
              </Link>
            </Button>
          </CardAction>
        ) : null}
      </CardHeader>
      <CardContent className="grid gap-5 pt-6">
        {sampleSize === 0 ? (
          <EmptyState
            icon={<IconPlugConnected />}
            title="Let's see your first decision"
            description="Nothing has been checked yet. Connect one of your AI apps, and every request it makes will appear here — clearly marked as allowed, cleaned up, held for review, or blocked."
            action={
              <Button asChild size="sm">
                <Link
                  href={`/onboarding/connect?workspace=${encodeURIComponent(data.activeWorkspace.slug)}&environment=${encodeURIComponent(data.activeEnvironment.id)}`}
                >
                  Connect your first agent
                  <IconArrowRight />
                </Link>
              </Button>
            }
          />
        ) : (
          <>
            <DataTable
              columns={decisionColumns}
              rows={data.recentDecisions}
              getRowKey={(decision) => decision.id}
              caption="Recent guardrail decisions"
              empty="No decisions recorded yet."
            />
            <div className="rounded-lg border bg-muted/30 px-4 py-3">
              <p className="mb-2 text-xs font-medium text-muted-foreground">
                What each decision means
              </p>
              <VerdictLegend />
            </div>
          </>
        )}
      </CardContent>
    </Card>
  );
}

function DecisionMixWidget({ data }: WidgetProps): ReactNode {
  const verdictCounts = countVerdicts(data.recentDecisions);
  const sampleSize = data.recentDecisions.length;
  return (
    <Card>
      <CardHeader>
        <CardDescription>How requests are landing</CardDescription>
        <CardTitle className="flex items-center gap-1.5">
          Decision mix
          <InfoHint term="verdict" />
        </CardTitle>
      </CardHeader>
      <CardContent className="grid gap-4">
        <VerdictBar counts={verdictCounts} total={sampleSize} />
        <dl className="grid gap-2">
          {VERDICT_ORDER.map((verdict) => (
            <div key={verdict} className="flex items-center justify-between gap-3">
              <dt>
                <Badge variant={verdict}>{GLOSSARY[verdict].label}</Badge>
              </dt>
              <dd className="font-data text-sm tabular-nums text-foreground">
                {verdictCounts[verdict]}
              </dd>
            </div>
          ))}
        </dl>
        {sampleSize === 0 ? (
          <p className="text-xs text-muted-foreground [text-wrap:pretty]">
            This fills in once your agents start sending requests.
          </p>
        ) : null}
      </CardContent>
    </Card>
  );
}

function GuardrailConfigWidget({ data }: WidgetProps): ReactNode {
  return (
    <Card>
      <CardHeader>
        <CardDescription>Your current setup</CardDescription>
        <CardTitle>How the guardrail behaves</CardTitle>
      </CardHeader>
      <CardContent className="grid gap-3 text-sm">
        <ConfigRow
          label="If no rule applies"
          hint="What happens to a request when none of your rules have an opinion about it."
        >
          <Badge variant={verdictVariant(data.settings.defaultAction)}>
            {verdictLabel(data.settings.defaultAction)}
          </Badge>
        </ConfigRow>
        <Separator />
        <ConfigRow
          label="Alert when held for review"
          hint="Whether we ping an outside system the moment a request needs a person to look at it."
        >
          <Badge variant={data.settings.escalationWebhookUrl ? 'secondary' : 'outline'}>
            {data.settings.escalationWebhookUrl ? 'On' : 'Off'}
          </Badge>
        </ConfigRow>
        <Separator />
        <ConfigRow
          label="Keep records for"
          hint="How long we store each request's decision before it is removed."
        >
          <span className="font-data tabular-nums text-foreground">
            {data.settings.retentionDays} days
          </span>
        </ConfigRow>
        <Separator />
        <ConfigRow
          label="Showing data from"
          hint="The space these decisions belong to — like testing or live traffic."
        >
          <span className="text-foreground">{data.activeEnvironment.name}</span>
        </ConfigRow>
      </CardContent>
    </Card>
  );
}

function UsageWidget({ data, usage, usagePeriod }: WidgetProps): ReactNode {
  return (
    <UsageContent
      workspaceSlug={data.activeWorkspace.slug}
      environmentId={data.activeEnvironment.id}
      period={usagePeriod}
      dayBuckets={usage.dayBuckets}
      principalBuckets={usage.principalBuckets}
      modelBuckets={usage.modelBuckets}
    />
  );
}

function MetricsWidget({ data }: WidgetProps): ReactNode {
  return (
    <div className="grid gap-4 sm:grid-cols-2 xl:grid-cols-4">
      {data.metrics.map((metric) => (
        <Card key={metric.label} className="gap-0 py-5">
          <CardHeader className="px-5">
            <CardDescription>{metric.label}</CardDescription>
            <CardAction>
              <Badge variant="outline" className="font-mono text-[0.7rem]">
                {metric.delta}
              </Badge>
            </CardAction>
            <CardTitle className="font-data text-3xl tabular-nums">{metric.value}</CardTitle>
          </CardHeader>
          <CardContent className="px-5 pt-2 text-xs text-muted-foreground">
            {metric.detail}
          </CardContent>
        </Card>
      ))}
    </div>
  );
}

function SetupShortcutsWidget({ data }: WidgetProps): ReactNode {
  const setupItems: SetupItem[] = [
    {
      label: 'Protection rules',
      why: 'Rules decide what your AI is allowed to say or do.',
      value: `${data.activeWorkspace.enabledPolicies}/${data.activeWorkspace.policyCount}`,
      href: '/policies',
      cta: 'Review rules',
      icon: IconShieldCheck,
    },
    {
      label: 'Connected agents',
      why: 'Each agent is an AI app whose requests get checked here.',
      value: String(data.activeWorkspace.agentCount),
      href: '/agents',
      cta: 'Manage agents',
      icon: IconRobot,
    },
    {
      label: 'Trusted content',
      why: 'Approved docs your guardrail treats as a source of truth.',
      value: String(data.activeWorkspace.sourceCount),
      href: '/knowledge-sources',
      cta: 'Add content',
      icon: IconBook2,
    },
    {
      label: 'Connection keys',
      why: 'Secret keys let your apps connect to the guardrail safely.',
      value: String(data.activeWorkspace.apiKeyCount),
      href: '/api-keys',
      cta: 'Manage keys',
      icon: IconKey,
    },
  ];

  return (
    <section aria-labelledby="dashboard-shortcuts" className="grid gap-3">
      <div className="grid gap-1">
        <h2
          id="dashboard-shortcuts"
          className="text-xs font-medium tracking-wide text-muted-foreground uppercase"
        >
          Set up your protection
        </h2>
        <p className="text-sm text-muted-foreground">
          Four things to check so your guardrail covers everything it should.
        </p>
      </div>
      <div className="grid gap-3 sm:grid-cols-2 xl:grid-cols-4">
        {setupItems.map((item) => (
          <ShortcutTile key={item.label} {...item} />
        ))}
      </div>
    </section>
  );
}

// --- Registry ----------------------------------------------------------------

export type WidgetKey =
  | 'recent-decisions'
  | 'decision-mix'
  | 'guardrail-config'
  | 'usage'
  | 'metrics'
  | 'setup-shortcuts';

export type DashboardWidget = {
  key: WidgetKey;
  /** Human label shown in the Customize menu. */
  title: string;
  /** Coarse width in the 2-column grid: `full` spans both columns. */
  span: 'full' | 'half';
  render: (props: WidgetProps) => ReactNode;
};

/** Default dashboard layout: the order and set of widgets shown out of the box. */
export const DASHBOARD_WIDGETS: readonly DashboardWidget[] = [
  { key: 'recent-decisions', title: 'Recent decisions', span: 'full', render: RecentDecisionsWidget },
  { key: 'decision-mix', title: 'Decision mix', span: 'half', render: DecisionMixWidget },
  { key: 'guardrail-config', title: 'How the guardrail behaves', span: 'half', render: GuardrailConfigWidget },
  { key: 'usage', title: 'LLM usage', span: 'full', render: UsageWidget },
  { key: 'metrics', title: 'Key metrics', span: 'full', render: MetricsWidget },
  { key: 'setup-shortcuts', title: 'Set up your protection', span: 'full', render: SetupShortcutsWidget },
];

export const DASHBOARD_WIDGET_KEYS: readonly WidgetKey[] = DASHBOARD_WIDGETS.map((w) => w.key);
