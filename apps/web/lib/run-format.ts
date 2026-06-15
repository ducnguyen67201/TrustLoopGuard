// Shared, client-safe formatters for run rows in the live runs list and run detail views.
// Kept separate from server `dashboard-data` mappers so both client parsers reuse one source.

type MetadataRecord = Record<string, unknown>;

export function titleize(value: string): string {
  return value
    .split(/[_\s-]+/)
    .filter(Boolean)
    .map((part) => part.charAt(0).toUpperCase() + part.slice(1))
    .join(' ');
}

export function shortRunId(id: string): string {
  if (id.length <= 16) return id;
  return `${id.slice(0, 8)}...${id.slice(-4)}`;
}

export function relativeTime(date: Date): string {
  const time = date.getTime();
  if (!Number.isFinite(time)) return 'Unknown';

  const seconds = Math.max(0, Math.round((Date.now() - time) / 1000));
  if (seconds < 60) return `${seconds}s ago`;
  const minutes = Math.round(seconds / 60);
  if (minutes < 60) return `${minutes}m ago`;
  const hours = Math.round(minutes / 60);
  if (hours < 24) return `${hours}h ago`;
  const days = Math.round(hours / 24);
  return `${days}d ago`;
}

export function formatDateTime(date: Date): string {
  const time = date.getTime();
  if (!Number.isFinite(time)) return 'Unknown';
  return new Intl.DateTimeFormat(undefined, {
    dateStyle: 'medium',
    timeStyle: 'short',
  }).format(date);
}

export function formatClockTime(date: Date): string {
  const time = date.getTime();
  if (!Number.isFinite(time)) return 'Unknown';
  return new Intl.DateTimeFormat(undefined, {
    hour: 'numeric',
    minute: '2-digit',
    second: '2-digit',
  }).format(date);
}

export function metadataEntries(
  metadata: MetadataRecord,
): Array<{ label: string; value: string }> {
  return Object.entries(metadata)
    .filter(([, value]) => value !== null && value !== '')
    .map(([label, value]) => ({ label, value: stringifyMetadataValue(value) }));
}

function stringifyMetadataValue(value: unknown): string {
  if (typeof value === 'string') return value;
  if (typeof value === 'number' || typeof value === 'boolean') return String(value);
  return JSON.stringify(value);
}
