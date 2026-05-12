import Link from 'next/link';
import type { ReactNode } from 'react';
import { Button } from '@/components/ui/button';
import { Separator } from '@/components/ui/separator';
import { cn } from '@/lib/utils';

interface AppShellProps {
  title: string;
  eyebrow?: string;
  description?: string;
  active?: 'playground' | 'policies';
  children: ReactNode;
  footer?: ReactNode;
  className?: string;
}

const NAV_ITEMS = [
  { href: '/', label: 'Playground', value: 'playground' },
  { href: '/policies', label: 'Policies', value: 'policies' },
] as const;

export function AppShell({
  title,
  eyebrow = 'TrustLoopGuard',
  description,
  active,
  children,
  footer,
  className,
}: AppShellProps) {
  return (
    <main className={cn('mx-auto w-full max-w-7xl px-4 py-6 sm:px-6 lg:px-8', className)}>
      <header className="mb-6 flex flex-col gap-4 sm:flex-row sm:items-end sm:justify-between">
        <div className="min-w-0 space-y-2">
          <p className="font-mono text-xs uppercase tracking-[0.2em] text-muted-foreground">
            {eyebrow}
          </p>
          <div className="space-y-2">
            <h1 className="text-3xl font-semibold tracking-tight sm:text-4xl">{title}</h1>
            {description !== undefined ? (
              <p className="max-w-2xl text-sm text-muted-foreground">{description}</p>
            ) : null}
          </div>
        </div>
        {active !== undefined ? (
          <nav className="flex flex-wrap items-center gap-2">
            {NAV_ITEMS.map((item) => (
              <Button
                key={item.href}
                asChild
                size="sm"
                variant={active === item.value ? 'default' : 'outline'}
              >
                <Link href={item.href}>{item.label}</Link>
              </Button>
            ))}
          </nav>
        ) : null}
      </header>

      <Separator className="mb-6" />
      {children}
      {footer !== undefined ? (
        <footer className="mt-8">
          <Separator className="mb-4" />
          <div className="flex flex-col gap-2 font-mono text-xs text-muted-foreground sm:flex-row sm:items-center sm:justify-between">
            {footer}
          </div>
        </footer>
      ) : null}
    </main>
  );
}
