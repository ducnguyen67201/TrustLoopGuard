'use client';

import { useActionState, useState } from 'react';

import {
  createKeyAction,
  revokeKeyAction,
  type CreateKeyState,
  type RevokeKeyState,
} from './actions';
import type { ApiKeyView } from '@/lib/tl-admin/client';

const initialCreate: CreateKeyState = { ok: 'idle' };
const initialRevoke: RevokeKeyState = {};

type Props = {
  initialKeys: ApiKeyView[];
};

export function KeysClient({ initialKeys }: Props) {
  const [createState, createAction, creating] = useActionState(
    createKeyAction,
    initialCreate,
  );
  const [revokeState, revokeAction, revoking] = useActionState(
    revokeKeyAction,
    initialRevoke,
  );

  const justCreated = createState.ok === true ? createState.key : null;

  return (
    <section className="mt-10 space-y-8">
      <CreateForm action={createAction} pending={creating} state={createState} />
      {justCreated ? <PlaintextReveal plaintext={justCreated.plaintext} /> : null}
      <KeysTable
        keys={initialKeys}
        action={revokeAction}
        pending={revoking}
        {...(revokeState.error ? { error: revokeState.error } : {})}
      />
    </section>
  );
}

function CreateForm({
  action,
  pending,
  state,
}: {
  action: (formData: FormData) => void;
  pending: boolean;
  state: CreateKeyState;
}) {
  return (
    <form action={action} className="flex items-end gap-3">
      <label className="flex-1 text-sm">
        <span className="block">Key name</span>
        <input
          name="name"
          required
          maxLength={80}
          placeholder="ci-pipeline"
          className="mt-1 w-full rounded border border-[color:var(--color-border)] bg-transparent px-3 py-2 text-sm"
        />
      </label>
      <button
        type="submit"
        disabled={pending}
        className="rounded bg-[color:var(--color-accent)] px-4 py-2 text-sm font-medium text-white disabled:opacity-60"
      >
        {pending ? 'Creating...' : 'Create key'}
      </button>
      {state.ok === false ? (
        <p className="text-sm text-red-600" role="alert">
          {state.error}
        </p>
      ) : null}
    </form>
  );
}

function PlaintextReveal({ plaintext }: { plaintext: string }) {
  const [copied, setCopied] = useState(false);
  return (
    <div className="rounded border border-amber-500/40 bg-amber-500/10 p-4 text-sm">
      <p className="font-medium">Copy this token now — it will not be shown again.</p>
      <div className="mt-3 flex items-center gap-2">
        <code className="flex-1 break-all rounded bg-[color:var(--color-surface)] px-3 py-2 font-mono text-xs">
          {plaintext}
        </code>
        <button
          type="button"
          onClick={async () => {
            await navigator.clipboard.writeText(plaintext);
            setCopied(true);
            setTimeout(() => setCopied(false), 2000);
          }}
          className="rounded border border-[color:var(--color-border)] px-3 py-2 text-xs"
        >
          {copied ? 'Copied' : 'Copy'}
        </button>
      </div>
    </div>
  );
}

function KeysTable({
  keys,
  action,
  pending,
  error,
}: {
  keys: ApiKeyView[];
  action: (formData: FormData) => void;
  pending: boolean;
  error?: string;
}) {
  if (keys.length === 0) {
    return (
      <p className="text-sm text-[color:var(--color-text-muted)]">
        You have no API keys yet.
      </p>
    );
  }

  return (
    <div>
      {error ? (
        <p className="mb-3 text-sm text-red-600" role="alert">
          {error}
        </p>
      ) : null}
      <table className="w-full border-collapse text-sm">
        <thead>
          <tr className="border-b border-[color:var(--color-border)] text-left text-xs uppercase tracking-wider text-[color:var(--color-text-muted)]">
            <th className="py-2">Name</th>
            <th className="py-2">Prefix</th>
            <th className="py-2">Created</th>
            <th className="py-2">Last used</th>
            <th className="py-2">Status</th>
            <th className="py-2"></th>
          </tr>
        </thead>
        <tbody>
          {keys.map((k) => {
            const revoked = !!k.revoked_at;
            return (
              <tr key={k.id} className="border-b border-[color:var(--color-border)]">
                <td className="py-3">{k.name}</td>
                <td className="py-3 font-mono text-xs">
                  tlg_{k.prefix}_{'•'.repeat(8)}
                </td>
                <td className="py-3">{formatDate(k.created_at)}</td>
                <td className="py-3">
                  {k.last_used_at ? formatDate(k.last_used_at) : '—'}
                </td>
                <td className="py-3">
                  {revoked ? (
                    <span className="text-red-600">revoked</span>
                  ) : (
                    <span className="text-green-700">active</span>
                  )}
                </td>
                <td className="py-3 text-right">
                  {!revoked ? (
                    <form action={action}>
                      <input type="hidden" name="keyId" value={k.id} />
                      <button
                        type="submit"
                        disabled={pending}
                        className="text-xs text-red-600 underline disabled:opacity-50"
                      >
                        Revoke
                      </button>
                    </form>
                  ) : null}
                </td>
              </tr>
            );
          })}
        </tbody>
      </table>
    </div>
  );
}

function formatDate(iso: string): string {
  return new Date(iso).toLocaleString();
}
