'use client';

import { Crosshair, Loader2, Radar, ShieldPlus, Trash2, Workflow } from 'lucide-react';

import { Badge } from '@/components/ui/badge';
import { Button } from '@/components/ui/button';
import { InfoHint } from '@/components/ui/info-hint';
import { Input } from '@/components/ui/input';
import { Label } from '@/components/ui/label';
import { cn } from '@/lib/utils';
import type { AttackVector, RedteamPlan } from '@/lib/redteam-plan';

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
        This step is optional. Pick a saved agent above and we can build attacks aimed at its
        specific weak spots. Without one, the test just uses our standard set of tricky prompts.
      </p>
    );
  }

  return (
    <div className="grid w-full max-w-full min-w-0 gap-3">
      {savedPlans.length > 0 ? (
        <div className="grid min-w-0 gap-1.5">
          <span className="text-[11px] font-medium tracking-wide text-muted-foreground uppercase">
            Saved attack sets · pick one to reuse
          </span>
          <ul className="grid gap-1">
            {savedPlans.map((saved) => {
              const active = saved.id === activePlanId;
              return (
                <li
                  key={saved.id}
                  className={cn(
                    'group flex min-w-0 items-center gap-2 rounded-md border px-2.5 py-1.5 transition-colors',
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
                    className="grid min-w-0 flex-1 text-left disabled:opacity-60"
                  >
                    <span className="flex min-w-0 items-center gap-1.5">
                      <span
                        className={cn(
                          'size-1.5 rounded-full',
                          active ? 'bg-primary' : 'bg-muted-foreground/40',
                        )}
                        aria-hidden="true"
                      />
                      <span className="truncate text-sm font-medium">{saved.name}</span>
                    </span>
                    <span className="min-w-0 truncate pl-3 text-[11px] tabular-nums text-muted-foreground">
                      {saved.vectors.length} {saved.vectors.length === 1 ? 'attack' : 'attacks'} ·{' '}
                      {saved.paths.length} {saved.paths.length === 1 ? 'weak spot' : 'weak spots'}
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
        <Label
          htmlFor="plan-name"
          className="text-[11px] tracking-wide text-muted-foreground uppercase"
        >
          {savedPlans.length > 0 ? 'Or build a new attack set' : 'Build a tailored attack set'}
        </Label>
        <div className="flex min-w-0 flex-wrap items-center gap-2">
          <Input
            id="plan-name"
            value={planName}
            onChange={(e) => onPlanNameChange(e.target.value)}
            placeholder="Give it a name, e.g. Nightly check"
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
            {planning ? 'Building…' : 'Build tailored attacks'}
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
        <p
          role="alert"
          className="rounded-md border border-destructive/40 bg-destructive/10 px-3 py-2 text-sm text-destructive"
        >
          {planError}
        </p>
      ) : null}

      {staticCount !== null ? (
        <p
          className="rounded-md border px-3 py-2 text-xs"
          style={{
            color: 'var(--color-allow)',
            borderColor: 'color-mix(in oklab, var(--color-allow), transparent 65%)',
            backgroundColor: 'color-mix(in oklab, var(--color-allow), transparent 90%)',
          }}
        >
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
    <div className="grid min-w-0 gap-2 rounded-md border border-l-2 border-primary/30 border-l-primary/60 bg-primary/[0.04] px-3 py-2.5">
      <div className="flex flex-wrap items-center gap-1.5 text-xs text-muted-foreground">
        <Badge variant="outline" className="gap-1 border-primary/40 text-foreground">
          <Crosshair className="size-3 text-primary" aria-hidden="true" />
          <span className="tabular-nums">{plan.vectors.length}</span>{' '}
          {plan.vectors.length === 1 ? 'vector' : 'vectors'} ready
        </Badge>
        {plan.paths.length > 0 ? (
          <Badge variant="outline" className="gap-1">
            <Workflow className="size-3" aria-hidden="true" />
            <span className="tabular-nums">{plan.paths.length}</span>{' '}
            {plan.paths.length === 1 ? 'weak spot' : 'weak spots'}
          </Badge>
        ) : null}
      </div>

      {plan.paths.length > 0 ? (
        <div className="grid gap-1.5">
          <span className="flex items-center gap-1.5 text-[11px] font-medium text-muted-foreground">
            Where an attack could slip through
            <InfoHint label="What a weak spot means">
              Each line shows a spot where something untrusted could reach a
              sensitive action — read it as &ldquo;from here → to there.&rdquo; These
              are the places we&apos;ll aim the attacks.
            </InfoHint>
          </span>
          <ul className="grid gap-1">
            {plan.paths.map((path, index) => (
              <li
                key={`${path.source_node}-${path.sink_node}-${index}`}
                className="flex flex-wrap items-center gap-1.5 text-[11px]"
              >
                <span className="rounded bg-amber-500/15 px-1.5 py-0.5 text-amber-700 dark:text-amber-400">
                  {humanizeToken(path.source_category)}
                </span>
                <span aria-hidden="true" className="text-muted-foreground">
                  →
                </span>
                <span
                  className="rounded px-1.5 py-0.5"
                  style={{
                    color: 'var(--color-block)',
                    backgroundColor: 'color-mix(in oklab, var(--color-block), transparent 85%)',
                  }}
                >
                  {humanizeToken(path.sink_category)}
                </span>
              </li>
            ))}
          </ul>
        </div>
      ) : null}

      {plan.unmapped_node_types.length > 0 ? (
        <p className="flex items-center gap-1.5 text-[11px] text-muted-foreground">
          A few parts couldn&apos;t be checked automatically
          <InfoHint label="Parts we couldn’t check">
            We didn&apos;t recognize these pieces of the agent, so they weren&apos;t
            included in this analysis: {plan.unmapped_node_types.join(', ')}.
          </InfoHint>
        </p>
      ) : null}
    </div>
  );
}

// ───────────────────────────── Threat Board ─────────────────────────────────
//
// The right pane is a tactical board: every planned vector becomes a severity-
// graded threat card. Severity is a UI-only read of `vector.technique` — no
// contract or data change, purely how we present the same wire payload.

type ThreatSeverity = 'critical' | 'high' | 'medium' | 'low';

const SEVERITY_ORDER: Record<ThreatSeverity, number> = {
  critical: 0,
  high: 1,
  medium: 2,
  low: 3,
};

/** UI-only severity, derived from the technique. Mapping per the operator brief;
 *  it never travels back to the API. */
function severityOfTechnique(technique: string): ThreatSeverity {
  switch (technique) {
    case 'credential_disclosure':
    case 'data_exfiltration':
      return 'critical';
    case 'tool_misuse':
    case 'instruction_override':
      return 'high';
    case 'scope_violation':
      return 'medium';
    default:
      return 'low';
  }
}

/** Severity → restrained color ramp + pip count. Color is never the only signal:
 *  every consumer also renders the label and the pip cluster. */
interface SeverityStyle {
  label: string;
  pips: number;
  accent: string; // left bar / pip fill
  text: string; // tier label color
  chip: string; // tier label chip bg
  ring: string; // card hover ring
}

const SEVERITY_STYLE: Record<ThreatSeverity, SeverityStyle> = {
  critical: {
    label: 'CRITICAL',
    pips: 3,
    accent: 'bg-destructive',
    // Darker red for the small label/pip text so it clears WCAG AA on the
    // off-white card; the loud `bg-destructive` bar stays vivid.
    text: 'text-red-700',
    chip: 'bg-destructive/10',
    ring: 'hover:ring-destructive/30',
  },
  high: {
    label: 'HIGH',
    pips: 3,
    accent: 'bg-orange-600',
    text: 'text-orange-700',
    chip: 'bg-orange-600/10',
    ring: 'hover:ring-orange-600/30',
  },
  medium: {
    label: 'MEDIUM',
    pips: 2,
    // Amber (not yellow) so the ramp reads as a deliberate red → orange → amber
    // descent and the label clears AA contrast.
    accent: 'bg-amber-500',
    text: 'text-amber-700',
    chip: 'bg-amber-500/10',
    ring: 'hover:ring-amber-600/30',
  },
  low: {
    label: 'LOW',
    pips: 1,
    accent: 'bg-muted-foreground/50',
    text: 'text-muted-foreground',
    chip: 'bg-muted',
    ring: 'hover:ring-foreground/15',
  },
};

const SEVERITY_TIERS: readonly ThreatSeverity[] = ['critical', 'high', 'medium', 'low'];

/** A three-slot pip cluster (●●●/●●/●). Filled pips carry severity color; empty
 *  slots are dim — so the *count* reads even in grayscale, not just the hue. */
function SeverityPips({ severity }: { severity: ThreatSeverity }) {
  const { pips, accent, label } = SEVERITY_STYLE[severity];
  return (
    <span className="inline-flex items-center gap-[3px]" aria-label={`${label} severity`}>
      {[0, 1, 2].map((i) => (
        <span
          key={i}
          aria-hidden="true"
          className={cn('size-1.5 rounded-full', i < pips ? accent : 'bg-border')}
        />
      ))}
    </span>
  );
}

/** Maps an attack `target_operation` to a compact route token. The planner emits
 *  raw operation ids; we show the channel they hit. */
function targetRoute(operation: string): string {
  const op = operation.toLowerCase();
  if (op.includes('http') || op.startsWith('post') || op.startsWith('get')) return '→ http';
  if (op.includes('chat') || op.includes('reply') || op.includes('message')) return '→ chat_reply';
  if (op.includes('tool') || op.includes('call')) return '→ tool_call';
  if (op.includes('doc') || op.includes('pdf') || op.includes('file')) return '→ document';
  return `→ ${operation}`;
}

/** Turns an internal taxonomy token like `untrusted_input` into spaced words.
 *  Purely presentational — the underlying value is unchanged. */
function humanizeToken(token: string): string {
  return token.replaceAll('_', ' ').trim() || 'unknown';
}

/** Plain-language name for a planner technique id, for a non-technical owner.
 *  Falls back to the de-underscored id so an unexpected technique still reads. */
const TECHNIQUE_LABELS: Record<string, string> = {
  credential_disclosure: 'Leaks a secret',
  data_exfiltration: 'Steals data',
  tool_misuse: 'Misuses a tool',
  instruction_override: 'Ignores its rules',
  scope_violation: 'Goes out of bounds',
};

function techniqueLabel(technique: string): string {
  return TECHNIQUE_LABELS[technique] ?? humanizeToken(technique);
}

/** One-sentence explanation shown in the tag's info hint. */
const TECHNIQUE_HINTS: Record<string, string> = {
  credential_disclosure: 'Tries to make the agent reveal a password, key, or other secret.',
  data_exfiltration: 'Tries to make the agent hand over private or sensitive data.',
  tool_misuse: 'Tries to make the agent use one of its tools in a harmful way.',
  instruction_override: 'Tries to make the agent ignore its own safety rules.',
  scope_violation: 'Tries to make the agent do something outside what it should.',
};

function techniqueHint(technique: string): string {
  return (
    TECHNIQUE_HINTS[technique] ??
    `An attack of type “${humanizeToken(technique)}.”`
  );
}

interface ThreatCardData {
  severity: ThreatSeverity;
  vector: AttackVector;
}

/** The full tailored vectors, laid out for the wide right pane as a severity-
 *  graded Threat Board: critical-first, accent bar + pip cluster, technique tag,
 *  target route, and goal. Cards stagger-in on plan (reduced-motion: instant). */
export function PlanVectors({ plan }: { plan: RedteamPlan }) {
  if (plan.vectors.length === 0) return null;

  const threats: ThreatCardData[] = plan.vectors
    .map((vector) => ({ severity: severityOfTechnique(vector.technique), vector }))
    .sort((a, b) => SEVERITY_ORDER[a.severity] - SEVERITY_ORDER[b.severity]);

  const counts = threats.reduce<Record<ThreatSeverity, number>>(
    (acc, t) => {
      acc[t.severity] += 1;
      return acc;
    },
    { critical: 0, high: 0, medium: 0, low: 0 },
  );

  const summary = SEVERITY_TIERS.filter((tier) => counts[tier] > 0)
    .map((tier) => `${counts[tier]} ${tier}`)
    .join(' · ');

  return (
    <section
      aria-label="Attacks we will try"
      className="min-w-0 overflow-hidden rounded-xl border bg-card shadow-sm ring-1 ring-border/60"
    >
      {/* Board header: instrument strip with live counts. */}
      <header className="flex min-w-0 flex-wrap items-center gap-x-3 gap-y-1 border-b bg-muted/40 px-4 py-2.5">
        <Radar className="size-4 text-primary" aria-hidden="true" />
        <h2 className="font-mono text-[11px] font-semibold tracking-[0.15em] uppercase">
          Attacks we&apos;ll try
        </h2>
        <span className="ml-auto flex min-w-0 flex-wrap items-center gap-x-2 gap-y-0.5 font-mono text-[11px] text-muted-foreground">
          <span className="tabular-nums text-foreground">
            {plan.vectors.length} {plan.vectors.length === 1 ? 'attack' : 'attacks'}
          </span>
          {summary ? (
            <>
              <span aria-hidden="true">·</span>
              <span className="tabular-nums">{summary}</span>
            </>
          ) : null}
        </span>
      </header>

      <p className="border-b px-4 py-2 text-xs leading-relaxed text-muted-foreground">
        These are the attacks we&apos;ll try, built for this agent and sorted most dangerous first.
        Press{' '}
        <span className="font-medium text-foreground">Run test</span> on the left to send all{' '}
        <span className="tabular-nums">{plan.vectors.length}</span> of them.
      </p>

      <ul className="grid gap-px bg-border/50 sm:grid-cols-2">
        {threats.map(({ severity, vector }, index) => (
          <li
            key={`${vector.technique}-${vector.target_operation}-${index}`}
            // The single deadliest threat anchors the board — spans full width,
            // heavier type — so the most dangerous vector reads first.
            className={cn('threat-rise', index === 0 && 'sm:col-span-2')}
            style={{ animationDelay: `${index * 40}ms` }}
          >
            <ThreatCard severity={severity} vector={vector} featured={index === 0} />
          </li>
        ))}
      </ul>
    </section>
  );
}

function ThreatCard({ severity, vector, featured = false }: ThreatCardData & { featured?: boolean }) {
  const style = SEVERITY_STYLE[severity];
  return (
    <article
      className={cn(
        'group flex h-full min-w-0 gap-3 bg-card transition-[box-shadow,background-color] duration-200 ease-out',
        'ring-0 ring-inset hover:bg-muted/30 hover:ring-2',
        featured ? 'px-4 py-3' : 'px-3 py-2.5',
        style.ring,
      )}
    >
      {/* Left severity accent bar — the single loudest signal on the card. */}
      <span
        aria-hidden="true"
        className={cn('shrink-0 self-stretch', featured ? 'w-1.5' : 'w-1', style.accent)}
      />

      <div className="grid min-w-0 gap-1.5">
        {/* Tier row: chip + pips, then the route on the right. */}
        <div className="flex min-w-0 flex-wrap items-center gap-x-2 gap-y-1">
          <span
            className={cn(
              'inline-flex items-center gap-1.5 rounded px-1.5 py-0.5 font-mono text-[10px] font-semibold tracking-[0.12em]',
              style.chip,
              style.text,
            )}
          >
            {style.label}
            <SeverityPips severity={severity} />
          </span>
          <span className="ml-auto truncate font-mono text-[11px] tabular-nums text-muted-foreground">
            {targetRoute(vector.target_operation)}
          </span>
        </div>

        {/* Technique tag — the kind of trick, in plain words, with a hint that
            spells out the internal name for anyone who wants it. */}
        <span className="inline-flex w-fit max-w-full items-center gap-1 rounded-r border-l-2 border-border bg-muted/60 px-1.5 py-0.5 text-[11px] tracking-wide text-foreground/80">
          <span className="truncate">{techniqueLabel(vector.technique)}</span>
          <InfoHint label={`What “${techniqueLabel(vector.technique)}” means`}>
            {techniqueHint(vector.technique)}
          </InfoHint>
        </span>

        {/* The objective. */}
        <p
          className={cn(
            'leading-snug text-foreground/90',
            featured ? 'text-base font-medium' : 'text-sm',
          )}
        >
          {vector.goal}
        </p>
      </div>
    </article>
  );
}
