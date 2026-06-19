import { Badge } from '@/components/ui/badge';

type BadgeVariant = 'secondary' | 'outline' | 'destructive';

const SEVERITY_VARIANT: Record<string, BadgeVariant> = {
  low: 'secondary',
  medium: 'outline',
  high: 'destructive',
  critical: 'destructive',
};

// Plain-language gloss for each severity so a non-technical reader gets the
// meaning from a hover, not just a colored chip.
const SEVERITY_LABEL: Record<string, string> = {
  low: 'Low',
  medium: 'Medium',
  high: 'High',
  critical: 'Critical',
};

const SEVERITY_HELP: Record<string, string> = {
  low: 'Minor — worth knowing about, low risk.',
  medium: 'Moderate — should be looked at.',
  high: 'Serious — needs attention soon.',
  critical: 'Severe — the most urgent kind of violation.',
};

/**
 * Renders a policy severity as a tone-mapped badge. Severity is operator metadata,
 * not a guardrail verdict, so it never borrows the reserved verdict colors. The
 * title attribute gives a plain-language meaning on hover.
 */
export function PolicySeverityBadge({ severity }: { severity: string }) {
  const key = severity.toLowerCase();
  const variant = SEVERITY_VARIANT[key] ?? 'outline';
  const label = SEVERITY_LABEL[key] ?? severity;
  const help = SEVERITY_HELP[key];
  return (
    <Badge variant={variant} className="text-[0.6875rem]" title={help}>
      {label}
    </Badge>
  );
}
