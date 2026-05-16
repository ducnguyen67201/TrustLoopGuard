import Link from 'next/link';
import {
  IconActivity,
  IconArrowRight,
  IconBook2,
  IconKey,
  IconRobot,
  IconShieldCheck,
} from '@tabler/icons-react';

import { Badge } from '@/components/ui/badge';
import { Button } from '@/components/ui/button';
import {
  Card,
  CardAction,
  CardContent,
  CardDescription,
  CardHeader,
  CardTitle,
} from '@/components/ui/card';
import {
  Table,
  TableBody,
  TableCell,
  TableHead,
  TableHeader,
  TableRow,
} from '@/components/ui/table';
import type { WorkspaceDashboardData } from '@/lib/server/dashboard-data';

const verdictClassName: Record<string, string> = {
  allow: 'border-[color:var(--color-allow)] text-[color:var(--color-allow)]',
  block: 'border-[color:var(--color-block)] text-[color:var(--color-block)]',
  escalate: 'border-[color:var(--color-escalate)] text-[color:var(--color-escalate)]',
  rewrite: 'border-[color:var(--color-rewrite)] text-[color:var(--color-rewrite)]',
};

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

  return (
    <div className="grid gap-4 px-4 lg:px-6">
      <div className="grid gap-4 xl:grid-cols-[minmax(0,1.6fr)_minmax(320px,0.8fr)]">
        <Card>
          <CardHeader>
            <CardDescription>{data.organization.name}</CardDescription>
            <CardTitle className="text-2xl">{data.activeWorkspace.name}</CardTitle>
            <CardAction>
              <Button asChild size="sm" variant="outline">
                <Link href="/settings">
                  Settings
                  <IconArrowRight />
                </Link>
              </Button>
            </CardAction>
          </CardHeader>
          <CardContent className="grid gap-3 md:grid-cols-2 xl:grid-cols-4">
            {setupItems.map((item) => (
              <Link
                key={item.label}
                href={item.href}
                className="grid gap-3 border bg-background p-4 transition-colors hover:bg-muted/40"
              >
                <div className="flex items-center justify-between gap-3">
                  <item.icon className="size-4 text-muted-foreground" />
                  <IconArrowRight className="size-4 text-muted-foreground" />
                </div>
                <div>
                  <div className="text-2xl font-semibold tabular-nums">{item.value}</div>
                  <div className="text-sm text-muted-foreground">{item.label}</div>
                </div>
              </Link>
            ))}
          </CardContent>
        </Card>

        <Card>
          <CardHeader>
            <CardDescription>Runtime health</CardDescription>
            <CardTitle>Active config</CardTitle>
          </CardHeader>
          <CardContent className="grid gap-4 text-sm">
            <div className="flex items-center justify-between gap-3">
              <span className="text-muted-foreground">Default action</span>
              <Badge variant="outline" className="rounded-sm">
                {data.settings.defaultAction}
              </Badge>
            </div>
            <div className="flex items-center justify-between gap-3">
              <span className="text-muted-foreground">Escalation webhook</span>
              <Badge variant="outline" className="rounded-sm">
                {data.settings.escalationWebhookUrl ? 'configured' : 'not set'}
              </Badge>
            </div>
            <div className="flex items-center justify-between gap-3">
              <span className="text-muted-foreground">Trace retention</span>
              <span>{data.settings.retentionDays} days</span>
            </div>
            <div className="flex items-center justify-between gap-3">
              <span className="text-muted-foreground">API key scope</span>
              <span>workspace</span>
            </div>
          </CardContent>
        </Card>
      </div>

      <div className="grid gap-4 md:grid-cols-2 xl:grid-cols-4">
        {data.metrics.map((metric) => (
          <Card key={metric.label}>
            <CardHeader>
              <CardDescription>{metric.label}</CardDescription>
              <CardTitle className="text-2xl tabular-nums">{metric.value}</CardTitle>
              <CardAction>
                <Badge variant="outline" className="rounded-sm">
                  {metric.delta}
                </Badge>
              </CardAction>
            </CardHeader>
            <CardContent className="text-sm text-muted-foreground">{metric.detail}</CardContent>
          </Card>
        ))}
      </div>

      <Card>
        <CardHeader>
          <CardDescription>Recent decisions</CardDescription>
          <CardTitle className="flex items-center gap-2">
            <IconActivity className="size-5" />
            Guardrail trace stream
          </CardTitle>
          <CardAction>
            <Button asChild size="sm" variant="outline">
              <Link href="/policies">Tune policies</Link>
            </Button>
          </CardAction>
        </CardHeader>
        <CardContent>
          <Table>
            <TableHeader>
              <TableRow>
                <TableHead>Trace</TableHead>
                <TableHead>Agent</TableHead>
                <TableHead>Verdict</TableHead>
                <TableHead>Policy</TableHead>
                <TableHead className="text-right">Latency</TableHead>
                <TableHead className="text-right">Time</TableHead>
              </TableRow>
            </TableHeader>
            <TableBody>
              {data.recentDecisions.map((decision) => (
                <TableRow key={decision.id}>
                  <TableCell className="font-mono text-xs">{decision.id}</TableCell>
                  <TableCell>{decision.agent}</TableCell>
                  <TableCell>
                    <Badge
                      variant="outline"
                      className={`rounded-sm ${verdictClassName[decision.verdict]}`}
                    >
                      {decision.verdict}
                    </Badge>
                  </TableCell>
                  <TableCell>{decision.policy}</TableCell>
                  <TableCell className="text-right tabular-nums">{decision.latency}</TableCell>
                  <TableCell className="text-right text-muted-foreground">{decision.time}</TableCell>
                </TableRow>
              ))}
            </TableBody>
          </Table>
        </CardContent>
      </Card>
    </div>
  );
}
