import { Suspense } from 'react';
import type { CSSProperties, ReactNode } from 'react';
import { AppSidebar } from '@/components/app-sidebar';
import { SiteHeader, type Breadcrumb } from '@/components/site-header';
import { SidebarInset, SidebarProvider } from '@/components/ui/sidebar';
import { getDashboardShell, type DashboardShellData } from '@/lib/server/dashboard-data';

interface AppLayoutProps {
  title: string;
  /** Wayfinding trail for nested routes (e.g. Runs / <run id>). */
  breadcrumbs?: Breadcrumb[];
  workspaceSlug?: string | null;
  environmentId?: string | null;
  shell?: DashboardShellData;
  children: ReactNode;
}

export async function AppLayout({
  title,
  breadcrumbs,
  workspaceSlug,
  environmentId,
  shell,
  children,
}: AppLayoutProps) {
  const resolvedShell = shell ?? (await getDashboardShell(workspaceSlug, environmentId));

  return (
    <SidebarProvider
      style={
        {
          '--sidebar-width': 'calc(var(--spacing) * 72)',
          '--header-height': 'calc(var(--spacing) * 12)',
        } as CSSProperties
      }
    >
      <Suspense>
        <AppSidebar
          variant="inset"
          user={resolvedShell.user}
          organization={resolvedShell.organization}
          activeWorkspace={resolvedShell.activeWorkspace}
          workspaces={resolvedShell.workspaces}
          activeEnvironment={resolvedShell.activeEnvironment}
          environments={resolvedShell.environments}
          agents={resolvedShell.agents}
        />
      </Suspense>
      <SidebarInset>
        <SiteHeader
          title={title}
          breadcrumbs={breadcrumbs}
          activeWorkspaceName={resolvedShell.activeWorkspace.name}
          activeEnvironmentName={resolvedShell.activeEnvironment.name}
        />
        <div className="flex flex-1 flex-col">
          <div className="@container/main flex flex-1 flex-col gap-2">
            <div className="flex flex-col gap-4 py-4 md:gap-6 md:py-6">{children}</div>
          </div>
        </div>
      </SidebarInset>
    </SidebarProvider>
  );
}
