'use client';

import type { Dispatch, ReactNode, SetStateAction } from 'react';
import { useMemo, useState } from 'react';
import { toast } from 'sonner';

import { Badge } from '@/components/ui/badge';
import { Button } from '@/components/ui/button';
import { Card, CardContent, CardHeader, CardTitle } from '@/components/ui/card';
import { Checkbox } from '@/components/ui/checkbox';
import { DataTable, type DataTableColumn } from '@/components/ui/data-table';
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from '@/components/ui/dialog';
import { EmptyState } from '@/components/ui/empty-state';
import { Input } from '@/components/ui/input';
import { Label } from '@/components/ui/label';
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from '@/components/ui/select';
import { Switch } from '@/components/ui/switch';
import type {
  BudgetAlertConfig,
  BudgetAlertFiring,
  BudgetAlertThresholdType,
  BudgetAlertWindow,
} from '@trustloopguard/sdk';

type BudgetAlertForm = {
  name: string;
  window: BudgetAlertWindow;
  principalId: string;
  thresholdType: BudgetAlertThresholdType;
  thresholdValue: string;
  webhookUrl: string;
  enabled: boolean;
};

const DEFAULT_FORM: BudgetAlertForm = {
  name: 'weekly-80-percent',
  window: 'week',
  principalId: '',
  thresholdType: 'percent',
  thresholdValue: '80',
  webhookUrl: '',
  enabled: true,
};

const WINDOW_LABELS: Record<BudgetAlertWindow, string> = {
  day: 'Daily',
  week: 'Weekly',
  month: 'Monthly',
};

export function BudgetAlertsCard({
  contextQuery,
  configs,
  firings,
}: {
  contextQuery: string;
  configs: BudgetAlertConfig[];
  firings: BudgetAlertFiring[];
}) {
  const [rows, setRows] = useState(configs);
  const [dialogOpen, setDialogOpen] = useState(false);
  const [busyIds, setBusyIds] = useState<string[]>([]);
  const busySet = useMemo(() => new Set(busyIds), [busyIds]);
  const nameByConfigId = useMemo(
    () => new Map(rows.map((config) => [config.id, config.name])),
    [rows],
  );

  async function toggleEnabled(config: BudgetAlertConfig, enabled: boolean) {
    setBusyIds((prev) => [...prev, config.id]);
    try {
      const response = await fetch(
        `/api/financial/budget-alerts/${encodeURIComponent(config.id)}${contextQuery}`,
        {
          method: 'PATCH',
          headers: { 'Content-Type': 'application/json' },
          body: JSON.stringify({ enabled }),
        },
      );
      const text = await response.text();
      if (!response.ok) {
        throw new Error(safeError(text) ?? 'Unable to update budget alert');
      }
      const updated = JSON.parse(text) as BudgetAlertConfig;
      setRows((prev) => prev.map((row) => (row.id === updated.id ? updated : row)));
      toast.success(enabled ? 'Budget alert enabled' : 'Budget alert disabled');
    } catch (error) {
      toast.error(error instanceof Error ? error.message : 'Unable to update budget alert');
    } finally {
      setBusyIds((prev) => prev.filter((id) => id !== config.id));
    }
  }

  async function deleteConfig(config: BudgetAlertConfig) {
    setBusyIds((prev) => [...prev, config.id]);
    try {
      const response = await fetch(
        `/api/financial/budget-alerts/${encodeURIComponent(config.id)}${contextQuery}`,
        { method: 'DELETE' },
      );
      if (!response.ok && response.status !== 204) {
        throw new Error(safeError(await response.text()) ?? 'Unable to delete budget alert');
      }
      setRows((prev) => prev.filter((row) => row.id !== config.id));
      toast.success('Budget alert deleted');
    } catch (error) {
      toast.error(error instanceof Error ? error.message : 'Unable to delete budget alert');
    } finally {
      setBusyIds((prev) => prev.filter((id) => id !== config.id));
    }
  }

  const configColumns: DataTableColumn<BudgetAlertConfig>[] = [
    {
      id: 'name',
      header: 'Name',
      cell: (row) => <span className="font-medium">{row.name}</span>,
    },
    {
      id: 'window',
      header: 'Window',
      cell: (row) => WINDOW_LABELS[row.window],
    },
    {
      id: 'threshold',
      header: 'Threshold',
      cell: (row) => thresholdLabel(row),
    },
    {
      id: 'scope',
      header: 'Principal',
      cell: (row) =>
        row.principal_id ?? <span className="text-muted-foreground">Any principal</span>,
    },
    {
      id: 'webhook',
      header: 'Webhook',
      cell: (row) =>
        row.webhook_url ? (
          <Badge variant="outline">Custom</Badge>
        ) : (
          <Badge variant="outline">Workspace default</Badge>
        ),
    },
    {
      id: 'enabled',
      header: 'Enabled',
      cell: (row) => (
        <Switch
          checked={row.enabled}
          disabled={busySet.has(row.id)}
          aria-label={`Toggle ${row.name}`}
          onCheckedChange={(checked) => void toggleEnabled(row, checked)}
        />
      ),
    },
    {
      id: 'actions',
      header: '',
      align: 'right',
      cell: (row) => (
        <Button
          variant="ghost"
          size="sm"
          disabled={busySet.has(row.id)}
          onClick={() => void deleteConfig(row)}
        >
          Delete
        </Button>
      ),
    },
  ];

  const firingColumns: DataTableColumn<BudgetAlertFiring>[] = [
    {
      id: 'fired_at',
      header: 'Fired',
      cell: (row) => new Date(row.fired_at).toLocaleString(),
    },
    {
      id: 'config',
      header: 'Alert',
      cell: (row) => nameByConfigId.get(row.config_id) ?? row.config_id,
    },
    {
      id: 'principal',
      header: 'Principal',
      cell: (row) => row.principal_id,
    },
    {
      id: 'spend',
      header: 'Spend at firing',
      align: 'right',
      cell: (row) =>
        `${formatMinor(row.spent_minor, row.currency)} of ${formatMinor(row.cap_minor, row.currency)}`,
    },
  ];

  return (
    <>
      <Card>
        <CardHeader className="flex flex-row items-center justify-between">
          <CardTitle>Budget alerts</CardTitle>
          <Button variant="outline" onClick={() => setDialogOpen(true)}>
            New alert
          </Button>
        </CardHeader>
        <CardContent className="grid gap-4">
          <DataTable
            columns={configColumns}
            rows={rows}
            getRowKey={(row) => row.id}
            empty={
              <EmptyState
                title="No budget alerts"
                description="Get a webhook before a budget window hits its hard cap — for example at 80% of the weekly budget."
                action={
                  <Button variant="outline" onClick={() => setDialogOpen(true)}>
                    Create budget alert
                  </Button>
                }
              />
            }
            caption="Budget alert configs"
          />
          <div className="grid gap-2">
            <p className="text-sm font-medium">Recent firings</p>
            <DataTable
              columns={firingColumns}
              rows={firings}
              getRowKey={(row) => row.id}
              empty={
                <EmptyState
                  title="No alerts fired yet"
                  description="Firings appear here the first time spend crosses a configured threshold in a window."
                />
              }
              caption="Budget alert firings"
            />
          </div>
        </CardContent>
      </Card>
      <BudgetAlertCreateDialog
        open={dialogOpen}
        onOpenChange={setDialogOpen}
        contextQuery={contextQuery}
        onCreated={(config) => setRows((prev) => [...prev, config])}
      />
    </>
  );
}

function BudgetAlertCreateDialog({
  open,
  onOpenChange,
  contextQuery,
  onCreated,
}: {
  open: boolean;
  onOpenChange: (open: boolean) => void;
  contextQuery: string;
  onCreated: (config: BudgetAlertConfig) => void;
}) {
  const [saving, setSaving] = useState(false);
  const [form, setForm] = useState<BudgetAlertForm>(DEFAULT_FORM);

  async function createAlert() {
    let payload: ReturnType<typeof formPayload>;
    try {
      payload = formPayload(form);
    } catch (error) {
      toast.error(error instanceof Error ? error.message : 'Alert is invalid');
      return;
    }
    setSaving(true);
    try {
      const response = await fetch(`/api/financial/budget-alerts${contextQuery}`, {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify(payload),
      });
      const text = await response.text();
      if (!response.ok) {
        throw new Error(safeError(text) ?? 'Unable to create budget alert');
      }
      onCreated(JSON.parse(text) as BudgetAlertConfig);
      onOpenChange(false);
      setForm(DEFAULT_FORM);
      toast.success('Budget alert created');
    } catch (error) {
      toast.error(error instanceof Error ? error.message : 'Unable to create budget alert');
    } finally {
      setSaving(false);
    }
  }

  return (
    <Dialog open={open} onOpenChange={onOpenChange}>
      <DialogContent className="max-w-xl">
        <DialogHeader>
          <DialogTitle>Create budget alert</DialogTitle>
          <DialogDescription>
            Get a webhook when spend crosses a threshold of a capped budget window — before the
            hard limit blocks.
          </DialogDescription>
        </DialogHeader>
        <div className="grid gap-4">
          <div className="grid gap-3 md:grid-cols-2">
            <Field label="Name">
              <Input
                value={form.name}
                onChange={(event) => setFormValue(setForm, 'name', event.target.value)}
              />
            </Field>
            <Field label="Window">
              <Select
                value={form.window}
                onValueChange={(value) =>
                  setFormValue(setForm, 'window', value as BudgetAlertWindow)
                }
              >
                <SelectTrigger className="w-full">
                  <SelectValue />
                </SelectTrigger>
                <SelectContent>
                  <SelectItem value="day">Daily</SelectItem>
                  <SelectItem value="week">Weekly</SelectItem>
                  <SelectItem value="month">Monthly</SelectItem>
                </SelectContent>
              </Select>
            </Field>
            <Field label="Threshold type">
              <Select
                value={form.thresholdType}
                onValueChange={(value) =>
                  setFormValue(setForm, 'thresholdType', value as BudgetAlertThresholdType)
                }
              >
                <SelectTrigger className="w-full">
                  <SelectValue />
                </SelectTrigger>
                <SelectContent>
                  <SelectItem value="percent">Percent of cap</SelectItem>
                  <SelectItem value="absolute">Amount remaining</SelectItem>
                </SelectContent>
              </Select>
            </Field>
            <Field
              label={form.thresholdType === 'percent' ? 'Percent (1-100)' : 'Remaining (dollars)'}
            >
              <Input
                inputMode="decimal"
                value={form.thresholdValue}
                onChange={(event) => setFormValue(setForm, 'thresholdValue', event.target.value)}
              />
            </Field>
            <Field label="Principal (optional)">
              <Input
                placeholder="Any principal"
                value={form.principalId}
                onChange={(event) => setFormValue(setForm, 'principalId', event.target.value)}
              />
            </Field>
            <Field label="Webhook URL (optional)">
              <Input
                placeholder="Workspace escalation webhook"
                value={form.webhookUrl}
                onChange={(event) => setFormValue(setForm, 'webhookUrl', event.target.value)}
              />
            </Field>
          </div>
          <label className="flex items-center gap-2 rounded-md border px-3 py-2 text-sm">
            <Checkbox
              checked={form.enabled}
              onCheckedChange={(checked) => setFormValue(setForm, 'enabled', checked === true)}
            />
            Enabled
          </label>
        </div>
        <DialogFooter>
          <Button type="button" variant="outline" onClick={() => onOpenChange(false)}>
            Cancel
          </Button>
          <Button type="button" disabled={saving} onClick={createAlert}>
            {saving ? 'Saving...' : 'Create budget alert'}
          </Button>
        </DialogFooter>
      </DialogContent>
    </Dialog>
  );
}

function Field({ label, children }: { label: string; children: ReactNode }) {
  return (
    <div className="grid gap-1.5">
      <Label>{label}</Label>
      {children}
    </div>
  );
}

function thresholdLabel(config: BudgetAlertConfig): string {
  if (config.threshold_type === 'percent') {
    return `${config.threshold_value}% of cap`;
  }
  return `${formatMinor(config.threshold_value, 'USD')} remaining`;
}

function formatMinor(minor: number | bigint, currency: string): string {
  return `${(Number(minor) / 100).toFixed(2)} ${currency}`;
}

function formPayload(form: BudgetAlertForm) {
  const name = form.name.trim();
  if (name === '') throw new Error('Name is required');
  const rawValue = form.thresholdValue.trim();
  let thresholdValue: number;
  if (form.thresholdType === 'percent') {
    if (!/^\d+$/.test(rawValue)) {
      throw new Error('Percent must be a whole number between 1 and 100');
    }
    thresholdValue = Number(rawValue);
    if (thresholdValue < 1 || thresholdValue > 100) {
      throw new Error('Percent must be a whole number between 1 and 100');
    }
  } else {
    thresholdValue = dollarsToMinor(rawValue);
  }
  const principalId = form.principalId.trim();
  const webhookUrl = form.webhookUrl.trim();
  return {
    name,
    window: form.window,
    principal_id: principalId === '' ? undefined : principalId,
    threshold_type: form.thresholdType,
    threshold_value: thresholdValue,
    webhook_url: webhookUrl === '' ? undefined : webhookUrl,
    enabled: form.enabled,
  };
}

function dollarsToMinor(value: string): number {
  if (!/^\d+(\.\d{1,2})?$/.test(value)) {
    throw new Error('Amounts must be positive dollars with up to two decimals');
  }
  const [dollars, cents = ''] = value.split('.');
  return Number(dollars) * 100 + Number(cents.padEnd(2, '0'));
}

function setFormValue<K extends keyof BudgetAlertForm>(
  setForm: Dispatch<SetStateAction<BudgetAlertForm>>,
  key: K,
  value: BudgetAlertForm[K],
) {
  setForm((prev) => ({ ...prev, [key]: value }));
}

function safeError(text: string): string | null {
  try {
    const parsed = JSON.parse(text) as { error?: string; message?: string };
    return parsed.error ?? parsed.message ?? null;
  } catch {
    return text.trim() === '' ? null : text;
  }
}
