import { Badge } from '@/components/ui/badge';
import { GLOSSARY } from '@/lib/glossary';
import { cn } from '@/lib/utils';

// The four verdicts in escalating order of intervention, so the key reads as a
// gradient from "nothing happened" to "stopped".
const VERDICTS = ['allow', 'rewrite', 'escalate', 'block'] as const;

type Verdict = (typeof VERDICTS)[number];

interface VerdictLegendProps {
  /**
   * Limit the key to a subset of verdicts, in canonical order. Use this in
   * policy-authoring contexts where only block/rewrite/escalate are selectable,
   * so "Allow" (the no-rule-matched default) doesn't read as a pickable option.
   */
  verdicts?: ReadonlyArray<Verdict>;
  className?: string;
}

/**
 * Plain-language key for the allow / rewrite / escalate / block verdicts. Drop it
 * near any table or chart that uses verdict badges so a first-time viewer knows
 * what each color means without leaving the page. Definitions come from
 * `lib/glossary.ts`. See docs/concept/web-ui-conventions.md.
 */
export function VerdictLegend({ verdicts, className }: VerdictLegendProps) {
  const shown = verdicts ? VERDICTS.filter((v) => verdicts.includes(v)) : VERDICTS;
  const columns =
    shown.length >= 4
      ? 'sm:grid-cols-2 xl:grid-cols-4'
      : shown.length === 3
        ? 'sm:grid-cols-3'
        : shown.length === 2
          ? 'sm:grid-cols-2'
          : 'sm:grid-cols-1';

  return (
    <dl
      className={cn('grid gap-x-6 gap-y-2', columns, className)}
      aria-label="What each verdict means"
    >
      {shown.map((verdict) => (
        <div key={verdict} className="flex items-start gap-2">
          <dt className="shrink-0">
            <Badge variant={verdict}>{GLOSSARY[verdict].label}</Badge>
          </dt>
          <dd className="text-xs leading-relaxed text-muted-foreground [text-wrap:pretty]">
            {GLOSSARY[verdict].short}
          </dd>
        </div>
      ))}
    </dl>
  );
}
