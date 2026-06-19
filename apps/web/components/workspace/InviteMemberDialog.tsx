'use client';

import { IconUsersPlus } from '@tabler/icons-react';
import { useRouter, useSearchParams } from 'next/navigation';
import { useState, type FormEvent } from 'react';
import { toast } from 'sonner';

import { Button } from '@/components/ui/button';
import {
  Dialog,
  DialogContent,
  DialogTrigger,
} from '@/components/ui/dialog';
import { InfoHint } from '@/components/ui/info-hint';
import { Input } from '@/components/ui/input';
import { Label } from '@/components/ui/label';
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from '@/components/ui/select';
import {
  DialogFooterBar,
  DialogShellHeader,
  FieldHint,
  FormRow,
} from '@/components/workspace/dialog-scaffold';

type Role = 'admin' | 'editor' | 'viewer';

const ROLE_HINTS: Record<Role, string> = {
  admin: 'Can do everything you can — invite people, change policies, and manage settings. Pick this for a co-owner you fully trust.',
  editor: 'Can create and change policies and agents, but not manage members or settings. Pick this for hands-on teammates.',
  viewer: 'Can look but not touch — sees dashboards and activity, changes nothing. Pick this for stakeholders who just need visibility.',
};

type CreateInviteResponse =
  | { kind: 'added'; member: { username: string; role: string } }
  | { kind: 'invited'; invite: { email: string; role: string } };

export function InviteMemberDialog() {
  const router = useRouter();
  const searchParams = useSearchParams();
  const workspace = searchParams.get('workspace') ?? '';

  const [open, setOpen] = useState(false);
  const [email, setEmail] = useState('');
  const [role, setRole] = useState<Role>('viewer');
  const [submitting, setSubmitting] = useState(false);

  async function onSubmit(event: FormEvent<HTMLFormElement>) {
    event.preventDefault();
    if (submitting) return;
    setSubmitting(true);

    const queryString = workspace
      ? `?workspace=${encodeURIComponent(workspace)}`
      : '';
    try {
      const res = await fetch(`/api/team/invites${queryString}`, {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({ email: email.trim(), role }),
      });
      const text = await res.text();
      if (!res.ok) {
        const message = safeMessage(text) ?? `invite failed (${res.status})`;
        toast.error(message);
        return;
      }
      const data = JSON.parse(text) as CreateInviteResponse;
      if (data.kind === 'added') {
        toast.success(`Added ${data.member.username} to the workspace`);
      } else {
        toast.success(
          `Invited ${data.invite.email} — they'll join automatically when they sign up`,
        );
      }
      setEmail('');
      setRole('viewer');
      setOpen(false);
      router.refresh();
    } catch (err) {
      const message = err instanceof Error ? err.message : 'invite failed';
      toast.error(message);
    } finally {
      setSubmitting(false);
    }
  }

  return (
    <Dialog open={open} onOpenChange={setOpen}>
      <DialogTrigger asChild>
        <Button>
          <IconUsersPlus />
          Invite member
        </Button>
      </DialogTrigger>
      <DialogContent className="sm:max-w-md">
        <form onSubmit={onSubmit} className="grid gap-5">
          <DialogShellHeader
            icon={<IconUsersPlus />}
            eyebrow="Invite member"
            title="Add a teammate"
            description="Inviting someone lets them into this workspace with the access level you choose. If they already have an account, they're added right away. If not, we'll hold their spot — they join automatically when they sign up with this email."
          />
          <fieldset disabled={submitting} className="grid gap-4">
            <FormRow>
              <Label htmlFor="invite-email">Their email address</Label>
              <Input
                id="invite-email"
                type="email"
                required
                autoComplete="off"
                placeholder="teammate@example.com"
                value={email}
                onChange={(e) => setEmail(e.target.value)}
                className="font-mono"
              />
              <FieldHint>Use the email they&apos;ll sign in with. The invite is tied to this exact address.</FieldHint>
            </FormRow>
            <FormRow>
              <Label htmlFor="invite-role" className="flex items-center gap-1.5">
                What they can do
                <InfoHint term="role" />
              </Label>
              <Select value={role} onValueChange={(v) => setRole(v as Role)}>
                <SelectTrigger id="invite-role">
                  <SelectValue />
                </SelectTrigger>
                <SelectContent>
                  <SelectItem value="admin">Admin</SelectItem>
                  <SelectItem value="editor">Editor</SelectItem>
                  <SelectItem value="viewer">Viewer</SelectItem>
                </SelectContent>
              </Select>
              <FieldHint>{ROLE_HINTS[role]}</FieldHint>
            </FormRow>
          </fieldset>
          <DialogFooterBar
            onCancel={() => setOpen(false)}
            submitting={submitting}
            submitDisabled={email.trim() === ''}
            submitLabel="Send invite"
            submittingLabel="Sending…"
          />
        </form>
      </DialogContent>
    </Dialog>
  );
}

function safeMessage(text: string): string | null {
  try {
    const parsed = JSON.parse(text) as { message?: string; error?: string };
    return parsed.message ?? parsed.error ?? null;
  } catch {
    return text.length > 0 ? text : null;
  }
}
