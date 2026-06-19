"use client"

import { IconCirclePlusFilled, IconMail, type Icon } from "@tabler/icons-react"
import Link from "next/link"
import { usePathname } from "next/navigation"

import { Button } from "@/components/ui/button"
import {
  SidebarGroup,
  SidebarGroupContent,
  SidebarGroupLabel,
  SidebarMenu,
  SidebarMenuButton,
  SidebarMenuItem,
} from "@/components/ui/sidebar"
import { QuickCreateAgentDialog } from "@/components/workspace/QuickCreateAgentDialog"

type NavItem = {
  title: string
  url: string
  icon?: Icon
  /** Plain-language hint shown in the collapsed (icon-only) sidebar tooltip. */
  description?: string
}

type NavGroup = {
  label: string
  items: NavItem[]
}

function NavList({ items }: { items: NavItem[] }) {
  const pathname = usePathname()

  return (
    <SidebarMenu>
      {items.map((item) => {
        const isInternal = item.url.startsWith("/")
        const itemPathname = item.url.split("?")[0] || "/"
        const isActive =
          isInternal &&
          (itemPathname === "/"
            ? pathname === "/"
            : pathname === itemPathname || pathname.startsWith(`${itemPathname}/`))
        return (
          <SidebarMenuItem key={item.title}>
            <SidebarMenuButton
              asChild={isInternal}
              tooltip={item.description ? `${item.title} — ${item.description}` : item.title}
              isActive={isActive}
            >
              {isInternal ? (
                <Link href={item.url}>
                  {item.icon && <item.icon />}
                  <span>{item.title}</span>
                </Link>
              ) : (
                <>
                  {item.icon && <item.icon />}
                  <span>{item.title}</span>
                </>
              )}
            </SidebarMenuButton>
          </SidebarMenuItem>
        )
      })}
    </SidebarMenu>
  )
}

export function NavMain({ groups }: { groups: NavGroup[] }) {
  return (
    <SidebarGroup>
      <SidebarGroupContent className="flex flex-col gap-2">
        <SidebarMenu>
          <SidebarMenuItem className="flex items-center gap-2">
            <QuickCreateAgentDialog>
              <SidebarMenuButton
                aria-label="Quick create agent"
                className="min-w-8 bg-primary text-primary-foreground duration-200 ease-linear hover:bg-primary/90 hover:text-primary-foreground active:bg-primary/90 active:text-primary-foreground"
              >
                <IconCirclePlusFilled />
                <span>Create agent</span>
              </SidebarMenuButton>
            </QuickCreateAgentDialog>
            <Button
              size="icon"
              className="size-8 group-data-[collapsible=icon]:opacity-0"
              variant="outline"
            >
              <IconMail />
              <span className="sr-only">Inbox</span>
            </Button>
          </SidebarMenuItem>
        </SidebarMenu>
        {groups.map((group) => (
          <div key={group.label} className="grid gap-1">
            <SidebarGroupLabel>{group.label}</SidebarGroupLabel>
            <NavList items={group.items} />
          </div>
        ))}
      </SidebarGroupContent>
    </SidebarGroup>
  )
}

export function NavSecondary({ items }: { items: NavItem[] }) {
  return (
    <SidebarGroup>
      <SidebarGroupContent>
        <NavList items={items} />
      </SidebarGroupContent>
    </SidebarGroup>
  )
}
