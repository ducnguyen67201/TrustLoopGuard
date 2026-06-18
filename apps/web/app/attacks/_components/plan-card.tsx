'use client';

import { Crosshair, Loader2, ShieldPlus, Workflow } from 'lucide-react';

import { Badge } from '@/components/ui/badge';
import { Button } from '@/components/ui/button';
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from '@/components/ui/card';
import { Label } from '@/components/ui/label';
import { cn } from '@/lib/utils';
import type { AgentSummary } from '@/lib/agents';
import type { RedteamPlan, StaticPoliciesResult } from '@/lib/redteam-plan';

interface PlanCardProps {
  agents: readonly AgentSummary[];
  selectedAgentId: string | null;
  onSelectAgent: (id: string | null) => void;
  plan: RedteamPlan | null;
  planning: boolean;
  planError: string | null;
  onPlan: () => void;
  staticBusy: boolean;
  staticCount: number | null;
  onGenerateStatic: () => void;
  busy: boolean;
}

/**
 * Step 1 of the hardening loop: pick an imported agent and derive tailored
 * attack vectors from its own definition. The planned vectors seed the next
 * attack (so it is gray-box, not generic) and the agent receives the verified
 * policies that harden produces.
 */
export function PlanCard({
  agents,
  selectedAgentId,
  onSelectAgent,
  plan,
  planning,
  planError,
  onPlan,
  staticBusy,
  staticCount,
  onGenerateStatic,
  busy,
}: PlanCardProps) {
  const selected = agents.find((a) => a.agentId === selectedAgentId) ?? null;
  const canPlan = selected !== null && !planning && !busy;

  return (
    <Card className="min-w-0 border-primary/30">
      <CardHeader>
        <CardTitle className="flex items-center gap-2">
          <Crosshair className="size-5 text-primary" aria-hidden="true" />
          Tailor the attack
        </CardTitle>
        <CardDescription>
          Plan attacks from an imported agent&apos;s own definition. The vectors seed the run and the
          agent receives the policies that land.
        </CardDescription>
      </CardHeader>
      <CardContent className="grid gap-4">
        <div className="grid gap-2">
          <Label htmlFor="plan-agent">Agent</Label>
          <select
            id="plan-agent"
            value={selectedAgentId ?? ''}
            disabled={busy}
            onChange={(e) => onSelectAgent(e.target.value === '' ? null : e.target.value)}
            className="h-9 rounded-md border bg-background px-3 text-sm disabled:opacity-60"
          >
            <option value="">No agent — generic attack</option>
            {agents.map((agent) => (
              <option key={agent.agentId} value={agent.agentId}>
                {agent.displayName}
                {agent.hasWorkflow ? ' · workflow' : agent.hasSystemPrompt ? ' · chat' : ''}
              </option>
            ))}
          </select>
        </div>

        <div className="flex flex-wrap items-center gap-2">
          <Button type="button" onClick={onPlan} disabled={!canPlan} className="gap-2">
            {planning ? (
              <Loader2 className="size-4 animate-spin" aria-hidden="true" />
            ) : (
              <Crosshair className="size-4" aria-hidden="true" />
            )}
            {planning ? 'Planning…' : 'Plan tailored attacks'}
          </Button>
          {plan !== null && plan.paths.length > 0 ? (
            <Button
              type="button"
              variant="outline"
              onClick={onGenerateStatic}
              disabled={staticBusy || busy}
              className="gap-2"
            >
              {staticBusy ? (
                <Loader2 className="size-4 animate-spin" aria-hidden="true" />
              ) : (
                <ShieldPlus className="size-4" aria-hidden="true" />
              )}
              Preventive policies
            </Button>
          ) : null}
        </div>

        {planError ? (
          <p className="rounded-md border border-destructive/40 bg-destructive/10 px-3 py-2 text-sm text-destructive">
            {planError}
          </p>
        ) : null}

        {staticCount !== null ? (
          <p className="rounded-md border border-emerald-500/40 bg-emerald-500/10 px-3 py-2 text-sm text-emerald-700">
            Attached {staticCount} preventive {staticCount === 1 ? 'policy' : 'policies'} to{' '}
            {selected?.displayName ?? 'the agent'} (disabled — enable from Policies).
          </p>
        ) : null}

        {plan !== null ? <PlanDetail plan={plan} /> : null}
      </CardContent>
    </Card>
  );
}

function PlanDetail({ plan }: { plan: RedteamPlan }) {
  return (
    <div className="grid gap-4">
      <div className="flex flex-wrap items-center gap-2 text-xs text-muted-foreground">
        <Badge variant="outline" className="gap-1">
          <Crosshair className="size-3" aria-hidden="true" />
          {plan.vectors.length} tailored {plan.vectors.length === 1 ? 'vector' : 'vectors'}
        </Badge>
        {plan.paths.length > 0 ? (
          <Badge variant="outline" className="gap-1">
            <Workflow className="size-3" aria-hidden="true" />
            {plan.paths.length} source→sink {plan.paths.length === 1 ? 'path' : 'paths'}
          </Badge>
        ) : null}
      </div>

      {plan.paths.length > 0 ? (
        <ul className="grid gap-1.5">
          {plan.paths.map((path, index) => (
            <li
              key={`${path.source_node}-${path.sink_node}-${index}`}
              className="flex flex-wrap items-center gap-2 font-mono text-xs"
            >
              <span className="rounded bg-amber-500/15 px-1.5 py-0.5 text-amber-700">
                {path.source_node} [{path.source_category}]
              </span>
              <span aria-hidden="true" className="text-muted-foreground">
                →
              </span>
              <span className="rounded bg-destructive/15 px-1.5 py-0.5 text-destructive">
                {path.sink_node} [{path.sink_category}]
              </span>
            </li>
          ))}
        </ul>
      ) : null}

      <ul className="grid gap-2">
        {plan.vectors.map((vector, index) => (
          <li key={index} className="grid gap-1 rounded-md border bg-muted/30 px-3 py-2">
            <div className="flex flex-wrap items-center gap-2">
              <Badge variant="secondary" className="font-mono text-[10px] uppercase">
                {vector.technique.replaceAll('_', ' ')}
              </Badge>
              <span className="font-mono text-[11px] text-muted-foreground">
                → {vector.target_operation}
              </span>
            </div>
            <p className="text-sm">{vector.goal}</p>
          </li>
        ))}
      </ul>

      {plan.unmapped_node_types.length > 0 ? (
        <p className={cn('text-xs text-muted-foreground')}>
          Unmapped node types (not analysed): {plan.unmapped_node_types.join(', ')}
        </p>
      ) : null}
    </div>
  );
}
