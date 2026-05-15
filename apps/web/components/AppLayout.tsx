import type { CSSProperties, ReactNode } from 'react';
import { AppSidebar } from '@/components/app-sidebar';
import { SiteHeader } from '@/components/site-header';
import { SidebarInset, SidebarProvider } from '@/components/ui/sidebar';
import { getDashboardShell } from '@/lib/server/dashboard-data';

interface AppLayoutProps {
  title: string;
  workspaceSlug?: string | null;
  children: ReactNode;
}

export async function AppLayout({ title, workspaceSlug, children }: AppLayoutProps) {
  const shell = await getDashboardShell(workspaceSlug);

  return (
    <SidebarProvider
      style={
        {
          '--sidebar-width': 'calc(var(--spacing) * 72)',
          '--header-height': 'calc(var(--spacing) * 12)',
        } as CSSProperties
      }
    >
      <AppSidebar
        variant="inset"
        user={shell.user}
        organization={shell.organization}
        activeWorkspace={shell.activeWorkspace}
        workspaces={shell.workspaces}
      />
      <SidebarInset>
        <SiteHeader title={title} activeWorkspaceName={shell.activeWorkspace.name} />
        <div className="flex flex-1 flex-col">
          <div className="@container/main flex flex-1 flex-col gap-2">
            <div className="flex flex-col gap-4 py-4 md:gap-6 md:py-6">{children}</div>
          </div>
        </div>
      </SidebarInset>
    </SidebarProvider>
  );
}
