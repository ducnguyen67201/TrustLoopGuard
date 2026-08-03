import { BrandLogo } from '@/components/brand-logo';
import { Badge } from '@/components/ui/badge';

/**
 * First-run brand bar for the pre-shell onboarding pages. These pages render
 * outside AppLayout, so they own their own wayfinding: a brand mark plus a
 * plain "Setup" marker.
 */
export function SetupBrandHeader() {
  return (
    <header className="flex items-center justify-between gap-4">
      <div className="flex items-center gap-2.5">
        <BrandLogo alt="" className="h-7 w-auto" priority />
        <span className="font-mono text-sm font-normal tracking-wide text-foreground">
          Featherlane AI
        </span>
      </div>
      <Badge variant="outline" className="text-xs tracking-tight text-muted-foreground">
        Getting started
      </Badge>
    </header>
  );
}
