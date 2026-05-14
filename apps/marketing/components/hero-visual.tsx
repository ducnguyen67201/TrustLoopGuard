// Deterministic latency series — stable across SSR/CSR.
const LATENCY: readonly number[] = [
  2.1, 1.8, 2.4, 1.9, 2.7, 1.6, 2.0, 1.4, 1.2, 1.7,
  2.3, 1.5, 1.9, 2.6, 1.3, 1.1, 2.2, 1.8, 1.6, 2.4,
  1.2, 1.0, 1.5, 2.0, 1.3, 1.7, 2.5, 1.4, 1.1, 1.8,
  1.6, 2.1, 1.3, 1.0, 1.4, 1.9, 1.2, 1.5, 2.2, 1.6,
  1.1, 1.3, 1.8, 1.4, 1.0, 1.5, 1.7, 1.2, 1.6, 1.3,
  0.9, 1.1, 1.4, 1.0, 1.3, 0.8, 1.2, 1.0, 0.9, 1.1,
];

export function HeroVisual() {
  return (
    <div className="float-in float-in-4 w-full">
      <div className="surface overflow-hidden">
        {/* Window header */}
        <div className="flex items-center justify-between border-b border-[var(--color-border)] px-4 py-2.5">
          <div className="flex items-center gap-2 font-mono text-[11px] text-[var(--color-ink-mute)]">
            <span className="text-[var(--color-ink)]">POST</span>
            <span>/v1/check</span>
          </div>
          <span className="font-mono text-[11px] text-[var(--color-ink-mute)]">
            200 OK · 0.9 ms
          </span>
        </div>

        {/* Body */}
        <div className="space-y-5 p-5 sm:p-6">
          <CodeLine k="prompt" v={'"show me my password"'} />
          <CodeLine
            k="proposal"
            v={'"here it is: hunter2"'}
            tone="warn"
          />

          <div className="my-5 h-px bg-[var(--color-border)]" />

          <div>
            <div className="flex items-center justify-between">
              <span className="font-mono text-[11px] uppercase tracking-[0.16em] text-[var(--color-ink-mute)]">
                Response
              </span>
              <span className="inline-flex items-center gap-1.5 rounded-full bg-[var(--color-block)]/10 px-2 py-0.5 text-[11px] font-medium text-[var(--color-block)]">
                <span
                  aria-hidden
                  className="inline-block h-1.5 w-1.5 rounded-full bg-[var(--color-block)]"
                />
                block
              </span>
            </div>
            <p className="mt-3 text-sm text-[var(--color-ink)]">
              Refused — leaked secret pattern detected in proposal.
            </p>

            <div className="mt-5 space-y-2.5">
              <Detector name="prompt_injection" confidence={0.92} />
              <Detector name="secrets" confidence={0.81} />
              <Detector name="pii" confidence={0.12} muted />
            </div>
          </div>
        </div>

        {/* Footer: latency sparkline */}
        <div className="border-t border-[var(--color-border)] px-5 py-4 sm:px-6">
          <div className="flex items-center justify-between text-[11px] text-[var(--color-ink-mute)]">
            <span>Latency · last 60 requests</span>
            <span className="font-mono">
              p50 <span className="text-[var(--color-ink)]">1.4ms</span>
              <span className="mx-2 text-[var(--color-border-strong)]">·</span>
              p95 <span className="text-[var(--color-ink)]">2.6ms</span>
            </span>
          </div>
          <Sparkline values={LATENCY} />
        </div>
      </div>
    </div>
  );
}

function CodeLine({
  k,
  v,
  tone,
}: {
  k: string;
  v: string;
  tone?: 'warn';
}) {
  return (
    <div className="flex items-baseline gap-3 font-mono text-[12.5px] leading-relaxed">
      <span className="w-20 shrink-0 text-[var(--color-ink-mute)]">{k}</span>
      <span
        className={
          tone === 'warn'
            ? 'text-[var(--color-block)]'
            : 'text-[var(--color-ink)]'
        }
      >
        {v}
      </span>
    </div>
  );
}

function Detector({
  name,
  confidence,
  muted,
}: {
  name: string;
  confidence: number;
  muted?: boolean;
}) {
  const pct = Math.round(confidence * 100);
  const fill = muted ? 'var(--color-ink-mute)' : 'var(--color-ink)';
  return (
    <div className="flex items-center gap-3 font-mono text-[11px]">
      <span
        className={`w-40 truncate ${muted ? 'text-[var(--color-ink-mute)]' : 'text-[var(--color-ink)]'}`}
      >
        {name}
      </span>
      <div className="relative h-1 flex-1 overflow-hidden rounded-full bg-[var(--color-border)]">
        <span
          className="absolute inset-y-0 left-0 rounded-full"
          style={{ width: `${pct}%`, background: fill }}
        />
      </div>
      <span className="w-9 shrink-0 text-right text-[var(--color-ink-mute)]">
        {pct}%
      </span>
    </div>
  );
}

function Sparkline({ values }: { values: readonly number[] }) {
  const W = 320;
  const H = 40;
  const PAD = 2;
  const max = Math.max(...values) * 1.05;
  const min = Math.max(0, Math.min(...values) * 0.6);
  const stepX = (W - PAD * 2) / (values.length - 1);
  const scaleY = (v: number) =>
    H - PAD - ((v - min) / (max - min)) * (H - PAD * 2);

  const pts = values.map((v, i) => [PAD + i * stepX, scaleY(v)] as const);
  const head = pts[0];
  if (!head) return null;
  let d = `M ${head[0]} ${head[1]}`;
  for (let i = 1; i < pts.length; i++) {
    const prev = pts[i - 1];
    const curr = pts[i];
    if (!prev || !curr) continue;
    const midX = (prev[0] + curr[0]) / 2;
    const midY = (prev[1] + curr[1]) / 2;
    d += ` Q ${prev[0]} ${prev[1]} ${midX} ${midY}`;
  }
  const last = pts[pts.length - 1];
  if (last) d += ` T ${last[0]} ${last[1]}`;

  return (
    <svg
      viewBox={`0 0 ${W} ${H}`}
      className="mt-3 h-10 w-full"
      role="img"
      aria-label="Latency sparkline over the last 60 requests"
    >
      <path
        d={d}
        fill="none"
        stroke="var(--color-ink)"
        strokeWidth="1.25"
        strokeLinecap="round"
        strokeLinejoin="round"
      />
    </svg>
  );
}
