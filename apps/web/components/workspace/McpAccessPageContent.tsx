'use client';

import { IconCopy, IconRefresh, IconTrash } from '@tabler/icons-react';
import type { McpGatewayConnection, McpGatewayTool, SideEffectClass } from '@trustloopguard/sdk';
import { useRouter } from 'next/navigation';
import { useMemo, useState, type FormEvent } from 'react';
import { toast } from 'sonner';

import { Badge } from '@/components/ui/badge';
import { Button } from '@/components/ui/button';
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from '@/components/ui/card';
import { DataTable, type DataTableColumn } from '@/components/ui/data-table';
import { Input } from '@/components/ui/input';
import { Label } from '@/components/ui/label';
import { PageHeader } from '@/components/ui/page-header';
import { Select, SelectContent, SelectItem, SelectTrigger, SelectValue } from '@/components/ui/select';
import { Tabs, TabsContent, TabsList, TabsTrigger } from '@/components/ui/tabs';
import type { McpAccessPageData } from '@/lib/server/dashboard-data';

import { SetupRunway } from './mcp-access/SetupRunway';
import { SwitchyardMap } from './mcp-access/SwitchyardMap';

export function McpAccessPageContent({ data }: { data: McpAccessPageData }) {
  const defaultTab = data.isAdmin ? 'overview' : 'connect';
  return <div className="space-y-6"><PageHeader eyebrow={data.activeWorkspace.name} title="MCP Access" description="One managed endpoint for employee AI agents, with workspace assignments and runtime policy enforcement." descriptionClassName="max-w-4xl" /><Tabs defaultValue={defaultTab}><TabsList>{data.isAdmin ? <><TabsTrigger value="overview">Overview</TabsTrigger><TabsTrigger value="servers">Servers</TabsTrigger><TabsTrigger value="tools">Tool access</TabsTrigger></> : null}<TabsTrigger value="connect">Connect</TabsTrigger></TabsList>{data.isAdmin ? <><TabsContent value="overview" className="space-y-4"><SwitchyardMap connections={data.connections} />{data.connections.length === 0 ? <SetupRunway hasServer={false} hasAssignments={false} /> : <SetupRunway hasServer hasAssignments={data.tools.some((tool) => tool.assigned_user_ids.length > 0)} />}<Exceptions data={data} /></TabsContent><TabsContent value="servers"><Servers data={data} /></TabsContent><TabsContent value="tools"><ToolAccess data={data} /></TabsContent></> : null}<TabsContent value="connect"><Connect data={data} /></TabsContent></Tabs></div>;
}

function Exceptions({ data }: { data: McpAccessPageData }) {
  const exceptions = [...data.connections.filter((connection) => connection.last_sync_status === 'failed').map((connection) => `${connection.display_name} failed its last sync.`), ...data.tools.filter((tool) => tool.catalog_status === 'schema_changed').map((tool) => `${tool.public_name} changed schema and is hidden until sync.`)];
  return <Card><CardHeader><CardTitle>Action needed</CardTitle></CardHeader><CardContent>{exceptions.length ? <ul className="list-disc space-y-1 pl-5 text-sm text-muted-foreground">{exceptions.map((value) => <li key={value}>{value}</li>)}</ul> : <p className="text-sm text-muted-foreground">No catalog exceptions need attention.</p>}</CardContent></Card>;
}

function Servers({ data }: { data: McpAccessPageData }) {
  const router = useRouter();
  const [saving, setSaving] = useState(false);
  const columns: DataTableColumn<McpGatewayConnection>[] = [
    { id: 'name', header: 'Server', cell: (row) => <div><p className="font-medium">{row.display_name}</p><p className="text-xs text-muted-foreground">{new URL(row.endpoint_url).hostname}</p></div> },
    { id: 'enabled', header: 'State', cell: (row) => <Badge variant="outline">{row.enabled ? 'Enabled' : 'Disabled'}</Badge> },
    { id: 'credential', header: 'Credential', cell: (row) => row.credential_status },
    { id: 'tools', header: 'Tools', cell: (row) => row.tool_count, align: 'right' },
    { id: 'sync', header: 'Sync', cell: (row) => row.last_sync_status },
    { id: 'actions', header: '', cell: (row) => <div className="flex justify-end gap-1"><Button variant="ghost" size="icon-sm" aria-label={`Sync ${row.display_name}`} onClick={() => void act(scoped(`/api/mcp-gateway/connections/${row.id}/sync`, data), 'POST', router)}><IconRefresh /></Button><Button variant="ghost" size="sm" onClick={() => void act(scoped(`/api/mcp-gateway/connections/${row.id}`, data), 'PATCH', router, { enabled: !row.enabled })}>{row.enabled ? 'Disable' : 'Enable'}</Button><Button variant="ghost" size="icon-sm" aria-label={`Delete ${row.display_name}`} onClick={() => void act(scoped(`/api/mcp-gateway/connections/${row.id}`, data), 'DELETE', router)}><IconTrash /></Button></div>, align: 'right' },
  ];
  async function submit(event: FormEvent<HTMLFormElement>) { event.preventDefault(); const form = event.currentTarget; setSaving(true); const values = new FormData(form); try { const response = await fetch(scoped('/api/mcp-gateway/connections', data), { method: 'POST', headers: { 'Content-Type': 'application/json' }, body: JSON.stringify({ display_name: values.get('name'), server_slug: values.get('slug'), endpoint_url: values.get('endpoint'), auth_kind: values.get('credential') ? 'static_bearer' : 'none', credential: values.get('credential') || undefined }) }); const connection = await response.json() as { id?: string; error?: string }; if (!response.ok || !connection.id) throw new Error(connection.error ?? 'Could not add server'); const sync = await fetch(scoped(`/api/mcp-gateway/connections/${connection.id}/sync`, data), { method: 'POST' }); if (!sync.ok) toast.error('Server saved, but synchronization failed. The row remains available to retry.'); else toast.success('Server connected and synchronized.'); form.reset(); router.refresh(); } catch (error) { toast.error(error instanceof Error ? error.message : 'Could not add server'); } finally { setSaving(false); } }
  return <div className="grid gap-4 xl:grid-cols-[minmax(0,2fr)_minmax(18rem,1fr)]"><Card><CardHeader><CardTitle>Servers</CardTitle><CardDescription>Only remote Streamable HTTP servers are supported.</CardDescription></CardHeader><CardContent><DataTable columns={columns} rows={data.connections} getRowKey={(row) => row.id} empty="No MCP servers connected." /></CardContent></Card><Card><CardHeader><CardTitle>Connect a server</CardTitle><CardDescription>Credentials are write-only and are never shown again.</CardDescription></CardHeader><CardContent><form className="space-y-3" onSubmit={submit}><Field label="Display name" name="name" /><Field label="Stable slug" name="slug" /><Field label="HTTPS endpoint" name="endpoint" type="url" /><Field label="Bearer token (optional)" name="credential" type="password" required={false} /><Button type="submit" disabled={saving}>{saving ? 'Connecting…' : 'Connect and sync'}</Button></form></CardContent></Card></div>;
}

function ToolAccess({ data }: { data: McpAccessPageData }) {
  const router = useRouter();
  const [memberId, setMemberId] = useState(data.members[0]?.user_id ?? '');
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
      cell: (row) => { const assigned = row.assigned_user_ids.includes(memberId); return <Button size="sm" variant={assigned ? 'outline' : 'default'} disabled={!memberId || row.catalog_status !== 'active'} onClick={() => void replaceToolAssignment(row, memberId, !assigned, router, data)}>{assigned ? 'Revoke' : 'Grant'}</Button>; },
      align: 'right',
    },
  ], [data, memberId, router]);
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
            <Select value={memberId} onValueChange={setMemberId}>
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
          rows={data.tools}
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
async function act(url: string, method: 'POST' | 'PATCH' | 'PUT' | 'DELETE', router: ReturnType<typeof useRouter>, body?: Record<string, unknown>) { const init: RequestInit = { method }; if (body) { init.headers = { 'Content-Type': 'application/json' }; init.body = JSON.stringify(body); } const response = await fetch(url, init); if (!response.ok) { const value = await response.json().catch(() => ({})) as { error?: string; message?: string }; toast.error(value.message ?? value.error ?? 'MCP gateway operation failed'); return; } toast.success('MCP gateway updated.'); router.refresh(); }
async function replaceToolAssignment(tool: McpGatewayTool, memberId: string, grant: boolean, router: ReturnType<typeof useRouter>, data: McpAccessPageData) { const next = new Set(tool.assigned_user_ids); if (grant) next.add(memberId); else next.delete(memberId); await act(scoped(`/api/mcp-gateway/tools/${tool.id}/assignments`, data), 'PUT', router, { user_ids: Array.from(next) }); }
function scoped(path: string, data: McpAccessPageData) { const params = new URLSearchParams({ workspace: data.activeWorkspace.slug, environment: data.activeEnvironment.id }); return `${path}?${params}`; }
