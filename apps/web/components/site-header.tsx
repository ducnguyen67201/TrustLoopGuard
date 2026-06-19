import Link from 'next/link';
import { Fragment } from 'react';

import { Badge } from '@/components/ui/badge';
import { Separator } from '@/components/ui/separator';
import { SidebarTrigger } from '@/components/ui/sidebar';
import { cn } from '@/lib/utils';

export interface Breadcrumb {
  label: string;
  href?: string;
}

interface SiteHeaderProps {
  title: string;
  /** Wayfinding trail for nested routes. Falls back to just the page title. */
  breadcrumbs?: Breadcrumb[] | undefined;
  activeWorkspaceName: string;
  activeEnvironmentName: string;
}

export function SiteHeader({
  title,
  breadcrumbs,
  activeWorkspaceName,
  activeEnvironmentName,
}: SiteHeaderProps) {
  const trail = breadcrumbs && breadcrumbs.length > 0 ? breadcrumbs : [{ label: title }];

  return (
    <header className="flex h-(--header-height) shrink-0 items-center gap-2 border-b transition-[width,height] ease-linear group-has-data-[collapsible=icon]/sidebar-wrapper:h-(--header-height)">
      <div className="flex w-full items-center gap-1 px-4 lg:gap-2 lg:px-6">
        <SidebarTrigger className="-ml-1" />
        <Separator orientation="vertical" className="mx-2 data-[orientation=vertical]:h-4" />
        <nav
          aria-label="Breadcrumb"
          className="flex min-w-0 items-center gap-1.5 text-sm"
        >
          {trail.map((crumb, index) => {
            const isLast = index === trail.length - 1;
            return (
              <Fragment key={`${crumb.label}-${index}`}>
                {index > 0 ? (
                  <span className="text-muted-foreground/50" aria-hidden>
                    /
                  </span>
                ) : null}
                {crumb.href && !isLast ? (
                  <Link
                    href={crumb.href}
                    className="truncate text-muted-foreground transition-colors hover:text-foreground"
                  >
                    {crumb.label}
                  </Link>
                ) : (
                  <span
                    className={cn(
                      'truncate',
                      isLast ? 'font-medium text-foreground' : 'text-muted-foreground',
                    )}
                    aria-current={isLast ? 'page' : undefined}
                  >
                    {crumb.label}
                  </span>
                )}
              </Fragment>
            );
          })}
        </nav>
        <Badge
          variant="outline"
          className="ml-auto hidden rounded-md font-mono text-xs font-normal md:inline-flex"
          title={`You are viewing the “${activeWorkspaceName}” workspace (your project space) in its “${activeEnvironmentName}” environment (dev, staging, or production). Switch either one from the sidebar.`}
          aria-label={`Current context: ${activeWorkspaceName} workspace, ${activeEnvironmentName} environment`}
        >
          {activeWorkspaceName} / {activeEnvironmentName}
        </Badge>
      </div>
    </header>
  );
}
