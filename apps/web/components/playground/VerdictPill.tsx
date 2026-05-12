import { Badge } from '@/components/ui/badge';
import { cn } from '@/lib/utils';
import type { Verdict } from '../../lib/schemas';

interface VerdictPillProps {
  verdict: Verdict;
}

const VERDICT_COLOR: Record<Verdict, string> = {
  allow:
    'bg-[color:var(--color-allow)]/15 text-[color:var(--color-allow)] ring-[color:var(--color-allow)]/40',
  rewrite:
    'bg-[color:var(--color-rewrite)]/15 text-[color:var(--color-rewrite)] ring-[color:var(--color-rewrite)]/40',
  block:
    'bg-[color:var(--color-block)]/15 text-[color:var(--color-block)] ring-[color:var(--color-block)]/40',
  escalate:
    'bg-[color:var(--color-escalate)]/15 text-[color:var(--color-escalate)] ring-[color:var(--color-escalate)]/40',
};

export function VerdictPill({ verdict }: VerdictPillProps) {
  return (
    <Badge
      variant="outline"
      className={cn('gap-2 px-3 py-1 font-mono uppercase tracking-wider', VERDICT_COLOR[verdict])}
    >
      <span className="size-1.5 rounded-full bg-current" />
      {verdict}
    </Badge>
  );
}
