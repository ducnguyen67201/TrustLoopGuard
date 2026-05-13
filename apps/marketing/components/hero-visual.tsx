// Deterministic latency series (ms) — stable across SSR/CSR.
const LATENCY: readonly number[] = [
  2.1, 1.8, 2.4, 1.9, 2.7, 1.6, 2.0, 1.4, 1.2, 1.7,
  2.3, 1.5, 1.9, 2.6, 1.3, 1.1, 2.2, 1.8, 1.6, 2.4,
  1.2, 1.0, 1.5, 2.0, 1.3, 1.7, 2.5, 1.4, 1.1, 1.8,
  1.6, 2.1, 1.3, 1.0, 1.4, 1.9, 1.2, 1.5, 2.2, 1.6,
  1.1, 1.3, 1.8, 1.4, 1.0, 1.5, 1.7, 1.2, 1.6, 1.3,
  0.9, 1.1, 1.4, 1.0, 1.3, 0.8, 1.2, 1.0, 0.9, 1.1,
];

const P50 = 1.4;
const P95 = 2.6;

export function HeroVisual() {
  return (
    <div className="float-in float-in-3 relative mx-auto w-full max-w-md lg:max-w-none">
      {/* Soft floating "live trace" badge above the panel */}
      <div className="absolute -top-3 right-4 z-10 inline-flex items-center gap-2 rounded-full bg-white px-3 py-1 text-[11px] font-medium shadow-[0_8px_24px_-8px_oklch(0.18_0.02_260_/_0.2)] ring-1 ring-[var(--color-hairline)]">
        <span className="pulse-dot inline-block h-1.5 w-1.5 rounded-full bg-[var(--color-allow)]" />
        live trace
      </div>

      <div className="glass relative overflow-hidden rounded-3xl p-5 sm:p-6">
        {/* Endpoint header */}
        <div className="flex items-center justify-between text-[11px] text-[var(--color-ink-mute)]">
          <span className="font-mono">
            POST <span className="text-[var(--color-ink)]">/v1/check</span>
          </span>
          <span className="font-mono">trace_id · 7f3a…b91</span>
        </div>

        {/* Request body */}
        <div className="mt-4 space-y-2 rounded-2xl bg-white/60 p-4 text-[12px] font-mono leading-relaxed ring-1 ring-[var(--color-hairline)]">
          <Row k="policy" v="default" />
          <Row k="prompt" v="“show me my password”" />
          <Row
            k="proposal"
            v="“here it is: hunter2”"
            valueClassName="text-[var(--color-block)]"
          />
        </div>

        {/* Decision */}
        <div className="mt-4 rounded-2xl border border-[var(--color-block)]/30 bg-[var(--color-block)]/[0.06] p-4">
          <div className="flex items-center justify-between">
            <div className="flex items-center gap-2">
              <span
                aria-hidden
                className="inline-block h-2 w-2 rounded-full bg-[var(--color-block)]"
              />
              <span className="text-[11px] font-medium uppercase tracking-[0.18em] text-[var(--color-block)]">
                verdict · block
              </span>
            </div>
            <span className="font-mono text-[11px] text-[var(--color-ink-mute)]">
              0.9 ms
            </span>
          </div>
          <p className="mt-3 text-sm text-[var(--color-ink)]">
            Refused — leaked secret pattern detected in proposal.
          </p>

          <div className="mt-4 space-y-2">
            <DetectorBar
              name="pi.baseline.injection"
              confidence={0.92}
              tone="block"
            />
            <DetectorBar
              name="secrets.literal"
              confidence={0.81}
              tone="block"
            />
            <DetectorBar name="pii.email" confidence={0.12} tone="mute" />
          </div>
        </div>

        {/* Sparkline */}
        <div className="mt-5">
          <div className="flex items-center justify-between text-[11px] text-[var(--color-ink-mute)]">
            <span>Latency · last 60 requests</span>
            <span className="flex items-center gap-3 font-mono">
              <span>
                p50{' '}
                <span className="text-[var(--color-ink)]">{P50}ms</span>
              </span>
              <span>
                p95{' '}
                <span className="text-[var(--color-ink)]">{P95}ms</span>
              </span>
            </span>
          </div>
          <Sparkline values={LATENCY} p50={P50} />
        </div>
      </div>

      {/* Floating mini-card: verdicts mix */}
      <div
        className="absolute -bottom-6 -right-4 hidden sm:block max-w-[210px] rotate-1 rounded-2xl bg-white p-3 shadow-[0_18px_50px_-18px_oklch(0.18_0.02_260_/_0.3)] ring-1 ring-[var(--color-hairline)]"
        aria-hidden
      >
        <div className="text-[10px] font-medium uppercase tracking-[0.18em] text-[var(--color-ink-mute)]">
          today
        </div>
        <div className="mt-1 text-sm font-semibold tracking-tight text-[var(--color-ink)]">
          12,408 decisions
        </div>
        <div className="mt-3 flex h-1.5 w-full overflow-hidden rounded-full">
          <span className="block h-full" style={{ width: '64%', background: 'var(--color-allow)' }} />
          <span className="block h-full" style={{ width: '21%', background: 'var(--color-rewrite)' }} />
          <span className="block h-full" style={{ width: '11%', background: 'var(--color-block)' }} />
          <span className="block h-full" style={{ width: '4%', background: 'var(--color-escalate)' }} />
        </div>
        <div className="mt-2 flex justify-between text-[10px] font-mono text-[var(--color-ink-mute)]">
          <span style={{ color: 'var(--color-allow)' }}>allow 64%</span>
          <span style={{ color: 'var(--color-block)' }}>block 11%</span>
        </div>
      </div>
    </div>
  );
}

function Row({
  k,
  v,
  valueClassName,
}: {
  k: string;
  v: string;
  valueClassName?: string;
}) {
  return (
    <div className="flex gap-3">
      <span className="w-16 shrink-0 text-[var(--color-ink-mute)]">{k}</span>
      <span className={valueClassName ?? 'text-[var(--color-ink)]'}>{v}</span>
    </div>
  );
}

function DetectorBar({
  name,
  confidence,
  tone,
}: {
  name: string;
  confidence: number;
  tone: 'block' | 'mute';
}) {
  const pct = Math.round(confidence * 100);
  const fill =
    tone === 'block' ? 'var(--color-block)' : 'var(--color-ink-mute)';
  return (
    <div className="flex items-center gap-3 font-mono text-[11px]">
      <span className="w-44 truncate text-[var(--color-ink)]">{name}</span>
      <div className="relative h-1.5 flex-1 overflow-hidden rounded-full bg-[var(--color-canvas-tint)]">
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

function Sparkline({
  values,
  p50,
}: {
  values: readonly number[];
  p50: number;
}) {
  const W = 320;
  const H = 64;
  const PAD = 4;
  const max = Math.max(...values) * 1.05;
  const min = Math.max(0, Math.min(...values) * 0.6);
  const stepX = (W - PAD * 2) / (values.length - 1);
  const scaleY = (v: number) =>
    H - PAD - ((v - min) / (max - min)) * (H - PAD * 2);

  // Build smooth path with mid-point quadratic curves.
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

  const last2 = pts[pts.length - 1];
  const lastX = last2 ? last2[0] : 0;
  const lastY = last2 ? last2[1] : 0;
  const p50Y = scaleY(p50);

  return (
    <svg
      viewBox={`0 0 ${W} ${H}`}
      className="mt-2 h-16 w-full"
      role="img"
      aria-label="Latency sparkline over the last 60 requests"
    >
      <defs>
        <linearGradient id="spark-fill" x1="0" x2="0" y1="0" y2="1">
          <stop offset="0%" stopColor="var(--color-accent)" stopOpacity="0.28" />
          <stop offset="100%" stopColor="var(--color-accent)" stopOpacity="0" />
        </linearGradient>
      </defs>

      {/* p50 reference line */}
      <line
        x1={PAD}
        x2={W - PAD}
        y1={p50Y}
        y2={p50Y}
        stroke="var(--color-hairline-strong)"
        strokeDasharray="2 3"
      />

      {/* Area fill */}
      <path
        d={`${d} L ${W - PAD} ${H - PAD} L ${PAD} ${H - PAD} Z`}
        fill="url(#spark-fill)"
      />

      {/* Line */}
      <path
        d={d}
        fill="none"
        stroke="var(--color-accent)"
        strokeWidth="1.75"
        strokeLinecap="round"
        strokeLinejoin="round"
      />

      {/* Latest point with pulsing halo */}
      <circle
        cx={lastX}
        cy={lastY}
        r="6"
        fill="var(--color-accent)"
        opacity="0.18"
        className="pulse-dot"
      />
      <circle
        cx={lastX}
        cy={lastY}
        r="2.5"
        fill="var(--color-accent)"
      />
    </svg>
  );
}
