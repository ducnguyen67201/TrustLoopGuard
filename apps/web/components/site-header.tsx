import { Separator } from '@/components/ui/separator';
import { SidebarTrigger } from '@/components/ui/sidebar';
import { Badge } from '@/components/ui/badge';

interface SiteHeaderProps {
  title: string;
  activeWorkspaceName: string;
  activeEnvironmentName: string;
}

export function SiteHeader({ title, activeWorkspaceName, activeEnvironmentName }: SiteHeaderProps) {
  return (
    <header className="flex h-(--header-height) shrink-0 items-center gap-2 border-b transition-[width,height] ease-linear group-has-data-[collapsible=icon]/sidebar-wrapper:h-(--header-height)">
      <div className="flex w-full items-center gap-1 px-4 lg:gap-2 lg:px-6">
        <SidebarTrigger className="-ml-1" />
        <Separator
          orientation="vertical"
          className="mx-2 data-[orientation=vertical]:h-4"
        />
        <h1 className="text-base font-medium">{title}</h1>
        <Badge variant="outline" className="ml-auto hidden rounded-sm md:inline-flex">
          {activeWorkspaceName} / {activeEnvironmentName}
        </Badge>
      </div>
    </header>
  );
}
