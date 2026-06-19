import { Badge } from '@/components/ui/badge';

type BadgeVariant = 'secondary' | 'outline' | 'destructive';

const SEVERITY_VARIANT: Record<string, BadgeVariant> = {
  low: 'secondary',
  medium: 'outline',
  high: 'destructive',
  critical: 'destructive',
};

/**
 * Renders a policy severity as a tone-mapped badge. Severity is operator metadata,
 * not a guardrail verdict, so it never borrows the reserved verdict colors.
 */
export function PolicySeverityBadge({ severity }: { severity: string }) {
  const variant = SEVERITY_VARIANT[severity.toLowerCase()] ?? 'outline';
  return (
    <Badge variant={variant} className="font-mono text-[0.6875rem] uppercase tracking-wide">
      {severity}
    </Badge>
  );
}
