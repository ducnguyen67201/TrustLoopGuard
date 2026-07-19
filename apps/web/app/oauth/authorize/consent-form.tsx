'use client';

import { useState } from 'react';

import type { WorkspaceMembership } from '@/lib/workspace-access';

type Props = {
  clientId: string;
  redirectUri: string;
  state: string;
  codeChallenge: string;
  workspaces: WorkspaceMembership[];
  userLabel: string;
  resource?: string | undefined;
  scope?: string | undefined;
};

export function ConsentForm({
  clientId,
  redirectUri,
  state,
  codeChallenge,
  workspaces,
  userLabel,
  resource,
  scope,
}: Props) {
  const [workspaceId, setWorkspaceId] = useState(workspaces[0]?.id ?? '');
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);

  function redirectBack(extra: Record<string, string>) {
    const url = new URL(redirectUri);
    url.searchParams.set('state', state);
    for (const [key, value] of Object.entries(extra)) url.searchParams.set(key, value);
    window.location.href = url.toString();
  }

  async function approve() {
    if (!workspaceId) {
      setError('Select a workspace.');
      return;
    }
    setLoading(true);
    setError(null);
    try {
      const res = await fetch('/api/oauth/authorize', {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({ clientId, redirectUri, state, codeChallenge, workspaceId, resource, scope }),
      });
      if (!res.ok) {
        const body = (await res.json().catch(() => ({}))) as { error?: string };
        throw new Error(body.error ?? 'authorization failed');
      }
      const { code } = (await res.json()) as { code: string };
      redirectBack({ code });
    } catch (err) {
      setError(err instanceof Error ? err.message : 'authorization failed');
      setLoading(false);
    }
  }

  return (
    <div className="space-y-4">
      <p className="text-sm text-muted-foreground">
        Signed in as <span className="font-medium text-foreground">{userLabel}</span>
      </p>

      <div className="space-y-1">
        <label htmlFor="workspace" className="text-sm font-medium text-foreground">
          Workspace
        </label>
        <select
          id="workspace"
          value={workspaceId}
          onChange={(event) => setWorkspaceId(event.target.value)}
          className="w-full rounded-md border border-input bg-background px-3 py-2 text-sm text-foreground"
        >
          {workspaces.map((workspace) => (
            <option key={workspace.id} value={workspace.id}>
              {workspace.slug}
            </option>
          ))}
        </select>
      </div>

      {error ? (
        <p className="rounded-md bg-destructive/10 p-2 text-sm text-destructive">{error}</p>
      ) : null}

      <div className="flex gap-2 pt-2">
        <button
          type="button"
          onClick={() => redirectBack({ error: 'access_denied' })}
          disabled={loading}
          className="flex-1 rounded-md border border-input bg-background px-3 py-2 text-sm font-medium text-foreground hover:bg-accent disabled:opacity-50"
        >
          Deny
        </button>
        <button
          type="button"
          onClick={approve}
          disabled={loading || !workspaceId}
          className="flex-1 rounded-md bg-primary px-3 py-2 text-sm font-medium text-primary-foreground hover:bg-primary/90 disabled:opacity-50"
        >
          {loading ? 'Authorizing…' : 'Authorize'}
        </button>
      </div>
    </div>
  );
}
