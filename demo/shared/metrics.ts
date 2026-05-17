import type { Decision, GuardLogEvent } from '@trustloopguard/sdk';

export interface DemoMetric {
  label: string;
  branch: GuardLogEvent['branch'];
  verdict: Decision['verdict'];
  traceId: string;
  latencyMs: number;
}

export class Metrics {
  private readonly rows: DemoMetric[] = [];

  record(label: string, event: GuardLogEvent): void {
    this.rows.push({
      label,
      branch: event.branch,
      verdict: event.verdict,
      traceId: event.trace_id,
      latencyMs: event.latency_ms,
    });
  }

  latest(): DemoMetric | null {
    return this.rows.at(-1) ?? null;
  }

  printSummary(): void {
    if (this.rows.length === 0) return;

    const latencies = this.rows.map((row) => row.latencyMs).sort((a, b) => a - b);
    const p95 = percentile(latencies, 0.95);
    const avg = Math.round(latencies.reduce((sum, value) => sum + value, 0) / latencies.length);

    process.stdout.write('='.repeat(72) + '\n');
    process.stdout.write(`Pipeline: ${this.rows.length} guard checks, avg=${avg} ms, p95=${p95} ms\n`);
    for (const row of this.rows) {
      process.stdout.write(
        `  ${row.label.padEnd(30)} verdict=${row.verdict.padEnd(8)} branch=${row.branch.padEnd(
          8,
        )} latency=${String(row.latencyMs).padStart(4)} ms trace=${row.traceId || '(none)'}\n`,
      );
    }
  }
}

function percentile(sorted: number[], fraction: number): number {
  if (sorted.length === 0) return 0;
  const index = Math.min(sorted.length - 1, Math.ceil(sorted.length * fraction) - 1);
  return sorted[index] ?? 0;
}
