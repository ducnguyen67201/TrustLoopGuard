'use client';

import type { EnforcementMode } from '@trustloopguard/sdk';
import { useEffect, useState } from 'react';
import { toast } from 'sonner';
import { z } from 'zod';

import { Button } from '@/components/ui/button';
import { Label } from '@/components/ui/label';
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from '@/components/ui/select';

const mode = z.enum(['off', 'shadow', 'enforce']);
const modesSchema = z.object({
  flow_checker_mode: mode.optional(),
  memory_checker_mode: mode.optional(),
  param_checker_mode: mode.optional(),
  approval_checker_mode: mode.optional(),
  updated_at: z.string().optional(),
});
type ParsedModes = z.infer<typeof modesSchema>;

export function ApprovalCheckerModeControl({
  workspaceSlug,
  environmentId,
}: {
  workspaceSlug: string;
  environmentId: string;
}) {
  const [modes, setModes] = useState<ParsedModes | null>(null);
  const [selected, setSelected] = useState<EnforcementMode>('off');
  const [saving, setSaving] = useState(false);
  const query = `?workspace=${encodeURIComponent(workspaceSlug)}&environment=${encodeURIComponent(environmentId)}`;
  const endpoint = `/api/environments/${encodeURIComponent(environmentId)}/checker-modes${query}`;

  useEffect(() => {
    const controller = new AbortController();
    void fetch(endpoint, { signal: controller.signal })
      .then(async (response) => {
        if (!response.ok) throw new Error(`Failed to load checker modes (${response.status})`);
        return modesSchema.parse(await response.json());
      })
      .then((loaded) => {
        setModes(loaded);
        setSelected(loaded.approval_checker_mode ?? 'off');
      })
      .catch((error) => {
        if (!controller.signal.aborted)
          toast.error(error instanceof Error ? error.message : 'Failed to load checker modes');
      });
    return () => controller.abort();
  }, [endpoint]);

  async function save() {
    if (!modes) return;
    setSaving(true);
    try {
      const { updated_at: _updatedAt, ...currentOverrides } = modes;
      const response = await fetch(endpoint, {
        method: 'PUT',
        headers: { 'content-type': 'application/json' },
        body: JSON.stringify({ ...currentOverrides, approval_checker_mode: selected }),
      });
      if (!response.ok) throw new Error(`Failed to save checker mode (${response.status})`);
      const updated = modesSchema.parse(await response.json());
      setModes(updated);
      setSelected(updated.approval_checker_mode ?? 'off');
      toast.success('Approval checker mode updated');
    } catch (error) {
      toast.error(error instanceof Error ? error.message : 'Failed to save checker mode');
    } finally {
      setSaving(false);
    }
  }

  return (
    <div className="grid gap-2">
      <Label htmlFor="approval-checker-mode">Tool approval checker</Label>
      <div className="flex gap-2">
        <Select
          value={selected}
          onValueChange={(value) => setSelected(value as EnforcementMode)}
          disabled={!modes || saving}
        >
          <SelectTrigger id="approval-checker-mode" className="w-full">
            <SelectValue />
          </SelectTrigger>
          <SelectContent>
            <SelectItem value="off">Off</SelectItem>
            <SelectItem value="shadow">Shadow</SelectItem>
            <SelectItem value="enforce">Enforce</SelectItem>
          </SelectContent>
        </Select>
        <Button type="button" onClick={save} disabled={!modes || saving}>
          Save
        </Button>
      </div>
      <p className="text-xs text-muted-foreground">
        Off skips approval checks. Shadow records evidence without holding calls. Enforce creates a
        durable exact-action approval before execution.
      </p>
    </div>
  );
}
