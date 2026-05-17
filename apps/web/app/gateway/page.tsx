import { AppLayout } from '@/components/AppLayout';
import { rustApiForWorkspace, workspaceIdFromSlug } from '@/lib/server/tl-client';
import type { ReactNode } from 'react';

type ProviderConnection = {
  id: string;
  display_name: string;
  kind: string;
  base_url?: string | null;
  default_model: string;
  credential_status: string;
};

type EnforcementProfile = {
  id: string;
  display_name: string;
  input_action: string;
  output_action: string;
  fail_mode: string;
  retention_mode: string;
};

type GatewayRoute = {
  id: string;
  display_name: string;
  provider_connection_id: string;
  agent_id: string;
  enforcement_profile_id: string;
};

export default async function GatewayPage({
  searchParams,
}: {
  searchParams: Promise<{ workspace?: string | string[] }>;
}) {
  const workspaceSlug = readWorkspaceSlug(await searchParams);
  const workspaceId = workspaceIdFromSlug(workspaceSlug);
  const [providers, profiles, routes] = await Promise.all([
    safeLoad<{ provider_connections: ProviderConnection[] }>(
      workspaceId,
      '/v1/gateway/provider-connections',
      { provider_connections: [] },
    ),
    safeLoad<{ enforcement_profiles: EnforcementProfile[] }>(
      workspaceId,
      '/v1/enforcement-profiles',
      { enforcement_profiles: [] },
    ),
    safeLoad<{ gateway_routes: GatewayRoute[] }>(workspaceId, '/v1/gateway/routes', {
      gateway_routes: [],
    }),
  ]);

  return (
    <AppLayout title="Gateway" workspaceSlug={workspaceSlug}>
      <main className="space-y-6">
        <section className="grid gap-4 md:grid-cols-3">
          <SummaryCard label="Provider connections" value={providers.provider_connections.length} />
          <SummaryCard label="Enforcement profiles" value={profiles.enforcement_profiles.length} />
          <SummaryCard label="Gateway routes" value={routes.gateway_routes.length} />
        </section>

        <section className="grid gap-6 xl:grid-cols-3">
          <Panel title="Provider Connections">
            <Rows
              empty="No provider connections yet."
              rows={providers.provider_connections.map((provider) => ({
                id: provider.id,
                title: provider.display_name,
                detail: `${provider.kind} · ${provider.default_model}`,
                meta: provider.credential_status,
              }))}
            />
          </Panel>

          <Panel title="Enforcement Profiles">
            <Rows
              empty="No enforcement profiles yet."
              rows={profiles.enforcement_profiles.map((profile) => ({
                id: profile.id,
                title: profile.display_name,
                detail: `input ${profile.input_action} · output ${profile.output_action}`,
                meta: `${profile.fail_mode} · ${profile.retention_mode}`,
              }))}
            />
          </Panel>

          <Panel title="Gateway Routes">
            <Rows
              empty="No gateway routes yet."
              rows={routes.gateway_routes.map((route) => ({
                id: route.id,
                title: route.display_name,
                detail: `${route.agent_id} · ${route.provider_connection_id}`,
                meta: route.enforcement_profile_id,
              }))}
            />
          </Panel>
        </section>

        <section className="rounded-lg border border-slate-200 bg-white p-5">
          <h2 className="text-sm font-semibold text-slate-950">Integration</h2>
          <div className="mt-4 grid gap-4 lg:grid-cols-2">
            <Snippet
              title="OpenAI-compatible"
              code={`const openai = new OpenAI({
  apiKey: process.env.TLG_API_KEY,
  baseURL: "https://api.trustloopguard.com/v1/gateway/<route_id>/openai",
});`}
            />
            <Snippet
              title="Anthropic"
              code={`const anthropic = new Anthropic({
  apiKey: process.env.TLG_API_KEY,
  baseURL: "https://api.trustloopguard.com/v1/gateway/<route_id>/anthropic",
});`}
            />
          </div>
        </section>
      </main>
    </AppLayout>
  );
}

function SummaryCard({ label, value }: { label: string; value: number }) {
  return (
    <div className="rounded-lg border border-slate-200 bg-white p-4">
      <div className="text-xs font-medium uppercase tracking-wide text-slate-500">{label}</div>
      <div className="mt-2 text-2xl font-semibold text-slate-950">{value}</div>
    </div>
  );
}

function Panel({ title, children }: { title: string; children: ReactNode }) {
  return (
    <section className="rounded-lg border border-slate-200 bg-white p-4">
      <h2 className="text-sm font-semibold text-slate-950">{title}</h2>
      <div className="mt-3">{children}</div>
    </section>
  );
}

function Rows({
  rows,
  empty,
}: {
  rows: Array<{ id: string; title: string; detail: string; meta: string }>;
  empty: string;
}) {
  if (rows.length === 0) {
    return <p className="text-sm text-slate-500">{empty}</p>;
  }
  return (
    <div className="divide-y divide-slate-100">
      {rows.map((row) => (
        <div key={row.id} className="py-3 first:pt-0 last:pb-0">
          <div className="flex items-center justify-between gap-3">
            <div className="min-w-0">
              <p className="truncate text-sm font-medium text-slate-950">{row.title}</p>
              <p className="truncate text-xs text-slate-500">{row.detail}</p>
            </div>
            <span className="shrink-0 rounded-md bg-slate-100 px-2 py-1 text-xs text-slate-600">
              {row.meta}
            </span>
          </div>
        </div>
      ))}
    </div>
  );
}

function Snippet({ title, code }: { title: string; code: string }) {
  return (
    <div>
      <h3 className="text-xs font-medium uppercase tracking-wide text-slate-500">{title}</h3>
      <pre className="mt-2 overflow-x-auto rounded-lg bg-slate-950 p-4 text-xs text-slate-50">
        <code>{code}</code>
      </pre>
    </div>
  );
}

async function safeLoad<T>(workspaceId: string, path: string, fallback: T): Promise<T> {
  try {
    return await rustApiForWorkspace<T>(workspaceId, path, { method: 'GET' });
  } catch {
    return fallback;
  }
}

function readWorkspaceSlug(searchParams: { workspace?: string | string[] }): string | null {
  const value = searchParams.workspace;
  return Array.isArray(value) ? (value[0] ?? null) : (value ?? null);
}
