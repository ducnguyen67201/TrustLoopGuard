//! Pure per-action evaluation of the `financial` policy family.
//!
//! This module only evaluates fields already present on `FinancialAction` and
//! its matching policy. Stateful checks such as daily/monthly spend windows,
//! grant matching, approval records, eligibility evidence, and provider state
//! belong in `tl-server` services.

use tl_core::{AuthorizationEffect, FinancialAction, SpendMeter, TriggeredPolicy};
use tl_policy::{FamilyPolicy, FinancialPolicy};

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
        for (effect, reason) in per_action_effects(financial, action) {
            compose(&mut outcome, financial, effect, reason);
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

fn per_action_effects(
    financial: &FinancialPolicy,
    action: &FinancialAction,
) -> Vec<(AuthorizationEffect, String)> {
    let mut effects = Vec::new();
    let amount = action.amount.amount_minor;

    if amount <= 0 {
        effects.push((
            AuthorizationEffect::Deny,
            format!(
                "financial policy `{}`: non-positive amount {amount} — blocked",
                financial.id
            ),
        ));
        return effects;
    }

    if let Some(counterparty) = &action.counterparty {
        if financial
            .denied_counterparty_ids
            .iter()
            .any(|id| id == &counterparty.id)
        {
            effects.push((
                AuthorizationEffect::Deny,
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
            effects.push((
                AuthorizationEffect::RequireApproval,
                format!(
                    "financial policy `{}`: counterparty `{}` is not allowed",
                    financial.id, counterparty.id
                ),
            ));
        }
        if financial.require_approval_for_new_counterparty
            && !financial
                .allowed_counterparty_ids
                .iter()
                .any(|id| id == &counterparty.id)
            && !financial
                .denied_counterparty_ids
                .iter()
                .any(|id| id == &counterparty.id)
        {
            effects.push((
                AuthorizationEffect::RequireApproval,
                format!(
                    "financial policy `{}`: new counterparty `{}` requires approval",
                    financial.id, counterparty.id
                ),
            ));
        }
    } else if financial.require_approval_for_new_counterparty
        || !financial.allowed_counterparty_ids.is_empty()
    {
        effects.push((
            AuthorizationEffect::RequireApproval,
            format!(
                "financial policy `{}`: missing counterparty requires approval",
                financial.id
            ),
        ));
    }

    if financial.grant_required {
        effects.push((
            AuthorizationEffect::RequireApproval,
            format!(
                "financial policy `{}`: delegated authority required before execution",
                financial.id
            ),
        ));
    }

    if let Some(cap) = financial.per_transaction_minor {
        if amount > cap {
            effects.push((
                action_effect(financial.on_breach),
                format!(
                    "financial policy `{}`: amount {amount} over per-transaction cap {cap}",
                    financial.id
                ),
            ));
        }
    }
    if let Some(threshold) = financial.approval_threshold_minor {
        if amount >= threshold {
            effects.push((
                AuthorizationEffect::RequireApproval,
                format!(
                    "financial policy `{}`: amount {amount} at or above approval threshold {threshold}",
                    financial.id
                ),
            ));
        }
    }
    effects
}

/// Windowed financial cap check. The caller supplies ledger-derived spend
/// already counted in each window; this adds the current action amount and
/// reports a breach. Pure — no clock, no I/O.
pub fn financial_windowed_effect(
    financial: &FinancialPolicy,
    spent_today: i64,
    spent_week: i64,
    spent_month: i64,
    amount: i64,
) -> Option<(AuthorizationEffect, String)> {
    if let Some(cap) = financial.daily_minor {
        if spent_today.saturating_add(amount) > cap {
            return Some((
                action_effect(financial.on_breach),
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
                action_effect(financial.on_breach),
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
                action_effect(financial.on_breach),
                format!(
                    "financial policy `{}`: monthly spend would exceed cap {cap}",
                    financial.id
                ),
            ));
        }
    }
    None
}

fn action_effect(action: AuthorizationEffect) -> AuthorizationEffect {
    match action {
        AuthorizationEffect::Deny => AuthorizationEffect::Deny,
        AuthorizationEffect::RequireApproval => AuthorizationEffect::RequireApproval,
        AuthorizationEffect::Defer => AuthorizationEffect::Defer,
        AuthorizationEffect::Permit | AuthorizationEffect::Transform => AuthorizationEffect::Deny,
    }
}

fn compose(
    outcome: &mut EventPolicyOutcome,
    financial: &FinancialPolicy,
    effect: AuthorizationEffect,
    reason: String,
) {
    outcome.triggered.push(TriggeredPolicy {
        id: financial.id.clone(),
        severity: financial.severity,
        reason: reason.clone(),
    });
    match outcome.effect {
        None => {
            outcome.effect = Some(effect);
            outcome.reason = Some(reason);
        }
        Some(current) => {
            let worst = current.worst_with(effect);
            if worst == effect && effect != current {
                outcome.reason = Some(reason);
            }
            outcome.effect = Some(worst);
        }
    }
}
