use tl_core::{
    HumanReviewGroupRow, HumanReviewOutcome, HumanReviewOutcomeCounts, HumanReviewPolicyRow,
    HumanReviewReasonRow,
};

use super::validation::non_empty_string;

#[derive(Default)]
pub(super) struct WorkflowAccumulator {
    pub(super) human_review_count: i64,
    pub(super) corrected_count: i64,
    pub(super) rejected_count: i64,
    pub(super) false_positive_count: i64,
}

#[derive(Default)]
pub(super) struct PolicyAccumulator {
    pub(super) escalation_count: i64,
    pub(super) corrected_count: i64,
    pub(super) false_positive_count: i64,
}

#[derive(Default)]
pub(super) struct GroupAccumulator {
    pub(super) human_review_count: i64,
    pub(super) human_intervention_count: i64,
}

pub(super) fn is_human_intervention(outcome: HumanReviewOutcome) -> bool {
    matches!(
        outcome,
        HumanReviewOutcome::Corrected
            | HumanReviewOutcome::Rejected
            | HumanReviewOutcome::MissedIssue
    )
}

pub(super) fn count_outcome(counts: &mut HumanReviewOutcomeCounts, outcome: HumanReviewOutcome) {
    match outcome {
        HumanReviewOutcome::Accepted => counts.accepted_count += 1,
        HumanReviewOutcome::Corrected => counts.corrected_count += 1,
        HumanReviewOutcome::Rejected => counts.rejected_count += 1,
        HumanReviewOutcome::FalsePositive => counts.false_positive_count += 1,
        HumanReviewOutcome::MissedIssue => counts.missed_issue_count += 1,
        HumanReviewOutcome::Ignored => counts.ignored_count += 1,
    }
}

pub(super) fn percentage(numerator: i64, denominator: i64) -> f64 {
    if denominator == 0 {
        0.0
    } else {
        ((numerator as f64 / denominator as f64) * 10_000.0).round() / 100.0
    }
}

pub(super) fn payload_string(payload: &serde_json::Value, key: &str) -> Option<String> {
    payload
        .get(key)
        .and_then(serde_json::Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(str::to_string)
}

pub(super) fn policy_ids(payload: &serde_json::Value) -> Vec<String> {
    let ids = payload
        .get("triggered_policies")
        .and_then(serde_json::Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(|policy| payload_string(policy, "id"))
        .collect::<Vec<_>>();
    if ids.is_empty() {
        vec!["baseline".to_string()]
    } else {
        ids
    }
}

pub(super) fn workflow_step(
    event_kind: &str,
    event_label: Option<&str>,
    metadata: &serde_json::Value,
) -> Option<String> {
    metadata
        .get("workflow_step")
        .and_then(serde_json::Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(str::to_string)
        .or_else(|| {
            event_label
                .map(str::trim)
                .filter(|value| !value.is_empty())
                .map(str::to_string)
        })
        .or_else(|| non_empty_string(event_kind))
}

pub(super) fn sort_named_rows<T, F>(mut rows: Vec<T>, name: F) -> Vec<T>
where
    F: Fn(&T) -> &str,
{
    rows.sort_by(|a, b| name(a).cmp(name(b)));
    rows
}

pub(super) fn sort_reason_rows(mut rows: Vec<HumanReviewReasonRow>) -> Vec<HumanReviewReasonRow> {
    rows.sort_by(|a, b| {
        b.count
            .cmp(&a.count)
            .then_with(|| a.reason_code.cmp(&b.reason_code))
    });
    rows
}

pub(super) fn sort_policy_rows(mut rows: Vec<HumanReviewPolicyRow>) -> Vec<HumanReviewPolicyRow> {
    rows.sort_by(|a, b| {
        let b_total = b.escalation_count + b.corrected_count + b.false_positive_count;
        let a_total = a.escalation_count + a.corrected_count + a.false_positive_count;
        b_total
            .cmp(&a_total)
            .then_with(|| a.policy_id.cmp(&b.policy_id))
    });
    rows
}

pub(super) fn group_row(group: String, counts: GroupAccumulator) -> HumanReviewGroupRow {
    HumanReviewGroupRow {
        group,
        human_review_count: counts.human_review_count,
        human_intervention_count: counts.human_intervention_count,
    }
}
