//! Pure per-action evaluation of the `financial` policy family.
//!
//! This module only evaluates fields already present on `FinancialAction` and
//! its matching policy. Stateful checks such as daily/monthly spend windows,
//! mandate lookup, approval records, eligibility evidence, and provider state
//! belong in `tl-server` services.

use tl_core::{FinancialAction, SpendMeter, TriggeredPolicy, Verdict};
use tl_policy::{Action, FamilyPolicy, FinancialPolicy};

use crate::event_policy::EventPolicyOutcome;

pub fn evaluate_financial_policies<'a, I>(
    action: &FinancialAction,
    families: I,
) -> EventPolicyOutcome
where
    I: IntoIterator<Item = &'a FamilyPolicy>,
{
    let mut outcome = EventPolicyOutcome::empty();
    for family in families {
        let FamilyPolicy::Financial(financial) = family else {
            continue;
        };
        if !financial_matches(financial, action) {
            continue;
        }
        for (verdict, reason) in per_action_verdicts(financial, action) {
            compose(&mut outcome, financial, verdict, reason);
        }
    }
    outcome
}

pub fn financial_matches(financial: &FinancialPolicy, action: &FinancialAction) -> bool {
    // Meter isolation: only `actions`-meter policies ever match a typed
    // financial action. `llm_usage` budgets are evaluated exclusively by
    // the gateway budget hook against `llm_usage_events` — even a `when`
    // that would otherwise match must not touch money-action
    // authorization.
    if financial.meter != SpendMeter::Actions {
        return false;
    }
    let when = &financial.when;
    if !when.agents.is_empty()
        && !when
            .agents
            .iter()
            .any(|agent| agent == &action.principal_id)
    {
        return false;
    }
    if !when.action_kinds.is_empty() && !when.action_kinds.iter().any(|kind| kind == &action.kind) {
        return false;
    }
    if !when.currencies.is_empty()
        && !when
            .currencies
            .iter()
            .any(|currency| currency == &action.amount.currency)
    {
        return false;
    }
    if !when.rails.is_empty() && !when.rails.iter().any(|rail| rail == &action.rail) {
        return false;
    }
    if !when.operations.is_empty() && !when.operations.iter().any(|op| op == &action.operation) {
        return false;
    }
    true
}

fn per_action_verdicts(
    financial: &FinancialPolicy,
    action: &FinancialAction,
) -> Vec<(Verdict, String)> {
    let mut verdicts = Vec::new();
    let amount = action.amount.amount_minor;

    if amount <= 0 {
        verdicts.push((
            Verdict::Block,
            format!(
                "financial policy `{}`: non-positive amount {amount} — blocked",
                financial.id
            ),
        ));
        return verdicts;
    }

    if let Some(counterparty) = &action.counterparty {
        if financial
            .denied_counterparty_ids
            .iter()
            .any(|id| id == &counterparty.id)
        {
            verdicts.push((
                Verdict::Block,
                format!(
                    "financial policy `{}`: denied counterparty `{}`",
                    financial.id, counterparty.id
                ),
            ));
        }
        if !financial.allowed_counterparty_ids.is_empty()
            && !financial
                .allowed_counterparty_ids
                .iter()
                .any(|id| id == &counterparty.id)
        {
            verdicts.push((
                Verdict::Escalate,
                format!(
                    "financial policy `{}`: counterparty `{}` is not allowed",
                    financial.id, counterparty.id
                ),
            ));
        }
        if financial.hold_new_counterparty
            && !financial
                .allowed_counterparty_ids
                .iter()
                .any(|id| id == &counterparty.id)
            && !financial
                .denied_counterparty_ids
                .iter()
                .any(|id| id == &counterparty.id)
        {
            verdicts.push((
                Verdict::Escalate,
                format!(
                    "financial policy `{}`: new counterparty `{}` requires approval",
                    financial.id, counterparty.id
                ),
            ));
        }
    } else if financial.hold_new_counterparty || !financial.allowed_counterparty_ids.is_empty() {
        verdicts.push((
            Verdict::Escalate,
            format!(
                "financial policy `{}`: missing counterparty requires approval",
                financial.id
            ),
        ));
    }

    if financial.mandate_required && action.mandate.is_none() {
        verdicts.push((
            Verdict::Escalate,
            format!(
                "financial policy `{}`: mandate required before execution",
                financial.id
            ),
        ));
    }

    if let Some(cap) = financial.per_transaction_minor {
        if amount > cap {
            verdicts.push((
                action_verdict(financial.on_breach),
                format!(
                    "financial policy `{}`: amount {amount} over per-transaction cap {cap}",
                    financial.id
                ),
            ));
        }
    }
    if let Some(threshold) = financial.approval_threshold_minor {
        if amount >= threshold {
            verdicts.push((
                Verdict::Escalate,
                format!(
                    "financial policy `{}`: amount {amount} at or above approval threshold {threshold}",
                    financial.id
                ),
            ));
        }
    }
    if let Some(threshold) = financial.hold_above_minor {
        if amount >= threshold {
            verdicts.push((
                Verdict::Escalate,
                format!(
                    "financial policy `{}`: amount {amount} at or above hold threshold {threshold}",
                    financial.id
                ),
            ));
        }
    }

    verdicts
}

/// Windowed financial cap check. The caller supplies ledger-derived spend
/// already counted in each window; this adds the current action amount and
/// reports a breach. Pure — no clock, no I/O.
pub fn financial_windowed_verdict(
    financial: &FinancialPolicy,
    spent_today: i64,
    spent_week: i64,
    spent_month: i64,
    amount: i64,
) -> Option<(Verdict, String)> {
    if let Some(cap) = financial.daily_minor {
        if spent_today.saturating_add(amount) > cap {
            return Some((
                action_verdict(financial.on_breach),
                format!(
                    "financial policy `{}`: daily spend would exceed cap {cap}",
                    financial.id
                ),
            ));
        }
    }
    if let Some(cap) = financial.weekly_minor {
        if spent_week.saturating_add(amount) > cap {
            return Some((
                action_verdict(financial.on_breach),
                format!(
                    "financial policy `{}`: weekly spend would exceed cap {cap}",
                    financial.id
                ),
            ));
        }
    }
    if let Some(cap) = financial.monthly_minor {
        if spent_month.saturating_add(amount) > cap {
            return Some((
                action_verdict(financial.on_breach),
                format!(
                    "financial policy `{}`: monthly spend would exceed cap {cap}",
                    financial.id
                ),
            ));
        }
    }
    None
}

fn action_verdict(action: Action) -> Verdict {
    match action {
        Action::Block => Verdict::Block,
        Action::Escalate => Verdict::Escalate,
        Action::Allow | Action::Rewrite => Verdict::Block,
    }
}

fn compose(
    outcome: &mut EventPolicyOutcome,
    financial: &FinancialPolicy,
    verdict: Verdict,
    reason: String,
) {
    outcome.triggered.push(TriggeredPolicy {
        id: financial.id.clone(),
        severity: financial.severity,
        reason: reason.clone(),
    });
    match outcome.verdict {
        None => {
            outcome.verdict = Some(verdict);
            outcome.reason = Some(reason);
        }
        Some(current) => {
            let worst = current.worst_with(verdict);
            if worst == verdict && verdict != current {
                outcome.reason = Some(reason);
            }
            outcome.verdict = Some(worst);
        }
    }
}
