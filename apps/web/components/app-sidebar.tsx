"use client"

import * as React from "react"
import {
  IconBook2,
  IconBuilding,
  IconDashboard,
  IconKey,
  IconInnerShadowTop,
  IconRobot,
  IconSettings,
  IconShieldCheck,
  IconUsers,
} from "@tabler/icons-react"
import { Check, ChevronsUpDown, Plus } from "lucide-react"
import Link from "next/link"

import { NavMain } from "@/components/nav-main"
import { NavUser } from "@/components/nav-user"
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
} from "@/components/ui/sidebar"

const data = {
  navMain: [
    {
      title: "Dashboard",
      url: "/",
      icon: IconDashboard,
    },
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
  ...props
}: AppSidebarProps) {
  const navItems = data.navMain.map((item) => ({
    ...item,
    url: item.url.startsWith('/') ? withWorkspace(item.url, activeWorkspace.slug) : item.url,
  }));

  return (
    <Sidebar collapsible="offcanvas" {...props}>
      <SidebarHeader>
        <SidebarMenu>
          <SidebarMenuItem>
            <SidebarMenuButton
              asChild
              className="data-[slot=sidebar-menu-button]:p-1.5!"
            >
              <Link href={`/?workspace=${activeWorkspace.slug}`}>
                <IconInnerShadowTop className="size-5!" />
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
      </SidebarHeader>
      <SidebarContent>
        <NavMain items={navItems} />
      </SidebarContent>
      <SidebarFooter>
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

function withWorkspace(url: string, workspaceSlug: string): string {
  const separator = url.includes('?') ? '&' : '?';
  return `${url}${separator}workspace=${encodeURIComponent(workspaceSlug)}`;
}
