use std::collections::BTreeSet;

use tl_core::{AnalyticsChartType, AnalyticsDimension, AnalyticsFilter, AnalyticsMetric};

use crate::StorageError;

use super::facts::AnalyticsFact;

#[derive(Default)]
struct MetricAccumulator {
    traces: i64,
    permit: i64,
    deny: i64,
    transform: i64,
    require_approval: i64,
    defer: i64,
    human_reviews: i64,
    human_interventions: i64,
    false_positives: i64,
    latencies: Vec<i32>,
}

pub(super) fn validate_query(request: &tl_core::AnalyticsQueryRequest) -> Result<(), StorageError> {
    for filter in &request.filters {
        if filter.values.iter().any(|value| value.trim().is_empty()) {
            return Err(StorageError::Internal(
                "analytics filters must not contain empty values".into(),
            ));
        }
    }
    Ok(())
}

pub(super) fn matches_filters(fact: &AnalyticsFact, filters: &[AnalyticsFilter]) -> bool {
    filters.iter().all(|filter| {
        let wanted = filter.values.iter().collect::<BTreeSet<_>>();
        values_for_dimension(fact, filter.dimension)
            .iter()
            .any(|value| wanted.contains(value))
    })
}

pub(super) fn metric_value<'a>(
    metric: AnalyticsMetric,
    facts: impl Iterator<Item = &'a AnalyticsFact>,
) -> f64 {
    let acc = facts.fold(MetricAccumulator::default(), |mut acc, fact| {
        acc.traces += 1;
        acc.latencies.push(fact.elapsed_ms);
        match fact.decision.as_str() {
            "permit" => acc.permit += 1,
            "deny" => acc.deny += 1,
            "transform" => acc.transform += 1,
            "require_approval" => acc.require_approval += 1,
            "defer" => acc.defer += 1,
            _ => {}
        }
        if fact.review_outcome != "not_reviewed" {
            acc.human_reviews += 1;
        }
        if matches!(
            fact.review_outcome.as_str(),
            "corrected" | "rejected" | "missed_issue"
        ) {
            acc.human_interventions += 1;
        }
        if fact.review_outcome == "false_positive" {
            acc.false_positives += 1;
        }
        acc
    });
    match metric {
        AnalyticsMetric::TraceCount => acc.traces as f64,
        AnalyticsMetric::PermitCount => acc.permit as f64,
        AnalyticsMetric::DenyCount => acc.deny as f64,
        AnalyticsMetric::TransformCount => acc.transform as f64,
        AnalyticsMetric::RequireApprovalCount => acc.require_approval as f64,
        AnalyticsMetric::DeferCount => acc.defer as f64,
        AnalyticsMetric::InterventionRate => percentage(
            acc.deny + acc.transform + acc.require_approval + acc.defer,
            acc.traces,
        ),
        AnalyticsMetric::P95LatencyMs => p95(acc.latencies).unwrap_or_default() as f64,
        AnalyticsMetric::HumanReviewCount => acc.human_reviews as f64,
        AnalyticsMetric::HumanInterventionRate => percentage(acc.human_interventions, acc.traces),
        AnalyticsMetric::FalsePositiveRate => percentage(acc.false_positives, acc.traces),
    }
}

pub(super) fn values_for_dimension(
    fact: &AnalyticsFact,
    dimension: AnalyticsDimension,
) -> Vec<String> {
    match dimension {
        AnalyticsDimension::AgentId => vec![fact.agent_id.clone()],
        AnalyticsDimension::Environment => vec![fact.environment_id.clone()],
        AnalyticsDimension::RunKind => vec![fact.run_kind.clone()],
        AnalyticsDimension::RunStatus => vec![fact.run_status.clone()],
        AnalyticsDimension::AuthorizationEffect => vec![fact.decision.clone()],
        AnalyticsDimension::PolicyId => fact.policy_ids.clone(),
        AnalyticsDimension::WorkflowStep => vec![fact.workflow_step.clone()],
        AnalyticsDimension::ReviewOutcome => vec![fact.review_outcome.clone()],
        AnalyticsDimension::ExternalId => vec![fact.external_id.clone()],
    }
}

pub(super) fn fact_values(
    facts: &[AnalyticsFact],
    dimension: AnalyticsDimension,
) -> BTreeSet<String> {
    facts
        .iter()
        .flat_map(|fact| values_for_dimension(fact, dimension))
        .collect()
}

pub(super) fn supported_metrics() -> Vec<AnalyticsMetric> {
    vec![
        AnalyticsMetric::TraceCount,
        AnalyticsMetric::PermitCount,
        AnalyticsMetric::DenyCount,
        AnalyticsMetric::TransformCount,
        AnalyticsMetric::RequireApprovalCount,
        AnalyticsMetric::DeferCount,
        AnalyticsMetric::InterventionRate,
        AnalyticsMetric::P95LatencyMs,
        AnalyticsMetric::HumanReviewCount,
        AnalyticsMetric::HumanInterventionRate,
        AnalyticsMetric::FalsePositiveRate,
    ]
}

pub(super) fn supported_dimensions() -> Vec<AnalyticsDimension> {
    vec![
        AnalyticsDimension::AgentId,
        AnalyticsDimension::Environment,
        AnalyticsDimension::RunKind,
        AnalyticsDimension::RunStatus,
        AnalyticsDimension::AuthorizationEffect,
        AnalyticsDimension::PolicyId,
        AnalyticsDimension::WorkflowStep,
        AnalyticsDimension::ReviewOutcome,
        AnalyticsDimension::ExternalId,
    ]
}

pub(super) fn metric_label(metric: AnalyticsMetric) -> &'static str {
    match metric {
        AnalyticsMetric::TraceCount => "Traces",
        AnalyticsMetric::PermitCount => "Permitted",
        AnalyticsMetric::DenyCount => "Denied",
        AnalyticsMetric::TransformCount => "Transformed",
        AnalyticsMetric::RequireApprovalCount => "Approval required",
        AnalyticsMetric::DeferCount => "Deferred",
        AnalyticsMetric::InterventionRate => "Intervention rate",
        AnalyticsMetric::P95LatencyMs => "p95 latency",
        AnalyticsMetric::HumanReviewCount => "Human reviews",
        AnalyticsMetric::HumanInterventionRate => "Human intervention rate",
        AnalyticsMetric::FalsePositiveRate => "False positive rate",
    }
}

pub(super) fn dimension_label(dimension: AnalyticsDimension) -> &'static str {
    match dimension {
        AnalyticsDimension::AgentId => "Agent",
        AnalyticsDimension::Environment => "Environment",
        AnalyticsDimension::RunKind => "Run kind",
        AnalyticsDimension::RunStatus => "Run status",
        AnalyticsDimension::AuthorizationEffect => "Authorization effect",
        AnalyticsDimension::PolicyId => "Policy",
        AnalyticsDimension::WorkflowStep => "Workflow step",
        AnalyticsDimension::ReviewOutcome => "Review outcome",
        AnalyticsDimension::ExternalId => "External id",
    }
}

pub(super) fn default_chart_type(metric: AnalyticsMetric) -> AnalyticsChartType {
    match metric {
        AnalyticsMetric::TraceCount
        | AnalyticsMetric::PermitCount
        | AnalyticsMetric::DenyCount
        | AnalyticsMetric::TransformCount
        | AnalyticsMetric::RequireApprovalCount
        | AnalyticsMetric::DeferCount
        | AnalyticsMetric::HumanReviewCount => AnalyticsChartType::Bar,
        AnalyticsMetric::InterventionRate
        | AnalyticsMetric::P95LatencyMs
        | AnalyticsMetric::HumanInterventionRate
        | AnalyticsMetric::FalsePositiveRate => AnalyticsChartType::Line,
    }
}

fn percentage(numerator: i64, denominator: i64) -> f64 {
    if denominator == 0 {
        0.0
    } else {
        ((numerator as f64 / denominator as f64) * 10_000.0).round() / 100.0
    }
}

fn p95(mut latencies: Vec<i32>) -> Option<i32> {
    if latencies.is_empty() {
        return None;
    }
    latencies.sort_unstable();
    let index = ((latencies.len() as f64) * 0.95).ceil() as usize;
    latencies.get(index.saturating_sub(1)).copied()
}
