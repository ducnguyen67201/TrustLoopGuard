"use client"

import { IconCirclePlusFilled, IconMail, type Icon } from "@tabler/icons-react"
import Link from "next/link"
import { usePathname } from "next/navigation"

import { Button } from "@/components/ui/button"
import {
  SidebarGroup,
  SidebarGroupContent,
  SidebarMenu,
  SidebarMenuButton,
  SidebarMenuItem,
} from "@/components/ui/sidebar"
import { QuickCreateAgentDialog } from "@/components/workspace/QuickCreateAgentDialog"

type NavItem = {
  title: string
  url: string
  icon?: Icon
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
              tooltip={item.title}
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

export function NavMain({ items }: { items: NavItem[] }) {
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
                <span>Quick Create</span>
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
        <NavList items={items} />
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
