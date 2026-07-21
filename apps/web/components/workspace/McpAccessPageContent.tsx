'use client';

import {
  IconCopy,
  IconPlugConnected,
  IconPlus,
  IconRefresh,
  IconShieldLock,
  IconTrash,
} from '@tabler/icons-react';
import type { McpGatewayConnection, McpGatewayTool, SideEffectClass } from '@trustloopguard/sdk';
import { useRouter } from 'next/navigation';
import { useEffect, useMemo, useState, type FormEvent, type ReactNode } from 'react';
import { toast } from 'sonner';

import { Badge } from '@/components/ui/badge';
import { Button } from '@/components/ui/button';
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from '@/components/ui/card';
import { DataTable, type DataTableColumn } from '@/components/ui/data-table';
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from '@/components/ui/dialog';
import { Input } from '@/components/ui/input';
import { Label } from '@/components/ui/label';
import { PageHeader } from '@/components/ui/page-header';
import { Select, SelectContent, SelectItem, SelectTrigger, SelectValue } from '@/components/ui/select';
import { Tabs, TabsContent, TabsList, TabsTrigger } from '@/components/ui/tabs';
import type { McpAccessPageData } from '@/lib/server/dashboard-data';
import { cn } from '@/lib/utils';

import { SetupRunway } from './mcp-access/SetupRunway';
import { SwitchyardMap } from './mcp-access/SwitchyardMap';

export function McpAccessPageContent({
  data,
  initialMemberId,
}: {
  data: McpAccessPageData;
  initialMemberId?: string | undefined;
}) {
  const defaultTab = data.isAdmin ? 'overview' : 'connect';
  return (
    <div className="mx-auto w-full max-w-6xl space-y-6 px-4 sm:px-6 lg:px-8">
      <PageHeader
        eyebrow={data.activeWorkspace.name}
        title="MCP Access"
        description="One managed endpoint for employee AI agents, with workspace assignments and runtime policy enforcement."
        descriptionClassName="max-w-2xl"
      />
      <Tabs defaultValue={defaultTab} className="gap-4">
        <TabsList>
          {data.isAdmin ? (
            <>
              <TabsTrigger value="overview">Overview</TabsTrigger>
              <TabsTrigger value="servers">Servers</TabsTrigger>
              <TabsTrigger value="tools">Tool access</TabsTrigger>
            </>
          ) : null}
          <TabsTrigger value="connect">Connect</TabsTrigger>
        </TabsList>
        {data.isAdmin ? (
          <>
            <TabsContent value="overview" className="space-y-4">
              <SwitchyardMap connections={data.connections} />
              {data.connections.length === 0 ? (
                <SetupRunway hasServer={false} hasAssignments={false} />
              ) : (
                <SetupRunway
                  hasServer
                  hasAssignments={data.tools.some((tool) => tool.assigned_user_ids.length > 0)}
                />
              )}
              <Exceptions data={data} />
            </TabsContent>
            <TabsContent value="servers">
              <Servers data={data} />
            </TabsContent>
            <TabsContent value="tools">
              <ToolAccess data={data} initialMemberId={initialMemberId} />
            </TabsContent>
          </>
        ) : null}
        <TabsContent value="connect">
          <Connect data={data} />
        </TabsContent>
      </Tabs>
    </div>
  );
}

function Exceptions({ data }: { data: McpAccessPageData }) {
  const exceptions = [...data.connections.filter((connection) => connection.last_sync_status === 'failed').map((connection) => `${connection.display_name} failed its last sync.`), ...data.tools.filter((tool) => tool.catalog_status === 'schema_changed').map((tool) => `${tool.public_name} changed schema and is hidden until sync.`)];
  return <Card><CardHeader><CardTitle>Action needed</CardTitle></CardHeader><CardContent>{exceptions.length ? <ul className="list-disc space-y-1 pl-5 text-sm text-muted-foreground">{exceptions.map((value) => <li key={value}>{value}</li>)}</ul> : <p className="text-sm text-muted-foreground">No catalog exceptions need attention.</p>}</CardContent></Card>;
}

function Servers({ data }: { data: McpAccessPageData }) {
  const router = useRouter();
  const [saving, setSaving] = useState(false);
  const [connectOpen, setConnectOpen] = useState(false);
  const enabledCount = data.connections.filter((connection) => connection.enabled).length;
  const toolCount = data.connections.reduce((total, connection) => total + connection.tool_count, 0);

  async function submit(event: FormEvent<HTMLFormElement>) {
    event.preventDefault();
    const form = event.currentTarget;
    setSaving(true);
    const values = new FormData(form);
    try {
      const response = await fetch(scoped('/api/mcp-gateway/connections', data), {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({
          display_name: values.get('name'),
          server_slug: values.get('slug'),
          endpoint_url: values.get('endpoint'),
          auth_kind: values.get('credential') ? 'static_bearer' : 'none',
          credential: values.get('credential') || undefined,
        }),
      });
      const connection = await response.json() as { id?: string; error?: string };
      if (!response.ok || !connection.id) {
        throw new Error(connection.error ?? 'Could not add server');
      }
      const sync = await fetch(
        scoped(`/api/mcp-gateway/connections/${connection.id}/sync`, data),
        { method: 'POST' },
      );
      if (!sync.ok) {
        toast.error('Server saved, but synchronization failed. The row remains available to retry.');
      } else {
        toast.success('Server connected and synchronized.');
      }
      form.reset();
      setConnectOpen(false);
      router.refresh();
    } catch (error) {
      toast.error(error instanceof Error ? error.message : 'Could not add server');
    } finally {
      setSaving(false);
    }
  }

  return (
    <>
      <Card className="gap-0 overflow-hidden py-0">
        <CardHeader className="gap-5 border-b bg-muted/30 px-5 py-5 sm:px-6">
          <div className="flex flex-col gap-4 sm:flex-row sm:items-start sm:justify-between">
            <div className="space-y-1.5">
              <CardTitle>Server fleet</CardTitle>
              <CardDescription>
                Remote Streamable HTTP servers available to this workspace.
              </CardDescription>
            </div>
            <Button type="button" onClick={() => setConnectOpen(true)}>
              <IconPlus aria-hidden />
              Connect server
            </Button>
          </div>
          <dl className="grid grid-cols-3 divide-x overflow-hidden rounded-lg border bg-background">
            <FleetMetric label="Connected" value={data.connections.length} />
            <FleetMetric label="Enabled" value={enabledCount} />
            <FleetMetric label="Tools found" value={toolCount} />
          </dl>
        </CardHeader>
        <CardContent className="p-0">
          {data.connections.length === 0 ? (
            <div className="grid justify-items-center gap-3 px-6 py-12 text-center">
              <div className="grid size-11 place-items-center rounded-lg border bg-muted/40 text-muted-foreground">
                <IconPlugConnected aria-hidden />
              </div>
              <div className="space-y-1">
                <p className="font-medium">No servers connected</p>
                <p className="max-w-sm text-sm text-muted-foreground">
                  Connect a remote MCP server to discover its tools and control workspace access.
                </p>
              </div>
              <Button type="button" variant="outline" onClick={() => setConnectOpen(true)}>
                <IconPlus aria-hidden />
                Connect your first server
              </Button>
            </div>
          ) : (
            <ul aria-label="Connected MCP servers" className="divide-y">
              {data.connections.map((connection) => (
                <ServerRow key={connection.id} connection={connection} data={data} router={router} />
              ))}
            </ul>
          )}
        </CardContent>
      </Card>

      <Dialog
        open={connectOpen}
        onOpenChange={(open) => {
          if (!saving) setConnectOpen(open);
        }}
      >
        <DialogContent className="w-[calc(100vw-2rem)] sm:max-w-lg">
          <form className="grid gap-5" onSubmit={submit}>
            <DialogHeader>
              <DialogTitle>Connect an MCP server</DialogTitle>
              <DialogDescription>
                Add a remote Streamable HTTP endpoint, then synchronize its available tools.
              </DialogDescription>
            </DialogHeader>
            <div className="grid gap-4">
              <Field label="Display name" name="name" />
              <Field label="Stable slug" name="slug" />
              <Field label="HTTPS endpoint" name="endpoint" type="url" />
              <Field
                label="Bearer token (optional)"
                name="credential"
                type="password"
                required={false}
              />
            </div>
            <div className="flex gap-3 rounded-lg border bg-muted/30 p-3 text-sm text-muted-foreground">
              <IconShieldLock className="mt-0.5 size-4 shrink-0" aria-hidden />
              <p>Credentials are encrypted, write-only, and never shown again.</p>
            </div>
            <DialogFooter>
              <Button
                type="button"
                variant="outline"
                disabled={saving}
                onClick={() => setConnectOpen(false)}
              >
                Cancel
              </Button>
              <Button type="submit" disabled={saving}>
                {saving ? 'Connecting…' : 'Connect and sync'}
              </Button>
            </DialogFooter>
          </form>
        </DialogContent>
      </Dialog>
    </>
  );
}

function FleetMetric({ label, value }: { label: string; value: number }) {
  return (
    <div className="grid gap-1 px-3 py-3 sm:px-4">
      <dt className="text-2xs font-medium uppercase tracking-label text-muted-foreground">
        {label}
      </dt>
      <dd className="font-data text-lg font-semibold tabular-nums leading-none">{value}</dd>
    </div>
  );
}

function ServerRow({
  connection,
  data,
  router,
}: {
  connection: McpGatewayConnection;
  data: McpAccessPageData;
  router: ReturnType<typeof useRouter>;
}) {
  return (
    <li className="grid gap-5 px-5 py-5 sm:px-6 lg:grid-cols-[minmax(15rem,1.2fr)_minmax(24rem,1fr)_auto] lg:items-center">
      <div className="flex min-w-0 items-center gap-3">
        <div className="grid size-10 shrink-0 place-items-center rounded-lg border bg-muted/30 text-muted-foreground">
          <IconPlugConnected className="size-5" aria-hidden />
        </div>
        <div className="min-w-0 space-y-1">
          <div className="flex flex-wrap items-center gap-2">
            <p className="truncate font-medium">{connection.display_name}</p>
            <Badge variant={connection.enabled ? 'secondary' : 'outline'} className="gap-1.5">
              <span
                className={cn(
                  'size-1.5 rounded-full',
                  connection.enabled ? 'bg-primary' : 'bg-muted-foreground',
                )}
                aria-hidden
              />
              {connection.enabled ? 'Enabled' : 'Disabled'}
            </Badge>
          </div>
          <p className="truncate font-data text-xs text-muted-foreground" title={connection.endpoint_url}>
            {new URL(connection.endpoint_url).hostname}
          </p>
        </div>
      </div>

      <dl className="grid grid-cols-3 gap-3">
        <ServerMetric label="Tools">
          <span className="font-data tabular-nums">{connection.tool_count}</span>
        </ServerMetric>
        <ServerMetric label="Credential">
          <span className={connection.credential_status === 'missing' ? 'text-destructive' : undefined}>
            {credentialLabel(connection)}
          </span>
        </ServerMetric>
        <ServerMetric label="Last sync">
          <span className={connection.last_sync_status === 'failed' ? 'text-destructive' : undefined}>
            {syncLabel(connection.last_sync_status)}
          </span>
          {connection.last_synced_at ? (
            <time
              dateTime={connection.last_synced_at}
              className="block text-2xs font-normal text-muted-foreground"
            >
              {formatSyncDate(connection.last_synced_at)}
            </time>
          ) : null}
        </ServerMetric>
      </dl>

      <div className="flex flex-wrap items-center gap-1 lg:justify-end">
        <Button
          type="button"
          variant="ghost"
          size="sm"
          aria-label={`Sync ${connection.display_name}`}
          onClick={() => void act(
            scoped(`/api/mcp-gateway/connections/${connection.id}/sync`, data),
            'POST',
            router,
          )}
        >
          <IconRefresh aria-hidden />
          Sync
        </Button>
        <Button
          type="button"
          variant="ghost"
          size="sm"
          aria-label={`${connection.enabled ? 'Disable' : 'Enable'} ${connection.display_name}`}
          onClick={() => void act(
            scoped(`/api/mcp-gateway/connections/${connection.id}`, data),
            'PATCH',
            router,
            { enabled: !connection.enabled },
          )}
        >
          {connection.enabled ? 'Disable' : 'Enable'}
        </Button>
        <Button
          type="button"
          variant="ghost"
          size="sm"
          className="text-muted-foreground hover:text-destructive"
          aria-label={`Delete ${connection.display_name}`}
          onClick={() => void act(
            scoped(`/api/mcp-gateway/connections/${connection.id}`, data),
            'DELETE',
            router,
          )}
        >
          <IconTrash aria-hidden />
          Delete
        </Button>
      </div>
    </li>
  );
}

function ServerMetric({ label, children }: { label: string; children: ReactNode }) {
  return (
    <div className="min-w-0 space-y-1">
      <dt className="text-2xs font-medium uppercase tracking-label text-muted-foreground">
        {label}
      </dt>
      <dd className="text-xs font-medium leading-snug">{children}</dd>
    </div>
  );
}

function credentialLabel(connection: McpGatewayConnection) {
  if (connection.credential_status === 'not_required') return 'Not required';
  if (connection.credential_status === 'missing') return 'Missing';
  return connection.auth_kind === 'static_bearer' ? 'Bearer secured' : 'Configured';
}

function syncLabel(status: McpGatewayConnection['last_sync_status']) {
  if (status === 'succeeded') return 'Succeeded';
  if (status === 'failed') return 'Failed';
  return 'Not synced';
}

function formatSyncDate(value: string) {
  return new Intl.DateTimeFormat('en-US', {
    month: 'short',
    day: 'numeric',
    hour: 'numeric',
    minute: '2-digit',
    timeZone: 'UTC',
    timeZoneName: 'short',
  }).format(new Date(value));
}

function ToolAccess({
  data,
  initialMemberId,
}: {
  data: McpAccessPageData;
  initialMemberId?: string | undefined;
}) {
  const router = useRouter();
  const [memberId, setMemberId] = useState(() => selectedMemberId(data, initialMemberId));
  const [tools, setTools] = useState(data.tools);
  const [updatingToolId, setUpdatingToolId] = useState<string | null>(null);

  useEffect(() => {
    setTools(data.tools);
  }, [data.tools]);

  function selectMember(nextMemberId: string) {
    setMemberId(nextMemberId);
    const url = new URL(window.location.href);
    url.searchParams.set('member', nextMemberId);
    window.history.replaceState(null, '', `${url.pathname}${url.search}${url.hash}`);
  }

  async function updateAssignment(tool: McpGatewayTool, grant: boolean) {
    setUpdatingToolId(tool.id);
    try {
      const assignedUserIds = await replaceToolAssignment(tool, memberId, grant, router, data);
      if (assignedUserIds === null) return;
      setTools((current) => current.map((currentTool) => (
        currentTool.id === tool.id
          ? { ...currentTool, assigned_user_ids: assignedUserIds }
          : currentTool
      )));
    } finally {
      setUpdatingToolId(null);
    }
  }

  const columns = useMemo<DataTableColumn<McpGatewayTool>[]>(() => [
    {
      id: 'tool',
      header: 'Tool',
      headerClassName: 'min-w-72 px-6',
      cellClassName: 'px-6 py-4',
      cell: (row) => <div><p className="font-mono text-xs font-medium">{row.public_name}</p><p className="mt-1 text-xs text-muted-foreground">{row.connection_name}</p></div>,
    },
    {
      id: 'status',
      header: 'Catalog',
      headerClassName: 'min-w-40 px-4',
      cellClassName: 'px-4 py-4',
      cell: (row) => <Badge variant="outline" className="font-normal capitalize">{row.catalog_status.replaceAll('_', ' ')}</Badge>,
    },
    {
      id: 'effect',
      header: 'Side effect',
      headerClassName: 'min-w-56 px-4',
      cellClassName: 'px-4 py-4',
      cell: (row) => <SideEffectSelect tool={row} data={data} router={router} className="w-full" />,
    },
    {
      id: 'assigned',
      header: 'Assigned',
      headerClassName: 'min-w-32 px-4',
      cellClassName: 'px-4 py-4',
      cell: (row) => <Badge variant="secondary" className="min-w-7 font-data tabular-nums">{row.assigned_user_ids.length}</Badge>,
      align: 'right',
    },
    {
      id: 'access',
      header: <span className="sr-only">Access</span>,
      headerClassName: 'w-28 px-6',
      cellClassName: 'px-6 py-4',
      cell: (row) => { const assigned = row.assigned_user_ids.includes(memberId); return <Button size="sm" variant={assigned ? 'outline' : 'default'} disabled={!memberId || row.catalog_status !== 'active' || updatingToolId === row.id} onClick={() => void updateAssignment(row, !assigned)}>{assigned ? 'Revoke' : 'Grant'}</Button>; },
      align: 'right',
    },
  ], [data, memberId, router, updatingToolId]);
  return (
    <Card className="gap-0 overflow-hidden py-0">
      <CardHeader className="border-b bg-muted/30 px-6 py-5">
        <div className="grid gap-5 md:grid-cols-2 md:items-end">
          <div className="space-y-2">
            <CardTitle>Tool access</CardTitle>
            <CardDescription>
              Assignments control discovery. Runtime policy still evaluates every permitted call.
            </CardDescription>
          </div>
          <div className="w-full space-y-1.5 md:max-w-sm md:justify-self-end">
            <Label htmlFor="mcp-member" className="text-xs font-medium text-muted-foreground">
              Member
            </Label>
            <Select value={memberId} onValueChange={selectMember}>
              <SelectTrigger id="mcp-member" className="w-full bg-background">
                <SelectValue placeholder="Choose a member" />
              </SelectTrigger>
              <SelectContent>
                {data.members.map((member) => <SelectItem key={member.user_id} value={member.user_id}>{member.username} · {member.role}</SelectItem>)}
              </SelectContent>
            </Select>
          </div>
        </div>
      </CardHeader>
      <CardContent className="p-0">
        <DataTable
          columns={columns}
          rows={tools}
          getRowKey={(row) => row.id}
          caption="MCP tools and access assignments for the selected member."
          empty="Synchronize a server to review its tools."
        />
      </CardContent>
    </Card>
  );
}

const SIDE_EFFECTS: ReadonlyArray<{ value: SideEffectClass; label: string }> = [
  { value: 'none', label: 'None' },
  { value: 'read', label: 'Read' },
  { value: 'external_communication', label: 'External communication' },
  { value: 'file_write', label: 'File write' },
  { value: 'shell_exec', label: 'Shell execution' },
  { value: 'network_call', label: 'Network call' },
  { value: 'db_mutation', label: 'Database mutation' },
  { value: 'api_mutation', label: 'API mutation' },
  { value: 'memory_write', label: 'Memory write' },
  { value: 'publish', label: 'Publish' },
];

function SideEffectSelect({ tool, data, router, className }: { tool: McpGatewayTool; data: McpAccessPageData; router: ReturnType<typeof useRouter>; className?: string }) {
  return <Select value={tool.side_effect} onValueChange={(sideEffect: SideEffectClass) => void act(scoped(`/api/mcp-gateway/tools/${tool.id}`, data), 'PATCH', router, { side_effect: sideEffect })}><SelectTrigger size="sm" className={className} aria-label={`Classify ${tool.public_name}`}><SelectValue /></SelectTrigger><SelectContent>{SIDE_EFFECTS.map((effect) => <SelectItem key={effect.value} value={effect.value}>{effect.label}</SelectItem>)}</SelectContent></Select>;
}

function Connect({ data }: { data: McpAccessPageData }) {
  const config = JSON.stringify({ mcpServers: { trustloopguard: { type: 'http', url: data.connectInfo.resource_url } } }, null, 2);
  return <Card><CardHeader><CardTitle>Your managed connection</CardTitle><CardDescription>Every member uses the same endpoint. OAuth identity and workspace assignments personalize the tools they receive.</CardDescription></CardHeader><CardContent className="space-y-4"><div><Label htmlFor="mcp-endpoint">Remote MCP endpoint</Label><div className="flex gap-2"><Input id="mcp-endpoint" readOnly value={data.connectInfo.resource_url} /><Button variant="outline" size="icon" aria-label="Copy MCP endpoint" onClick={() => void navigator.clipboard.writeText(data.connectInfo.resource_url).then(() => toast.success('Endpoint copied'))}><IconCopy /></Button></div></div><p className="text-sm text-muted-foreground">Scope: <span className="font-mono">{data.connectInfo.scope}</span> · Policy environment: {data.connectInfo.default_environment_name}</p><pre className="overflow-x-auto rounded-lg bg-muted p-4 text-xs"><code>{config}</code></pre></CardContent></Card>;
}

function Field({ label, name, type = 'text', required = true }: { label: string; name: string; type?: string; required?: boolean }) { return <div className="space-y-1"><Label htmlFor={name}>{label}</Label><Input id={name} name={name} type={type} required={required} /></div>; }
async function act(url: string, method: 'POST' | 'PATCH' | 'PUT' | 'DELETE', router: ReturnType<typeof useRouter>, body?: Record<string, unknown>) { const init: RequestInit = { method }; if (body) { init.headers = { 'Content-Type': 'application/json' }; init.body = JSON.stringify(body); } const response = await fetch(url, init); if (!response.ok) { const value = await response.json().catch(() => ({})) as { error?: string; message?: string }; toast.error(value.message ?? value.error ?? 'MCP gateway operation failed'); return false; } toast.success('MCP gateway updated.'); router.refresh(); return true; }
async function replaceToolAssignment(tool: McpGatewayTool, memberId: string, grant: boolean, router: ReturnType<typeof useRouter>, data: McpAccessPageData) { const next = new Set(tool.assigned_user_ids); if (grant) next.add(memberId); else next.delete(memberId); const assignedUserIds = Array.from(next); return await act(scoped(`/api/mcp-gateway/tools/${tool.id}/assignments`, data), 'PUT', router, { user_ids: assignedUserIds }) ? assignedUserIds : null; }
function scoped(path: string, data: McpAccessPageData) { const params = new URLSearchParams({ workspace: data.activeWorkspace.slug, environment: data.activeEnvironment.id }); return `${path}?${params}`; }
function selectedMemberId(data: McpAccessPageData, requestedMemberId?: string) { return requestedMemberId && data.members.some((member) => member.user_id === requestedMemberId) ? requestedMemberId : (data.members[0]?.user_id ?? ''); }
