/**
 * The branded vulnerability-report PDF, built with `@react-pdf/renderer`.
 *
 * Rendered server-side only (see `route.ts`). This is not a DOM component — it
 * produces PDF primitives. Brand values are the resolved hex of the dashboard's
 * design tokens (`apps/web/app/globals.css`); react-pdf can't read CSS vars or
 * `oklch`, so the severity palette is hardcoded here.
 */
import { Document, Page, Polygon, StyleSheet, Svg, Text, View } from '@react-pdf/renderer';
import type { JSX } from 'react';

import type {
  ComparedAttackStatus,
  RedteamReportComparison,
  RedteamReportFinding,
  RedteamReportPayload,
  ReportSeverity,
} from '@/lib/redteam-report';

const COLORS = {
  ink: '#0a0a0a',
  body: '#222222',
  muted: '#6b6b6b',
  faint: '#9a9a9a',
  surface: '#f6f6f6',
  surfaceAlt: '#fbfbfb',
  border: '#e4e4e4',
  borderStrong: '#cecece',
  primary: '#ff6900',
  white: '#ffffff',
  critical: '#e5484d',
  high: '#f76808',
  medium: '#e0a017',
  low: '#3b7dd8',
  info: '#8a8a8a',
  good: '#2e9e6b',
} as const;

const SEVERITY: Record<ReportSeverity, { label: string; color: string }> = {
  critical: { label: 'Critical', color: COLORS.critical },
  high: { label: 'High', color: COLORS.high },
  medium: { label: 'Medium', color: COLORS.medium },
  low: { label: 'Low', color: COLORS.low },
  info: { label: 'Info', color: COLORS.info },
};

function outcomeStyle(outcome: string): { label: string; color: string } {
  switch (outcome) {
    case 'landed':
      return { label: 'Landed', color: COLORS.critical };
    case 'blocked':
      return { label: 'Blocked', color: COLORS.good };
    case 'clean':
      return { label: 'Control', color: COLORS.info };
    default:
      return { label: 'Error', color: COLORS.medium };
  }
}

const COMPARISON_STATUS: Record<ComparedAttackStatus, { label: string; color: string }> = {
  fixed: { label: 'Fixed', color: COLORS.good },
  still_vulnerable: { label: 'Still exposed', color: COLORS.critical },
  regressed: { label: 'Regressed', color: COLORS.critical },
  unchanged: { label: 'Unchanged', color: COLORS.info },
};

function pct(value: number): string {
  return `${Math.round(value * 100)}%`;
}

function formatDate(iso: string): string {
  const date = new Date(iso);
  if (Number.isNaN(date.getTime())) return iso;
  return date.toLocaleDateString('en-US', { year: 'numeric', month: 'short', day: 'numeric' });
}

const styles = StyleSheet.create({
  page: {
    paddingTop: 48,
    paddingBottom: 56,
    paddingHorizontal: 44,
    fontFamily: 'Helvetica',
    fontSize: 10,
    color: COLORS.body,
    backgroundColor: COLORS.white,
  },
  topRule: { height: 3, backgroundColor: COLORS.primary, marginBottom: 18 },
  brandRow: { flexDirection: 'row', alignItems: 'center', justifyContent: 'space-between' },
  brandLeft: { flexDirection: 'row', alignItems: 'center' },
  mark: {
    width: 16,
    height: 16,
    borderRadius: 4,
    backgroundColor: COLORS.primary,
    marginRight: 7,
  },
  wordmark: { fontSize: 12, fontFamily: 'Helvetica-Bold', color: COLORS.ink, letterSpacing: 0.2 },
  kicker: { fontSize: 8, color: COLORS.faint, letterSpacing: 1.4, fontFamily: 'Helvetica-Bold' },
  title: { fontSize: 24, fontFamily: 'Helvetica-Bold', color: COLORS.ink, marginTop: 22 },
  metaRow: { flexDirection: 'row', marginTop: 8, gap: 16 },
  metaItem: { flexDirection: 'row' },
  metaLabel: { fontSize: 8, color: COLORS.faint, letterSpacing: 0.8, marginRight: 4 },
  metaValue: { fontSize: 8, color: COLORS.muted, fontFamily: 'Helvetica-Bold' },

  summaryCard: {
    marginTop: 22,
    borderWidth: 1,
    borderColor: COLORS.border,
    borderRadius: 8,
    backgroundColor: COLORS.surfaceAlt,
    padding: 18,
    flexDirection: 'row',
  },
  riskChip: {
    width: 116,
    borderRadius: 6,
    paddingVertical: 14,
    paddingHorizontal: 10,
    alignItems: 'center',
    justifyContent: 'center',
  },
  riskWord: { fontSize: 19, fontFamily: 'Helvetica-Bold', color: COLORS.white },
  riskLabel: {
    fontSize: 7,
    letterSpacing: 1.6,
    color: COLORS.white,
    fontFamily: 'Helvetica-Bold',
    marginTop: 3,
    opacity: 0.9,
  },
  summaryBody: { flex: 1, paddingLeft: 18, justifyContent: 'center' },
  headlineStat: { fontSize: 16, fontFamily: 'Helvetica-Bold', color: COLORS.ink },
  headlineSub: { fontSize: 9, color: COLORS.muted, marginTop: 3 },

  bar: {
    flexDirection: 'row',
    height: 9,
    borderRadius: 5,
    overflow: 'hidden',
    marginTop: 12,
    backgroundColor: COLORS.surface,
  },
  legendRow: { flexDirection: 'row', marginTop: 7, gap: 12, flexWrap: 'wrap' },
  legendItem: { flexDirection: 'row', alignItems: 'center' },
  legendDot: { width: 6, height: 6, borderRadius: 3, marginRight: 4 },
  legendText: { fontSize: 7.5, color: COLORS.muted },

  sectionHead: { flexDirection: 'row', alignItems: 'flex-end', marginTop: 26, marginBottom: 10 },
  sectionTitle: { fontSize: 13, fontFamily: 'Helvetica-Bold', color: COLORS.ink },
  sectionCount: { fontSize: 9, color: COLORS.faint, marginLeft: 8 },

  finding: {
    flexDirection: 'row',
    borderWidth: 1,
    borderColor: COLORS.border,
    borderRadius: 6,
    marginBottom: 8,
    overflow: 'hidden',
  },
  stripe: { width: 4 },
  findingBody: { flex: 1, padding: 11 },
  findingTopRow: { flexDirection: 'row', justifyContent: 'space-between', alignItems: 'flex-start' },
  findingName: { fontSize: 11, fontFamily: 'Helvetica-Bold', color: COLORS.ink },
  findingCategory: { fontSize: 7.5, color: COLORS.faint, letterSpacing: 0.5, marginTop: 2 },
  findingGoal: { fontSize: 9, color: COLORS.muted, marginTop: 6 },
  chip: {
    paddingVertical: 2.5,
    paddingHorizontal: 7,
    borderRadius: 9,
    fontSize: 7.5,
    fontFamily: 'Helvetica-Bold',
    color: COLORS.white,
  },
  chipRow: { flexDirection: 'row', gap: 5, alignItems: 'center' },
  evidence: {
    marginTop: 8,
    borderRadius: 4,
    backgroundColor: '#fdecec',
    borderWidth: 1,
    borderColor: '#f6c9c9',
    padding: 7,
  },
  evidenceLabel: {
    fontSize: 6.5,
    letterSpacing: 1,
    color: COLORS.critical,
    fontFamily: 'Helvetica-Bold',
    marginBottom: 3,
  },
  evidenceText: { fontSize: 8, fontFamily: 'Courier', color: '#7a1f22' },
  trace: { fontSize: 7, color: COLORS.faint, marginTop: 6, fontFamily: 'Courier' },

  deltaBanner: {
    borderWidth: 1,
    borderColor: COLORS.border,
    borderRadius: 6,
    padding: 12,
    flexDirection: 'row',
    alignItems: 'center',
    justifyContent: 'space-between',
    backgroundColor: COLORS.surfaceAlt,
    marginBottom: 12,
  },
  deltaStat: { alignItems: 'center' },
  deltaValue: { fontSize: 16, fontFamily: 'Helvetica-Bold', color: COLORS.ink },
  deltaLabel: { fontSize: 7, color: COLORS.faint, letterSpacing: 0.8, marginTop: 2 },
  // SVG right-arrow: the built-in Helvetica has no `→` glyph.
  arrow: { marginHorizontal: 5 },

  cmpRow: {
    flexDirection: 'row',
    alignItems: 'center',
    borderBottomWidth: 1,
    borderBottomColor: COLORS.border,
    paddingVertical: 8,
  },
  cmpAttack: { flex: 1, paddingRight: 8 },
  cmpAttackName: { fontSize: 9.5, fontFamily: 'Helvetica-Bold', color: COLORS.ink },
  cmpFlow: { flexDirection: 'row', alignItems: 'center', width: 150, gap: 5 },

  helpBox: {
    marginTop: 22,
    borderTopWidth: 1,
    borderTopColor: COLORS.border,
    paddingTop: 12,
  },
  helpTitle: { fontSize: 9, fontFamily: 'Helvetica-Bold', color: COLORS.ink, marginBottom: 3 },
  helpText: { fontSize: 8.5, color: COLORS.muted, lineHeight: 1.5 },

  footer: {
    position: 'absolute',
    bottom: 24,
    left: 44,
    right: 44,
    flexDirection: 'row',
    justifyContent: 'space-between',
    borderTopWidth: 1,
    borderTopColor: COLORS.border,
    paddingTop: 7,
  },
  footerText: { fontSize: 7.5, color: COLORS.faint },
});

function Chip({ label, color }: { label: string; color: string }): JSX.Element {
  return <Text style={[styles.chip, { backgroundColor: color }]}>{label}</Text>;
}

/** A small drawn right-arrow (Helvetica lacks the `→` glyph). */
function Arrow(): JSX.Element {
  return (
    <Svg width={11} height={9} style={styles.arrow}>
      <Polygon points="0,1.5 7.5,4.5 0,7.5" fill={COLORS.faint} />
    </Svg>
  );
}

function MetaItem({ label, value }: { label: string; value: string }): JSX.Element {
  return (
    <View style={styles.metaItem}>
      <Text style={styles.metaLabel}>{label}</Text>
      <Text style={styles.metaValue}>{value}</Text>
    </View>
  );
}

function SeverityBar({
  landed,
  blocked,
  clean,
  errored,
}: {
  landed: number;
  blocked: number;
  clean: number;
  errored: number;
}): JSX.Element {
  const total = Math.max(landed + blocked + clean + errored, 1);
  const segments = [
    { value: landed, color: COLORS.critical, label: 'Landed' },
    { value: blocked, color: COLORS.good, label: 'Blocked' },
    { value: clean, color: COLORS.info, label: 'Control' },
    { value: errored, color: COLORS.medium, label: 'Errored' },
  ].filter((segment) => segment.value > 0);

  return (
    <View>
      <View style={styles.bar}>
        {segments.map((segment) => (
          <View
            key={segment.label}
            style={{ width: `${(segment.value / total) * 100}%`, backgroundColor: segment.color }}
          />
        ))}
      </View>
      <View style={styles.legendRow}>
        {segments.map((segment) => (
          <View key={segment.label} style={styles.legendItem}>
            <View style={[styles.legendDot, { backgroundColor: segment.color }]} />
            <Text style={styles.legendText}>
              {segment.label} {segment.value}
            </Text>
          </View>
        ))}
      </View>
    </View>
  );
}

function Finding({ finding }: { finding: RedteamReportFinding }): JSX.Element {
  const severity = SEVERITY[finding.severity];
  const outcome = outcomeStyle(finding.outcome);
  return (
    <View style={styles.finding} wrap={false}>
      <View style={[styles.stripe, { backgroundColor: severity.color }]} />
      <View style={styles.findingBody}>
        <View style={styles.findingTopRow}>
          <View style={{ flex: 1, paddingRight: 8 }}>
            <Text style={styles.findingName}>{finding.attack}</Text>
            <Text style={styles.findingCategory}>{finding.category.replace(/_/g, ' ').toUpperCase()}</Text>
          </View>
          <View style={styles.chipRow}>
            <Chip label={severity.label} color={severity.color} />
            <Chip label={outcome.label} color={outcome.color} />
          </View>
        </View>
        <Text style={styles.findingGoal}>{finding.goal}</Text>
        {finding.landed && finding.evidence ? (
          <View style={styles.evidence}>
            <Text style={styles.evidenceLabel}>EVIDENCE — DISCLOSED IN AGENT REPLY</Text>
            <Text style={styles.evidenceText}>{finding.evidence}</Text>
          </View>
        ) : null}
        {finding.trace_id ? <Text style={styles.trace}>trace {finding.trace_id}</Text> : null}
      </View>
    </View>
  );
}

function ComparisonSection({
  comparison,
}: {
  comparison: RedteamReportComparison;
}): JSX.Element {
  const before = pct(comparison.baseline_aggregates.success_rate);
  const after = pct(comparison.compare_aggregates.success_rate);
  const delta = Math.round(comparison.delta_points);
  const improved = delta >= 0;
  return (
    <View>
      <View style={styles.sectionHead}>
        <Text style={styles.sectionTitle}>Before &amp; after</Text>
        <Text style={styles.sectionCount}>same agent, two runs</Text>
      </View>
      <View style={styles.deltaBanner}>
        <View style={styles.deltaStat}>
          <Text style={[styles.deltaValue, { color: COLORS.critical }]}>{before}</Text>
          <Text style={styles.deltaLabel}>BASELINE LANDED</Text>
        </View>
        <Arrow />
        <View style={styles.deltaStat}>
          <Text style={[styles.deltaValue, { color: improved ? COLORS.good : COLORS.critical }]}>
            {after}
          </Text>
          <Text style={styles.deltaLabel}>AFTER LANDED</Text>
        </View>
        <View style={styles.deltaStat}>
          <Text style={[styles.deltaValue, { color: improved ? COLORS.good : COLORS.critical }]}>
            {improved ? `-${delta}` : `+${Math.abs(delta)}`} pts
          </Text>
          <Text style={styles.deltaLabel}>{improved ? 'RISK REDUCED' : 'RISK INCREASED'}</Text>
        </View>
      </View>
      {comparison.attacks.map((attack) => {
        const status = COMPARISON_STATUS[attack.status];
        const baseline = outcomeStyle(attack.baseline_outcome);
        const compare = outcomeStyle(attack.compare_outcome ?? 'error');
        return (
          <View key={attack.attack} style={styles.cmpRow} wrap={false}>
            <View style={styles.cmpAttack}>
              <Text style={styles.cmpAttackName}>{attack.attack}</Text>
            </View>
            <View style={styles.cmpFlow}>
              <Chip label={baseline.label} color={baseline.color} />
              <Arrow />
              <Chip
                label={attack.compare_outcome ? compare.label : '-'}
                color={attack.compare_outcome ? compare.color : COLORS.faint}
              />
            </View>
            <Chip label={status.label} color={status.color} />
          </View>
        );
      })}
    </View>
  );
}

export function ReportDocument({ payload }: { payload: RedteamReportPayload }): JSX.Element {
  const { job, aggregates, findings, comparison } = payload;
  const risk = SEVERITY[aggregates.risk_level];
  const landedFindings = findings.filter((finding) => finding.landed);

  return (
    <Document
      title={`TrustLoopGuard Vulnerability Report — ${job.target}`}
      author="TrustLoopGuard"
    >
      <Page size="A4" style={styles.page}>
        <View style={styles.topRule} />
        <View style={styles.brandRow}>
          <View style={styles.brandLeft}>
            <View style={styles.mark} />
            <Text style={styles.wordmark}>TrustLoopGuard</Text>
          </View>
          <Text style={styles.kicker}>AGENT VULNERABILITY REPORT</Text>
        </View>

        <Text style={styles.title}>What your agent is exposed to</Text>
        <View style={styles.metaRow}>
          <MetaItem label="TARGET" value={job.target} />
          <MetaItem label="PROFILE" value={job.profile} />
          <MetaItem label="GENERATED" value={formatDate(payload.generated_at)} />
        </View>

        {/* Executive summary */}
        <View style={styles.summaryCard}>
          <View style={[styles.riskChip, { backgroundColor: risk.color }]}>
            <Text style={styles.riskWord}>{risk.label}</Text>
            <Text style={styles.riskLabel}>RISK LEVEL</Text>
          </View>
          <View style={styles.summaryBody}>
            <Text style={styles.headlineStat}>
              {aggregates.landed} of {aggregates.attacks} attacks landed
            </Text>
            <Text style={styles.headlineSub}>
              {pct(aggregates.success_rate)} success rate · {aggregates.blocked} blocked ·{' '}
              {aggregates.clean} control{aggregates.clean === 1 ? '' : 's'}
            </Text>
            <SeverityBar
              landed={aggregates.landed}
              blocked={aggregates.blocked}
              clean={aggregates.clean}
              errored={aggregates.errored}
            />
          </View>
        </View>

        {/* Comparison (before/after) */}
        {comparison ? <ComparisonSection comparison={comparison} /> : null}

        {/* Findings */}
        <View style={styles.sectionHead}>
          <Text style={styles.sectionTitle}>Findings</Text>
          <Text style={styles.sectionCount}>
            {landedFindings.length} landed of {findings.length} tested
          </Text>
        </View>
        {findings.map((finding) => (
          <Finding key={finding.seq} finding={finding} />
        ))}

        {/* How TrustLoopGuard helps */}
        <View style={styles.helpBox}>
          <Text style={styles.helpTitle}>How TrustLoopGuard closes these gaps</Text>
          <Text style={styles.helpText}>
            Each landed attack is a real-time guardrail TrustLoopGuard can enforce before a response
            reaches a user — blocking credential and prompt-leak disclosure, rewriting unsafe output,
            or escalating to a human. Run the same attacks through a guarded agent to watch the
            success rate fall to zero.
          </Text>
        </View>

        <View style={styles.footer} fixed>
          <Text style={styles.footerText}>
            Generated by TrustLoopGuard · {formatDate(payload.generated_at)}
          </Text>
          <Text
            style={styles.footerText}
            render={({ pageNumber, totalPages }) => `${pageNumber} / ${totalPages}`}
          />
        </View>
      </Page>
    </Document>
  );
}
