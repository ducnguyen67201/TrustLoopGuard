import { Badge } from '@/components/ui/badge';
import { GLOSSARY } from '@/lib/glossary';
import { cn } from '@/lib/utils';

const EFFECTS = ['permit', 'transform', 'require_approval', 'defer', 'deny'] as const;

type AuthorizationEffect = (typeof EFFECTS)[number];

interface AuthorizationEffectLegendProps {
  effects?: ReadonlyArray<AuthorizationEffect>;
  className?: string;
}

const BADGE_VARIANT = {
  permit: 'permit',
  transform: 'transform',
  require_approval: 'require_approval',
  defer: 'defer',
  deny: 'deny',
} as const;

export function AuthorizationEffectLegend({ effects, className }: AuthorizationEffectLegendProps) {
  const shown = effects ? EFFECTS.filter((effect) => effects.includes(effect)) : EFFECTS;
  const columns =
    shown.length >= 5
      ? 'sm:grid-cols-2 xl:grid-cols-5'
      : shown.length >= 3
        ? 'sm:grid-cols-3'
        : shown.length === 2
          ? 'sm:grid-cols-2'
          : 'sm:grid-cols-1';

  return (
    <dl
      className={cn('grid gap-x-6 gap-y-2', columns, className)}
      aria-label="What each authorization effect means"
    >
      {shown.map((effect) => (
        <div key={effect} className="flex items-start gap-2">
          <dt className="shrink-0">
            <Badge variant={BADGE_VARIANT[effect]}>{GLOSSARY[effect].label}</Badge>
          </dt>
          <dd className="text-xs leading-relaxed text-muted-foreground [text-wrap:pretty]">
            {GLOSSARY[effect].short}
          </dd>
        </div>
      ))}
    </dl>
  );
}
