"use client"

import { usePathname, useRouter, useSearchParams } from 'next/navigation'
import { IconCheck, IconChevronDown, IconRobot, IconX } from '@tabler/icons-react'
import { cn } from '@/lib/utils'
import {
  DropdownMenu,
  DropdownMenuContent,
  DropdownMenuItem,
  DropdownMenuSeparator,
  DropdownMenuTrigger,
} from '@/components/ui/dropdown-menu'

interface AgentFilterProps {
  agents: { id: string; name: string }[]
}

export function AgentFilter({ agents }: AgentFilterProps) {
  const router = useRouter()
  const pathname = usePathname()
  const searchParams = useSearchParams()

  if (agents.length === 0) return null

  const currentId = searchParams.get('agent')
  const currentAgent = agents.find((a) => a.id === currentId) ?? null

  function navigate(agentId: string | null) {
    const params = new URLSearchParams(searchParams.toString())
    if (agentId) params.set('agent', agentId)
    else params.delete('agent')
    router.push(`${pathname}?${params.toString()}`)
  }

  return (
    <div className="px-2 pb-2 pt-1">
      <p className="mb-1 px-1 text-[10px] font-medium uppercase tracking-widest text-muted-foreground">
        Agent filter
      </p>
      <div className="flex items-center gap-1">
        <DropdownMenu>
          <DropdownMenuTrigger asChild>
            <button
              className={cn(
                'flex h-8 flex-1 items-center gap-2 rounded-md border px-2.5 text-xs font-medium transition-colors outline-none',
                currentAgent
                  ? 'border-primary/30 bg-primary/10 text-primary hover:bg-primary/15'
                  : 'border-sidebar-border bg-sidebar text-sidebar-foreground hover:bg-sidebar-accent hover:text-sidebar-accent-foreground',
              )}
            >
              <IconRobot className="size-3.5 shrink-0" />
              <span className="truncate">
                {currentAgent ? currentAgent.name : 'All agents'}
              </span>
              <IconChevronDown className="ml-auto size-3 shrink-0 opacity-40" />
            </button>
          </DropdownMenuTrigger>
          <DropdownMenuContent align="start" className="min-w-48">
            <DropdownMenuItem
              className="gap-2 text-xs"
              onSelect={() => navigate(null)}
            >
              <IconRobot className="size-3.5 text-muted-foreground" />
              All agents
              {!currentAgent && <IconCheck className="ml-auto size-3.5" />}
            </DropdownMenuItem>
            {agents.length > 0 && <DropdownMenuSeparator />}
            {agents.map((a) => (
              <DropdownMenuItem
                key={a.id}
                className="gap-2 text-xs"
                onSelect={() => navigate(a.id)}
              >
                <IconRobot className="size-3.5 text-muted-foreground" />
                <span className="truncate">{a.name}</span>
                {currentId === a.id && <IconCheck className="ml-auto size-3.5" />}
              </DropdownMenuItem>
            ))}
          </DropdownMenuContent>
        </DropdownMenu>

        {currentAgent && (
          <button
            aria-label="Clear agent filter"
            onClick={() => navigate(null)}
            className="flex size-8 shrink-0 items-center justify-center rounded-md border border-sidebar-border text-muted-foreground transition-colors hover:border-destructive/30 hover:bg-destructive/10 hover:text-destructive"
          >
            <IconX className="size-3.5" />
          </button>
        )}
      </div>
    </div>
  )
}
