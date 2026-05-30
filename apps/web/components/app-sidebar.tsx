"use client"

import * as React from "react"
import {
  IconBook2,
  IconBuilding,
  IconChartBar,
  IconDashboard,
  IconActivity,
  IconKey,
  IconRobot,
  IconSettings,
  IconShieldCheck,
  IconUsers,
} from "@tabler/icons-react"
import { Check, ChevronsUpDown, Plus } from "lucide-react"
import Link from "next/link"
import { usePathname, useRouter, useSearchParams } from "next/navigation"

import { AgentFilter } from "@/components/AgentFilter"
import { BrandLogo } from "@/components/brand-logo"
import { NavMain, NavSecondary } from "@/components/nav-main"
import { NavUser } from "@/components/nav-user"
import { ThemeToggle } from "@/components/theme-toggle"
import { CreateEnvironmentDialog } from "@/components/workspace/CreateEnvironmentDialog"
import type { DashboardShellData } from "@/lib/server/dashboard-data"
import {
  DropdownMenu,
  DropdownMenuContent,
  DropdownMenuItem,
  DropdownMenuLabel,
  DropdownMenuSeparator,
  DropdownMenuTrigger,
} from "@/components/ui/dropdown-menu"
import {
  Sidebar,
  SidebarContent,
  SidebarFooter,
  SidebarHeader,
  SidebarMenu,
  SidebarMenuButton,
  SidebarMenuItem,
  SidebarSeparator,
} from "@/components/ui/sidebar"

const data = {
  navMain: [
    {
      label: "Monitor",
      items: [
        {
          title: "Dashboard",
          url: "/",
          icon: IconDashboard,
        },
        {
          title: "Runs",
          url: "/runs",
          icon: IconActivity,
        },
        {
          title: "Analytics",
          url: "/analytics",
          icon: IconChartBar,
        },
      ],
    },
    {
      label: "Configure",
      items: [
        {
          title: "Policies",
          url: "/policies",
          icon: IconShieldCheck,
        },
        {
          title: "Agents",
          url: "/agents",
          icon: IconRobot,
        },
        {
          title: "Knowledge",
          url: "/knowledge-sources",
          icon: IconBook2,
        },
        {
          title: "Gateway",
          url: "/gateway",
          icon: IconActivity,
        },
      ],
    },
  ],
  navSecondary: [
    {
      title: "API Keys",
      url: "/api-keys",
      icon: IconKey,
    },
    {
      title: "Team",
      url: "/team",
      icon: IconUsers,
    },
    {
      title: "Settings",
      url: "/settings",
      icon: IconSettings,
    },
    {
      title: "Docs",
      url: "/docs",
      icon: IconBook2,
    },
  ],
}

type AppSidebarProps = React.ComponentProps<typeof Sidebar> & DashboardShellData;

export function AppSidebar({
  user,
  organization,
  activeWorkspace,
  workspaces,
  activeEnvironment,
  environments,
  agents,
  ...props
}: AppSidebarProps) {
  const searchParams = useSearchParams();
  const activeAgent = searchParams.get('agent');

  const withContext = (url: string) => {
    if (!url.startsWith('/')) return url;
    const params = new URLSearchParams();
    params.set('workspace', activeWorkspace.slug);
    params.set('environment', activeEnvironment.id);
    if (activeAgent) params.set('agent', activeAgent);
    return `${url}?${params.toString()}`;
  };
  const navGroups = data.navMain.map((group) => ({
    ...group,
    items: group.items.map((item) => ({ ...item, url: withContext(item.url) })),
  }));
  const navSecondaryItems = data.navSecondary.map((item) => ({ ...item, url: withContext(item.url) }));

  return (
    <Sidebar collapsible="offcanvas" {...props}>
      <SidebarHeader>
        <SidebarMenu>
          <SidebarMenuItem>
            <SidebarMenuButton
              asChild
              className="data-[slot=sidebar-menu-button]:p-1.5!"
            >
              <Link href={`/?workspace=${activeWorkspace.slug}&environment=${encodeURIComponent(activeEnvironment.id)}`}>
                <BrandLogo className="size-5" priority />
                <span className="text-base font-semibold">TrustLoopGuard</span>
              </Link>
            </SidebarMenuButton>
          </SidebarMenuItem>
        </SidebarMenu>
        <WorkspaceSwitcher
          organization={organization}
          activeWorkspace={activeWorkspace}
          workspaces={workspaces}
        />
        <EnvironmentSwitcher
          activeWorkspace={activeWorkspace}
          activeEnvironment={activeEnvironment}
          environments={environments}
        />
        <AgentFilter agents={agents} />
      </SidebarHeader>
      <SidebarContent className="overflow-x-hidden overflow-y-auto">
        <NavMain groups={navGroups} />
        <div className="mt-auto flex flex-col">
          <SidebarSeparator className="mx-2" />
          <NavSecondary items={navSecondaryItems} />
        </div>
      </SidebarContent>
      <SidebarFooter>
        <ThemeToggle />
        <NavUser user={user} />
      </SidebarFooter>
    </Sidebar>
  )
}

function WorkspaceSwitcher({
  organization,
  activeWorkspace,
  workspaces,
}: Pick<DashboardShellData, "organization" | "activeWorkspace" | "workspaces">) {
  return (
    <SidebarMenu>
      <SidebarMenuItem>
        <DropdownMenu>
          <DropdownMenuTrigger asChild>
            <SidebarMenuButton size="lg" className="mt-2 border border-sidebar-border">
              <div className="flex aspect-square size-8 items-center justify-center border bg-sidebar-accent text-sidebar-accent-foreground">
                <IconBuilding className="size-4" />
              </div>
              <div className="grid flex-1 text-left text-sm leading-tight">
                <span className="truncate font-medium">{activeWorkspace.name}</span>
                <span className="truncate text-xs text-muted-foreground">
                  {organization.name}
                </span>
              </div>
              <ChevronsUpDown className="ml-auto size-4 text-muted-foreground" />
            </SidebarMenuButton>
          </DropdownMenuTrigger>
          <DropdownMenuContent
            className="w-(--radix-dropdown-menu-trigger-width) min-w-64"
            align="start"
            side="bottom"
          >
            <DropdownMenuLabel className="text-xs text-muted-foreground">
              Workspaces
            </DropdownMenuLabel>
            {workspaces.map((workspace) => (
              <DropdownMenuItem key={workspace.id} asChild>
              <Link href={`/?workspace=${workspace.slug}`}>
                  <div className="grid flex-1">
                    <span>{workspace.name}</span>
                    <span className="text-xs text-muted-foreground">{workspace.role}</span>
                  </div>
                  {workspace.id === activeWorkspace.id ? <Check className="size-4" /> : null}
                </Link>
              </DropdownMenuItem>
            ))}
            <DropdownMenuSeparator />
            <DropdownMenuItem asChild>
              <Link href={`/workspaces?workspace=${activeWorkspace.slug}`}>
                <Plus className="size-4" />
                Manage workspaces
              </Link>
            </DropdownMenuItem>
          </DropdownMenuContent>
        </DropdownMenu>
      </SidebarMenuItem>
    </SidebarMenu>
  )
}

function EnvironmentSwitcher({
  activeWorkspace,
  activeEnvironment,
  environments,
}: Pick<DashboardShellData, "activeWorkspace" | "activeEnvironment" | "environments">) {
  const router = useRouter();
  const pathname = usePathname();
  const searchParams = useSearchParams();
  const [createOpen, setCreateOpen] = React.useState(false);

  function environmentHref(environmentId: string) {
    const params = new URLSearchParams(searchParams.toString());
    params.set('workspace', activeWorkspace.slug);
    params.set('environment', environmentId);
    return `${pathname}?${params.toString()}`;
  }

  function onCreatedEnvironment(environment: { id: string }) {
    router.push(environmentHref(environment.id));
    router.refresh();
  }

  return (
    <>
      <SidebarMenu>
        <SidebarMenuItem>
          <DropdownMenu>
            <DropdownMenuTrigger asChild>
              <SidebarMenuButton size="lg" className="border border-sidebar-border">
                <div className="flex aspect-square size-8 items-center justify-center border bg-sidebar-accent text-sidebar-accent-foreground">
                  <IconActivity className="size-4" />
                </div>
                <div className="grid flex-1 text-left text-sm leading-tight">
                  <span className="truncate font-medium">{activeEnvironment.name}</span>
                  <span className="truncate text-xs text-muted-foreground">
                    environment
                  </span>
                </div>
                <ChevronsUpDown className="ml-auto size-4 text-muted-foreground" />
              </SidebarMenuButton>
            </DropdownMenuTrigger>
            <DropdownMenuContent
              className="w-(--radix-dropdown-menu-trigger-width) min-w-64"
              align="start"
              side="bottom"
            >
              <DropdownMenuLabel className="text-xs text-muted-foreground">
                Environments
              </DropdownMenuLabel>
              {environments.map((environment) => (
                <DropdownMenuItem key={environment.id} asChild>
                  <Link href={environmentHref(environment.id)}>
                    <div className="grid flex-1">
                      <span>{environment.name}</span>
                      <span className="text-xs text-muted-foreground">{environment.slug}</span>
                    </div>
                    {environment.id === activeEnvironment.id ? <Check className="size-4" /> : null}
                  </Link>
                </DropdownMenuItem>
              ))}
              <DropdownMenuSeparator />
              <DropdownMenuItem
                onSelect={(event) => {
                  event.preventDefault();
                  setCreateOpen(true);
                }}
              >
                <Plus className="size-4" />
                New environment
              </DropdownMenuItem>
            </DropdownMenuContent>
          </DropdownMenu>
        </SidebarMenuItem>
      </SidebarMenu>
      <CreateEnvironmentDialog
        open={createOpen}
        onOpenChange={setCreateOpen}
        workspaceSlug={activeWorkspace.slug}
        onCreated={onCreatedEnvironment}
      />
    </>
  )
}
