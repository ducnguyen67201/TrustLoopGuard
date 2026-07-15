import type { ReactNode } from 'react';

import { BrandLogo } from '@/components/brand-logo';
import { Badge } from '@/components/ui/badge';
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from '@/components/ui/card';
import { InfoHint } from '@/components/ui/info-hint';

interface AuthScreenProps {
  eyebrow: string;
  title: string;
  description: string;
  /** Card title + description shown above the form. */
  cardTitle: string;
  cardDescription: string;
  children: ReactNode;
}

// Canonical order: allow -> rewrite -> escalate -> block, read as escalating
// intervention (matches AuthorizationEffectLegend and the rest of the app). Each chip carries
// a one-word plain gloss so it isn't an unexplained colored word.
const EFFECTS = [
  { variant: 'permit', label: 'permit', gloss: 'let through' },
  { variant: 'transform', label: 'transform', gloss: 'cleaned up' },
  { variant: 'require_approval', label: 'require_approval', gloss: 'sent to review' },
  { variant: 'deny', label: 'deny', gloss: 'stopped' },
] as const;

/**
 * The shared branded surface for sign in / sign up. Two-pane on large screens
 * (brand rail + auth card), a single centered column on small screens. Outside
 * the app shell, so it carries its own wordmark and wayfinding.
 */
export function AuthScreen({
  eyebrow,
  title,
  description,
  cardTitle,
  cardDescription,
  children,
}: AuthScreenProps) {
  return (
    <main className="grid min-h-screen bg-background text-foreground lg:grid-cols-[1.05fr_1fr]">
      <BrandRail eyebrow={eyebrow} title={title} description={description} />

      <section className="flex items-center justify-center px-4 py-10 sm:px-8 lg:px-12">
        <div className="grid w-full max-w-md gap-6">
          <div className="grid gap-2 lg:hidden">
            <div className="flex items-center gap-2 text-sm text-muted-foreground">
              <BrandLogo className="size-7" priority />
              <span className="font-mono text-xs tracking-wide">TrustLoopGuard</span>
            </div>
            <h1 className="text-2xl font-semibold tracking-tight">{title}</h1>
            <p className="text-sm text-muted-foreground">{description}</p>
          </div>

          <Card className="shadow-sm">
            <CardHeader>
              <CardTitle className="text-lg tracking-tight">{cardTitle}</CardTitle>
              <CardDescription>{cardDescription}</CardDescription>
            </CardHeader>
            <CardContent className="space-y-5">{children}</CardContent>
          </Card>
        </div>
      </section>
    </main>
  );
}

interface BrandRailProps {
  eyebrow: string;
  title: string;
  description: string;
}

/**
 * The left identity rail. Hidden below lg; the auth card reuses the same copy in
 * its own header for small screens. Signal-over-decoration: the effect chips are
 * the only saturated color, and they carry meaning.
 */
function BrandRail({ eyebrow, title, description }: BrandRailProps) {
  return (
    <aside
      aria-label="Product overview"
      className="relative hidden flex-col justify-between overflow-hidden border-r border-border bg-secondary px-12 py-12 lg:flex"
    >
      <div className="flex items-center gap-2.5">
        <BrandLogo className="size-8" priority />
        <span className="font-mono text-sm tracking-wide">TrustLoopGuard</span>
      </div>

      <div className="grid max-w-md gap-5">
        <p className="font-mono text-xs uppercase tracking-[0.2em] text-primary">{eyebrow}</p>
        <h1 className="text-balance text-4xl font-semibold leading-[1.1] tracking-tight">
          {title}
        </h1>
        <p className="text-pretty text-base leading-relaxed text-muted-foreground">{description}</p>
      </div>

      <div className="grid gap-3">
        <p className="flex items-center gap-1.5 font-mono text-[0.7rem] uppercase tracking-[0.18em] text-muted-foreground">
          Every request gets one of these
          <InfoHint term="effect" side="top" />
        </p>
        <dl className="flex flex-wrap gap-x-5 gap-y-2.5">
          {EFFECTS.map((effect) => (
            <div key={effect.variant} className="flex items-center gap-2">
              <dt>
                <Badge variant={effect.variant}>{effect.label}</Badge>
              </dt>
              <dd className="text-xs text-muted-foreground">{effect.gloss}</dd>
            </div>
          ))}
        </dl>
      </div>
    </aside>
  );
}
