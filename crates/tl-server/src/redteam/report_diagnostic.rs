use async_trait::async_trait;
use serde::Deserialize;
use tl_core::{
    RedteamAttackSession, RedteamReportFinding, RedteamReportPayload, RedteamTrajectoryDiagnostic,
};
use tl_llm::{prompts::trajectory_diagnostic, JudgeKind, LlmRouter};

const MAX_EVENT_LINES: usize = 30;
const MAX_TEXT_CHARS: usize = 360;
const MAX_SUMMARY_CHARS: usize = 480;

pub struct TrajectoryDiagnosticInput {
    pub tenant: String,
    pub finding: RedteamReportFinding,
    pub session: RedteamAttackSession,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TrajectoryDiagnosticError {
    Disabled,
    Provider(String),
    Invalid(String),
}

#[async_trait]
pub trait TrajectoryDiagnoser: Send + Sync {
    fn is_enabled(&self) -> bool;

    async fn diagnose(
        &self,
        input: TrajectoryDiagnosticInput,
    ) -> Result<RedteamTrajectoryDiagnostic, TrajectoryDiagnosticError>;
}

#[async_trait]
impl TrajectoryDiagnoser for LlmRouter {
    fn is_enabled(&self) -> bool {
        self.has_route(JudgeKind::TrajectoryDiagnostic)
    }

    async fn diagnose(
        &self,
        input: TrajectoryDiagnosticInput,
    ) -> Result<RedteamTrajectoryDiagnostic, TrajectoryDiagnosticError> {
        if !self.is_enabled() {
            return Err(TrajectoryDiagnosticError::Disabled);
        }
        let Some(baseline) = input.finding.diagnostic.clone() else {
            return Err(TrajectoryDiagnosticError::Invalid(
                "missing deterministic diagnostic baseline".into(),
            ));
        };

        let baseline_json = serde_json::to_string_pretty(&baseline)
            .unwrap_or_else(|_| "{\"summary\":\"unavailable\"}".into());
        let prompt = trajectory_diagnostic::build(
            &finding_context(&input.finding),
            &baseline_json,
            &trajectory_events(&input.session),
        );
        let output = self
            .judge(
                JudgeKind::TrajectoryDiagnostic,
                &input.tenant,
                &prompt,
                &trajectory_diagnostic::schema(),
            )
            .await
            .map_err(|error| TrajectoryDiagnosticError::Provider(error.to_string()))?;
        let decoded: TrajectoryDiagnosticOutput = serde_json::from_value(output.json)
            .map_err(|error| TrajectoryDiagnosticError::Invalid(error.to_string()))?;
        diagnostic_from_output(decoded, baseline)
    }
}

pub(crate) async fn enrich_report_diagnostics<D: TrajectoryDiagnoser + ?Sized>(
    diagnoser: &D,
    tenant: &str,
    report: &mut RedteamReportPayload,
    sessions: &[RedteamAttackSession],
) {
    if !diagnoser.is_enabled() {
        return;
    }

    for (finding, session) in report.findings.iter_mut().zip(sessions.iter()) {
        if finding.diagnostic.is_none() {
            continue;
        }
        let input = TrajectoryDiagnosticInput {
            tenant: tenant.to_string(),
            finding: finding.clone(),
            session: session.clone(),
        };
        match diagnoser.diagnose(input).await {
            Ok(diagnostic) => finding.diagnostic = Some(diagnostic),
            Err(error) => {
                tracing::warn!(
                    seq = finding.seq,
                    error = ?error,
                    "redteam report diagnostic LLM enrichment failed; using deterministic diagnostic"
                );
            }
        }
    }
}

#[derive(Debug, Deserialize)]
struct TrajectoryDiagnosticOutput {
    summary: String,
    #[serde(default)]
    risk_source: Option<String>,
    #[serde(default)]
    failure_mode: Option<String>,
    #[serde(default)]
    harm_class: Option<String>,
    #[serde(default)]
    suggested_substrate: Option<String>,
    #[serde(default)]
    source_chain: Vec<String>,
    confidence: f64,
}

fn diagnostic_from_output(
    output: TrajectoryDiagnosticOutput,
    mut baseline: RedteamTrajectoryDiagnostic,
) -> Result<RedteamTrajectoryDiagnostic, TrajectoryDiagnosticError> {
    baseline.summary = clean_required(output.summary, "summary")?;
    baseline.source = Some("llm".into());
    baseline.risk_source = clean_optional(output.risk_source).or(baseline.risk_source);
    baseline.failure_mode = clean_optional(output.failure_mode).or(baseline.failure_mode);
    baseline.harm_class = clean_optional(output.harm_class).or(baseline.harm_class);
    baseline.suggested_substrate =
        clean_optional(output.suggested_substrate).or(baseline.suggested_substrate);
    let source_chain = clean_vec(output.source_chain);
    if !source_chain.is_empty() {
        baseline.source_chain = source_chain;
    }
    baseline.confidence = clean_confidence(output.confidence, baseline.confidence);
    Ok(baseline)
}

fn finding_context(finding: &RedteamReportFinding) -> String {
    [
        format!("seq: {}", finding.seq),
        format!("attack: {}", finding.attack),
        format!("goal: {}", finding.goal),
        format!("category: {}", finding.category),
        format!("severity: {:?}", finding.severity),
        format!("outcome: {}", finding.outcome),
        format!("landed: {}", finding.landed),
        format!(
            "evidence_excerpt: {}",
            finding.evidence.as_deref().unwrap_or("(none)")
        ),
        format!("prompt: {}", finding.prompt.as_deref().unwrap_or("(none)")),
        format!(
            "trace_id: {}",
            finding.trace_id.as_deref().unwrap_or("(none)")
        ),
    ]
    .join("\n")
}

fn trajectory_events(session: &RedteamAttackSession) -> String {
    if session.events.is_empty() {
        return "(none)".into();
    }
    session
        .events
        .iter()
        .take(MAX_EVENT_LINES)
        .map(|event| {
            let mut lines = vec![
                format!(
                    "- event_id={} seq={} kind={} actor={}",
                    event.event_id, event.seq, event.kind, event.actor
                ),
                format!(
                    "  label={} trace_id={}",
                    event.label.as_deref().unwrap_or("(none)"),
                    event.trace_id.as_deref().unwrap_or("(none)")
                ),
            ];
            if let Some(text) = event.content_text.as_deref().and_then(non_empty_str) {
                lines.push(format!("  text={}", truncate_chars(text, MAX_TEXT_CHARS)));
            }
            if let Some(guard_event) = event.guard_event.as_ref() {
                lines.push(format!(
                    "  guard_event kind={:?} agent={} operation={} side_effect={:?} sources={} checks={}",
                    guard_event.kind,
                    guard_event.principal.agent_id,
                    guard_event.action.operation,
                    guard_event.action.side_effect,
                    guard_event.sources.len(),
                    guard_event.checks.len()
                ));
            }
            lines.join("\n")
        })
        .collect::<Vec<_>>()
        .join("\n")
}

fn clean_required(value: String, field: &str) -> Result<String, TrajectoryDiagnosticError> {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        return Err(TrajectoryDiagnosticError::Invalid(format!(
            "{field} must not be empty"
        )));
    }
    Ok(truncate_chars(trimmed, MAX_SUMMARY_CHARS))
}

fn clean_optional(value: Option<String>) -> Option<String> {
    value.and_then(|value| non_empty_owned(truncate_chars(value.trim(), MAX_SUMMARY_CHARS)))
}

fn clean_vec(values: Vec<String>) -> Vec<String> {
    values
        .into_iter()
        .filter_map(|value| non_empty_owned(truncate_chars(value.trim(), 160)))
        .take(8)
        .collect()
}

fn clean_confidence(candidate: f64, fallback: f64) -> f64 {
    if candidate.is_finite() {
        candidate.clamp(0.0, 1.0)
    } else {
        fallback
    }
}

fn non_empty_str(value: &str) -> Option<&str> {
    let trimmed = value.trim();
    (!trimmed.is_empty()).then_some(trimmed)
}

fn non_empty_owned(value: String) -> Option<String> {
    (!value.trim().is_empty()).then_some(value)
}

fn truncate_chars(value: &str, max_chars: usize) -> String {
    if value.chars().count() <= max_chars {
        return value.to_string();
    }
    let truncated: String = value.chars().take(max_chars).collect();
    format!("{truncated}...")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn diagnostic_from_output_preserves_evidence_ids_and_falls_back_chain() {
        let baseline = RedteamTrajectoryDiagnostic {
            summary: "deterministic".into(),
            source: Some("deterministic".into()),
            risk_source: Some("checker".into()),
            failure_mode: Some("approval_required".into()),
            harm_class: Some("workflow_integrity".into()),
            evidence_event_ids: vec!["event-1".into()],
            source_chain: vec!["source -> tool".into()],
            suggested_substrate: Some("approval".into()),
            confidence: 0.7,
        };
        let diagnostic = diagnostic_from_output(
            TrajectoryDiagnosticOutput {
                summary: "LLM root cause".into(),
                risk_source: None,
                failure_mode: Some("approval_bypass".into()),
                harm_class: None,
                suggested_substrate: None,
                source_chain: vec![],
                confidence: 1.5,
            },
            baseline,
        )
        .expect("diagnostic converts");

        assert_eq!(diagnostic.summary, "LLM root cause");
        assert_eq!(diagnostic.source.as_deref(), Some("llm"));
        assert_eq!(diagnostic.risk_source.as_deref(), Some("checker"));
        assert_eq!(diagnostic.failure_mode.as_deref(), Some("approval_bypass"));
        assert_eq!(diagnostic.evidence_event_ids, ["event-1"]);
        assert_eq!(diagnostic.source_chain, ["source -> tool"]);
        assert_eq!(diagnostic.confidence, 1.0);
    }
}
