import type { ComponentType, ReactNode } from 'react';
import Link from 'next/link';
import {
  IconActivity,
  IconArrowRight,
  IconArrowUpRight,
  IconBook2,
  IconKey,
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
import { PageHeader } from '@/components/ui/page-header';
import { Separator } from '@/components/ui/separator';
import type { VariantProps } from 'class-variance-authority';
import type { WorkspaceDashboardData } from '@/lib/server/dashboard-data';

type BadgeVariant = NonNullable<VariantProps<typeof badgeVariants>['variant']>;
type VerdictVariant = Extract<BadgeVariant, 'allow' | 'rewrite' | 'block' | 'escalate'>;

const VERDICT_ORDER: VerdictVariant[] = ['allow', 'rewrite', 'escalate', 'block'];

/** Map a raw verdict string to its Badge verdict variant; `allow` is the safe default. */
function verdictVariant(value: string): VerdictVariant {
  const key = value.trim().toLowerCase();
  return (VERDICT_ORDER as string[]).includes(key) ? (key as VerdictVariant) : 'allow';
}

type DecisionRow = WorkspaceDashboardData['recentDecisions'][number];

const decisionColumns: DataTableColumn<DecisionRow>[] = [
  {
    id: 'verdict',
    header: 'Verdict',
    cell: (row) => <Badge variant={verdictVariant(row.verdict)}>{row.verdict}</Badge>,
  },
  {
    id: 'id',
    header: 'Trace',
    cell: (row) => row.id,
    cellClassName: 'font-mono text-xs text-muted-foreground',
  },
  { id: 'agent', header: 'Agent', cell: (row) => row.agent },
  {
    id: 'policy',
    header: 'Policy',
    cell: (row) => row.policy,
    cellClassName: 'font-mono text-xs',
  },
  { id: 'environment', header: 'Environment', cell: (row) => row.environment },
  {
    id: 'latency',
    header: 'Latency',
    align: 'right',
    cell: (row) => row.latency,
    cellClassName: 'font-data text-xs',
  },
  {
    id: 'time',
    header: 'Time',
    align: 'right',
    cell: (row) => row.time,
    cellClassName: 'text-muted-foreground',
  },
];

export function WorkspaceDashboard({ data }: { data: WorkspaceDashboardData }) {
  const setupItems = [
    {
      label: 'Policies enabled',
      value: `${data.activeWorkspace.enabledPolicies}/${data.activeWorkspace.policyCount}`,
      href: '/policies',
      icon: IconShieldCheck,
    },
    {
      label: 'Agents configured',
      value: String(data.activeWorkspace.agentCount),
      href: '/agents',
      icon: IconRobot,
    },
    {
      label: 'Knowledge sources',
      value: String(data.activeWorkspace.sourceCount),
      href: '/knowledge-sources',
      icon: IconBook2,
    },
    {
      label: 'Runtime keys',
      value: String(data.activeWorkspace.apiKeyCount),
      href: '/api-keys',
      icon: IconKey,
    },
  ];

  const verdictCounts = countVerdicts(data.recentDecisions);
  const sampleSize = data.recentDecisions.length;

  return (
    <div className="grid gap-6 px-4 lg:px-6">
      <PageHeader
        eyebrow={data.organization.name}
        title={data.activeWorkspace.name}
        description={`Live guardrail signal for ${data.activeEnvironment.name}. Review the latest decisions, then tune the policies behind them.`}
        actions={
          <>
            <Button asChild variant="outline">
              <Link href="/settings">Settings</Link>
            </Button>
            <Button asChild>
              <Link href="/policies">
                Tune policies
                <IconArrowRight />
              </Link>
            </Button>
          </>
        }
      />

      {/* Bento: decisions lead the surface, runtime signal sits in the right rail. */}
      <div className="grid gap-6 xl:grid-cols-[minmax(0,1.8fr)_minmax(300px,0.85fr)]">
        <Card className="overflow-hidden">
          <CardHeader className="border-b pb-6">
            <CardDescription>Live trace stream</CardDescription>
            <CardTitle className="flex items-center gap-2">
              <IconActivity className="size-5 text-primary" />
              Recent decisions
            </CardTitle>
            <CardAction>
              <Button asChild size="sm" variant="ghost">
                <Link href="/runs">
                  All runs
                  <IconArrowRight />
                </Link>
              </Button>
            </CardAction>
          </CardHeader>
          <CardContent className="pt-6">
            {sampleSize === 0 ? (
              <EmptyState
                icon={<IconActivity />}
                title="No decisions recorded yet"
                description="When your agents call the guardrail through the SDK, every allow, rewrite, escalate, and block lands here in real time."
                action={
                  <Button asChild size="sm" variant="outline">
                    <Link href="/api-keys">Connect an agent</Link>
                  </Button>
                }
              />
            ) : (
              <DataTable
                columns={decisionColumns}
                rows={data.recentDecisions}
                getRowKey={(decision) => decision.id}
                caption="Recent guardrail decisions"
                empty="No decisions recorded yet."
              />
            )}
          </CardContent>
        </Card>

        <div className="grid content-start gap-6">
          <Card>
            <CardHeader>
              <CardDescription>Decision mix</CardDescription>
              <CardTitle>Verdict distribution</CardTitle>
            </CardHeader>
            <CardContent className="grid gap-4">
              <VerdictBar counts={verdictCounts} total={sampleSize} />
              <dl className="grid gap-2">
                {VERDICT_ORDER.map((verdict) => (
                  <div key={verdict} className="flex items-center justify-between gap-3">
                    <dt>
                      <Badge variant={verdict}>{verdict}</Badge>
                    </dt>
                    <dd className="font-data text-sm tabular-nums text-foreground">
                      {verdictCounts[verdict]}
                    </dd>
                  </div>
                ))}
              </dl>
            </CardContent>
          </Card>

          <Card>
            <CardHeader>
              <CardDescription>Runtime health</CardDescription>
              <CardTitle>Active config</CardTitle>
            </CardHeader>
            <CardContent className="grid gap-3 text-sm">
              <ConfigRow label="Default action">
                <Badge variant={verdictVariant(data.settings.defaultAction)}>
                  {data.settings.defaultAction}
                </Badge>
              </ConfigRow>
              <Separator />
              <ConfigRow label="Escalation webhook">
                <Badge variant={data.settings.escalationWebhookUrl ? 'secondary' : 'outline'}>
                  {data.settings.escalationWebhookUrl ? 'configured' : 'not set'}
                </Badge>
              </ConfigRow>
              <Separator />
              <ConfigRow label="Trace retention">
                <span className="font-data tabular-nums text-foreground">
                  {data.settings.retentionDays} days
                </span>
              </ConfigRow>
              <Separator />
              <ConfigRow label="Key scope">
                <span className="text-foreground">{data.activeEnvironment.name}</span>
              </ConfigRow>
            </CardContent>
          </Card>
        </div>
      </div>

      {/* Key metrics — emphasized numerals, distinct from the shortcut row below. */}
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

      {/* Shortcuts — quieter than metrics; a way into configuration. */}
      <section aria-labelledby="dashboard-shortcuts" className="grid gap-3">
        <h2
          id="dashboard-shortcuts"
          className="text-xs font-medium tracking-wide text-muted-foreground uppercase"
        >
          Configure this workspace
        </h2>
        <div className="grid gap-3 sm:grid-cols-2 xl:grid-cols-4">
          {setupItems.map((item) => (
            <ShortcutTile key={item.label} {...item} />
          ))}
        </div>
      </section>
    </div>
  );
}

function ConfigRow({ label, children }: { label: string; children: ReactNode }) {
  return (
    <div className="flex items-center justify-between gap-3">
      <span className="text-muted-foreground">{label}</span>
      {children}
    </div>
  );
}

function ShortcutTile({
  label,
  value,
  href,
  icon: Icon,
}: {
  label: string;
  value: string;
  href: string;
  icon: ComponentType<{ className?: string }>;
}) {
  return (
    <Link
      href={href}
      className="group grid gap-4 rounded-lg border bg-card p-4 shadow-sm transition-colors hover:bg-accent/50 focus-visible:ring-[3px] focus-visible:ring-ring/50 focus-visible:outline-none"
    >
      <div className="flex items-center justify-between gap-3">
        <span className="flex size-8 items-center justify-center rounded-md bg-muted text-muted-foreground">
          <Icon className="size-4" />
        </span>
        <IconArrowUpRight className="size-4 text-muted-foreground transition-transform group-hover:-translate-y-0.5 group-hover:translate-x-0.5 motion-reduce:transform-none" />
      </div>
      <div className="grid gap-0.5">
        <span className="font-data text-2xl tabular-nums text-foreground">{value}</span>
        <span className="text-sm text-muted-foreground">{label}</span>
      </div>
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
      aria-label={VERDICT_ORDER.map((v) => `${counts[v]} ${v}`).join(', ')}
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
