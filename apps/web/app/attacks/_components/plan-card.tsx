'use client';

import { Crosshair, Loader2, ShieldPlus, Trash2, Workflow } from 'lucide-react';

import { Badge } from '@/components/ui/badge';
import { Button } from '@/components/ui/button';
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from '@/components/ui/card';
import { Input } from '@/components/ui/input';
import { Label } from '@/components/ui/label';
import { cn } from '@/lib/utils';
import type { RedteamPlan } from '@/lib/redteam-plan';

interface PlanStepProps {
  /** True once an agent is chosen — plans are per-agent, so generic runs skip this. */
  agentSelected: boolean;
  agentName: string | null;
  planName: string;
  onPlanNameChange: (value: string) => void;
  plan: RedteamPlan | null;
  planning: boolean;
  planError: string | null;
  onPlan: () => void;
  savedPlans: readonly RedteamPlan[];
  onSelectPlan: (plan: RedteamPlan) => void;
  onDeletePlan: (planId: string) => void;
  staticBusy: boolean;
  staticCount: number | null;
  onGenerateStatic: () => void;
  busy: boolean;
}

/**
 * Step 2 of the flow: pick a saved plan or generate a tailored one from the
 * agent's own definition. The active plan's vectors seed the next attack (so it
 * is gray-box) and the agent receives the verified policies that land. Rendered
 * inside the single stepped flow card in {@link ./attacks-panel}, not as its own
 * standalone Card — the parent owns the chrome so the three steps read as one
 * sequence.
 */
export function PlanStep({
  agentSelected,
  agentName,
  planName,
  onPlanNameChange,
  plan,
  planning,
  planError,
  onPlan,
  savedPlans,
  onSelectPlan,
  onDeletePlan,
  staticBusy,
  staticCount,
  onGenerateStatic,
  busy,
}: PlanStepProps) {
  const canPlan = agentSelected && !planning && !busy;
  const activePlanId = plan?.id ?? null;

  // No agent chosen: planning is per-agent, so a generic run can't tailor. Say so
  // plainly instead of showing dead controls.
  if (!agentSelected) {
    return (
      <p className="text-xs leading-relaxed text-muted-foreground">
        Pick an agent above to plan tailored attacks from its definition. A generic run skips
        planning and fires the default attack set at the URL.
      </p>
    );
  }

  return (
    <div className="grid gap-3">
      {savedPlans.length > 0 ? (
        <div className="grid gap-1.5">
          <span className="text-[11px] font-medium tracking-wide text-muted-foreground uppercase">
            Saved plans · pick one to seed
          </span>
          <ul className="grid gap-1">
            {savedPlans.map((saved) => {
              const active = saved.id === activePlanId;
              return (
                <li
                  key={saved.id}
                  className={cn(
                    'group flex items-center gap-2 rounded-md border px-2.5 py-1.5 transition-colors',
                    active
                      ? 'border-primary/60 bg-primary/5'
                      : 'border-border hover:border-foreground/20 hover:bg-accent/50',
                  )}
                >
                  <button
                    type="button"
                    onClick={() => onSelectPlan(saved)}
                    disabled={busy}
                    aria-pressed={active}
                    className="grid flex-1 text-left disabled:opacity-60"
                  >
                    <span className="flex items-center gap-1.5">
                      <span
                        className={cn(
                          'size-1.5 rounded-full',
                          active ? 'bg-primary' : 'bg-muted-foreground/40',
                        )}
                        aria-hidden="true"
                      />
                      <span className="truncate text-sm font-medium">{saved.name}</span>
                    </span>
                    <span className="pl-3 font-mono text-[11px] text-muted-foreground">
                      {saved.vectors.length} vectors · {saved.paths.length} paths
                    </span>
                  </button>
                  <button
                    type="button"
                    onClick={() => onDeletePlan(saved.id)}
                    disabled={busy}
                    aria-label={`Delete plan ${saved.name}`}
                    className="rounded p-1 text-muted-foreground opacity-0 transition-[color,opacity] group-hover:opacity-100 hover:text-destructive focus-visible:opacity-100 disabled:opacity-60"
                  >
                    <Trash2 className="size-4" aria-hidden="true" />
                  </button>
                </li>
              );
            })}
          </ul>
        </div>
      ) : null}

      <div className="grid gap-2">
        <Label htmlFor="plan-name" className="text-[11px] tracking-wide text-muted-foreground uppercase">
          {savedPlans.length > 0 ? 'Or generate a new plan' : 'Name a new plan'}
        </Label>
        <div className="flex flex-wrap items-center gap-2">
          <Input
            id="plan-name"
            value={planName}
            onChange={(e) => onPlanNameChange(e.target.value)}
            placeholder="e.g. Nightly sweep"
            disabled={busy}
            className="h-8 min-w-0 flex-1 text-sm"
          />
          <Button
            type="button"
            size="sm"
            variant="secondary"
            onClick={onPlan}
            disabled={!canPlan}
            className="gap-1.5"
          >
            {planning ? (
              <Loader2 className="size-3.5 animate-spin" aria-hidden="true" />
            ) : (
              <Crosshair className="size-3.5" aria-hidden="true" />
            )}
            {planning ? 'Planning…' : 'Plan tailored attacks'}
          </Button>
        </div>
      </div>

      {plan !== null && plan.paths.length > 0 ? (
        <Button
          type="button"
          variant="ghost"
          size="sm"
          onClick={onGenerateStatic}
          disabled={staticBusy || busy}
          className="justify-self-start gap-1.5 text-muted-foreground hover:text-foreground"
        >
          {staticBusy ? (
            <Loader2 className="size-3.5 animate-spin" aria-hidden="true" />
          ) : (
            <ShieldPlus className="size-3.5" aria-hidden="true" />
          )}
          Preventive policies
        </Button>
      ) : null}

      {planError ? (
        <p className="rounded-md border border-destructive/40 bg-destructive/10 px-3 py-2 text-sm text-destructive">
          {planError}
        </p>
      ) : null}

      {staticCount !== null ? (
        <p className="rounded-md border border-emerald-500/40 bg-emerald-500/10 px-3 py-2 text-xs text-emerald-700">
          Attached {staticCount} preventive {staticCount === 1 ? 'policy' : 'policies'} to{' '}
          {agentName ?? 'the agent'} (disabled — enable from Policies).
        </p>
      ) : null}

      {plan !== null ? <PlanSummary plan={plan} /> : null}
    </div>
  );
}

/** Compact summary that stays in the narrow left column: counts, the source→sink
 *  classes, and any unmapped nodes. The full vector detail and the "next step"
 *  hint render in the wide right pane via {@link PlanVectors}. */
function PlanSummary({ plan }: { plan: RedteamPlan }) {
  return (
    <div className="grid gap-2 rounded-md border border-primary/30 bg-primary/[0.04] px-3 py-2.5">
      <div className="flex flex-wrap items-center gap-1.5 text-xs text-muted-foreground">
        <Badge variant="outline" className="gap-1 border-primary/40 text-foreground">
          <Crosshair className="size-3 text-primary" aria-hidden="true" />
          {plan.vectors.length} {plan.vectors.length === 1 ? 'vector' : 'vectors'} ready
        </Badge>
        {plan.paths.length > 0 ? (
          <Badge variant="outline" className="gap-1">
            <Workflow className="size-3" aria-hidden="true" />
            {plan.paths.length} {plan.paths.length === 1 ? 'path' : 'paths'}
          </Badge>
        ) : null}
      </div>

      {plan.paths.length > 0 ? (
        <ul className="grid gap-1">
          {plan.paths.map((path, index) => (
            <li
              key={`${path.source_node}-${path.sink_node}-${index}`}
              className="flex flex-wrap items-center gap-1.5 font-mono text-[11px]"
            >
              <span className="rounded bg-amber-500/15 px-1.5 py-0.5 text-amber-700">
                {path.source_category}
              </span>
              <span aria-hidden="true" className="text-muted-foreground">
                →
              </span>
              <span className="rounded bg-destructive/15 px-1.5 py-0.5 text-destructive">
                {path.sink_category}
              </span>
            </li>
          ))}
        </ul>
      ) : null}

      {plan.unmapped_node_types.length > 0 ? (
        <p className="text-[11px] text-muted-foreground">
          Unmapped (not analysed): {plan.unmapped_node_types.join(', ')}
        </p>
      ) : null}
    </div>
  );
}

/** The full tailored vectors, laid out for the wide right pane (2-up) so they're
 *  readable in one view instead of pushing the left-column controls off-screen. */
export function PlanVectors({ plan }: { plan: RedteamPlan }) {
  if (plan.vectors.length === 0) return null;
  return (
    <Card>
      <CardHeader>
        <CardTitle className="flex items-center gap-2 text-base">
          <Crosshair className="size-4 text-primary" aria-hidden="true" />
          Planned attack vectors
        </CardTitle>
        <CardDescription>
          Tailored from the agent&apos;s definition. Hit{' '}
          <span className="font-medium text-foreground">Attack</span> on the left — these{' '}
          {plan.vectors.length} vectors seed the run.
        </CardDescription>
      </CardHeader>
      <CardContent>
        <ul className="grid gap-3 sm:grid-cols-2">
          {plan.vectors.map((vector, index) => (
            <li
              key={index}
              className="grid content-start gap-1 rounded-md border bg-muted/30 px-3 py-2"
            >
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
      </CardContent>
    </Card>
  );
}
