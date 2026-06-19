import type { ReactNode } from 'react';

import { BrandLogo } from '@/components/brand-logo';
import { Badge } from '@/components/ui/badge';
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from '@/components/ui/card';

interface AuthScreenProps {
  eyebrow: string;
  title: string;
  description: string;
  /** Card title + description shown above the form. */
  cardTitle: string;
  cardDescription: string;
  children: ReactNode;
}

const VERDICTS = [
  { variant: 'allow', label: 'allow' },
  { variant: 'rewrite', label: 'rewrite' },
  { variant: 'block', label: 'block' },
  { variant: 'escalate', label: 'escalate' },
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
 * its own header for small screens. Signal-over-decoration: the verdict chips are
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
        <h1 className="text-balance text-4xl font-semibold leading-[1.1] tracking-tight">{title}</h1>
        <p className="text-pretty text-base leading-relaxed text-muted-foreground">{description}</p>
      </div>

      <div className="grid gap-3">
        <p className="font-mono text-[0.7rem] uppercase tracking-[0.18em] text-muted-foreground">
          Every call gets a verdict
        </p>
        <div className="flex flex-wrap gap-2">
          {VERDICTS.map((verdict) => (
            <Badge key={verdict.variant} variant={verdict.variant}>
              {verdict.label}
            </Badge>
          ))}
        </div>
      </div>
    </aside>
  );
}
