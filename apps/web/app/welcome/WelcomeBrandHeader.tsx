import { BrandLogo } from '@/components/brand-logo';
import { Badge } from '@/components/ui/badge';

interface WelcomeBrandHeaderProps {
  /** Short status word shown in the brand bar, e.g. "Pending". */
  status: string;
}

/**
 * Brand bar for the pre-shell welcome / waiting-room page. The page renders
 * outside AppLayout, so it carries its own brand mark plus an account-status
 * marker so the user knows why they're here.
 */
export function WelcomeBrandHeader({ status }: WelcomeBrandHeaderProps) {
  return (
    <header className="flex items-center justify-between gap-4">
      <div className="flex items-center gap-2.5">
        <BrandLogo className="size-7" priority />
        <span className="text-sm font-medium text-foreground">Featherlane AI</span>
      </div>
      <Badge variant="outline" className="gap-1.5 text-xs">
        <span aria-hidden className="relative flex size-1.5">
          <span className="absolute inline-flex size-full animate-ping rounded-full bg-primary/60 motion-reduce:hidden" />
          <span className="relative inline-flex size-1.5 rounded-full bg-primary" />
        </span>
        {status}
      </Badge>
    </header>
  );
}
