//! Per-call evaluation of the `payment` policy family.
//!
//! Caps that need only the event in hand — the per-transaction cap and the
//! hold band — are evaluated here and composed into the decision exactly like
//! content policies. Windowed (daily/monthly) caps need spend history and are
//! enforced by a stateful stage in the server, not here.
//!
//! Conservative money posture (mirrors the `value_limit` checker): a matched
//! payment operation whose `amount` is missing or non-integer escalates rather
//! than passing — an unverifiable money value is never treated as clean.

use tl_core::{GuardEvent, TriggeredPolicy, Verdict};
use tl_policy::{Action, FamilyPolicy, PaymentPolicy};

use crate::event_policy::EventPolicyOutcome;

/// Evaluate per-call payment caps for every enabled payment-family policy that
/// matches this event, returning the composed worst-verdict outcome.
pub fn evaluate_payment_policies<'a, I>(event: &GuardEvent, families: I) -> EventPolicyOutcome
where
    I: IntoIterator<Item = &'a FamilyPolicy>,
{
    let mut outcome = EventPolicyOutcome::empty();
    for family in families {
        let FamilyPolicy::Payment(payment) = family else {
            continue;
        };
        if !payment_matches(payment, event) {
            continue;
        }
        if let Some((verdict, reason)) = per_call_verdict(payment, payment_amount(event)) {
            compose(&mut outcome, payment, verdict, reason);
        }
    }
    outcome
}

/// The `amount` parameter of an event as minor units, if present and integer.
pub fn payment_amount(event: &GuardEvent) -> Option<i64> {
    event
        .action
        .parameters
        .get("amount")
        .and_then(serde_json::Value::as_i64)
}

/// A payment policy applies when the operation matches and the principal is in
/// scope. `operations` is validated non-empty, so an unmatched operation fails
/// closed (no cap from this policy). Empty `agents` means all owners.
pub fn payment_matches(payment: &PaymentPolicy, event: &GuardEvent) -> bool {
    if !payment
        .when
        .operations
        .iter()
        .any(|op| op == &event.action.operation)
    {
        return false;
    }
    if !payment.when.agents.is_empty()
        && !payment
            .when
            .agents
            .iter()
            .any(|agent| agent == &event.principal.agent_id)
    {
        return false;
    }
    true
}

/// `None` = within the per-call caps (windowed caps decided elsewhere).
fn per_call_verdict(payment: &PaymentPolicy, amount: Option<i64>) -> Option<(Verdict, String)> {
    let Some(amount) = amount else {
        return Some((
            Verdict::Escalate,
            format!(
                "payment `{}`: amount missing or non-integer — escalating (conservative)",
                payment.id
            ),
        ));
    };
    // A cap only bounds spend from above; a non-positive amount slips under
    // every `amount > cap` / `amount >= threshold` check and would otherwise
    // pass as clean. A payment is always positive minor units, so treat
    // amount <= 0 as malformed and block outright (a negative "charge" is a
    // provider-side credit an attacker could direct to themselves).
    if amount <= 0 {
        return Some((
            Verdict::Block,
            format!(
                "payment `{}`: non-positive amount {amount} — blocked",
                payment.id
            ),
        ));
    }
    if let Some(cap) = payment.per_transaction_minor {
        if amount > cap {
            return Some((
                action_verdict(payment.on_breach),
                format!(
                    "payment `{}`: amount {amount} over per-transaction cap {cap}",
                    payment.id
                ),
            ));
        }
    }
    if let Some(threshold) = payment.hold_above_minor {
        if amount >= threshold {
            return Some((
                Verdict::Escalate,
                format!(
                    "payment `{}`: amount {amount} at or above hold threshold {threshold}",
                    payment.id
                ),
            ));
        }
    }
    None
}

/// Windowed (daily/monthly) cap check. The caller supplies the spend already
/// counted in each window; this adds the current `amount` and reports a breach.
/// `None` = within the windowed caps. Pure — no clock, no I/O.
pub fn windowed_verdict(
    payment: &PaymentPolicy,
    spent_today: i64,
    spent_month: i64,
    amount: i64,
) -> Option<(Verdict, String)> {
    if let Some(cap) = payment.daily_minor {
        if spent_today.saturating_add(amount) > cap {
            return Some((
                action_verdict(payment.on_breach),
                format!(
                    "payment `{}`: daily spend would exceed cap {cap}",
                    payment.id
                ),
            ));
        }
    }
    if let Some(cap) = payment.monthly_minor {
        if spent_month.saturating_add(amount) > cap {
            return Some((
                action_verdict(payment.on_breach),
                format!(
                    "payment `{}`: monthly spend would exceed cap {cap}",
                    payment.id
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
        // `on_breach` is validated to be enforcing; default to Block defensively.
        Action::Allow | Action::Rewrite => Verdict::Block,
    }
}

fn compose(
    outcome: &mut EventPolicyOutcome,
    payment: &PaymentPolicy,
    verdict: Verdict,
    reason: String,
) {
    outcome.triggered.push(TriggeredPolicy {
        id: payment.id.clone(),
        severity: payment.severity,
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
                outcome.reason = Some(reason); // strictly worse → adopt its reason
            }
            outcome.verdict = Some(worst);
        }
    }
}

#[cfg(test)]
mod tests {
    use serde_json::json;
    use tl_core::{Action as EventAction, EventKind, Principal};
    use tl_policy::{PaymentPolicy, PaymentWhen};

    use super::*;

    fn event(owner: &str, operation: &str, params: serde_json::Value) -> GuardEvent {
        GuardEvent {
            kind: EventKind::ToolCallProposed,
            principal: Principal {
                workspace_id: "ws".into(),
                environment_id: "production".into(),
                agent_id: owner.into(),
                user_id: None,
                session_id: None,
                task_id: None,
                run_id: None,
                run_event_id: None,
            },
            action: EventAction {
                operation: operation.into(),
                parameters: params,
                side_effect: None,
            },
            sources: vec![],
            provenance: Default::default(),
            resolution: None,
            label_resolution: None,
            checks: vec![],
            signals: vec![],
            context: serde_json::Value::Null,
        }
    }

    fn policy() -> FamilyPolicy {
        FamilyPolicy::Payment(PaymentPolicy {
            id: "alice-caps".into(),
            description: None,
            severity: tl_core::Severity::High,
            when: PaymentWhen {
                agents: vec!["alice".into()],
                operations: vec!["pay".into()],
            },
            per_transaction_minor: Some(10_000),
            hold_above_minor: Some(5_000),
            daily_minor: None,
            monthly_minor: None,
            on_breach: Action::Block,
        })
    }

    fn run(owner: &str, op: &str, params: serde_json::Value) -> Option<Verdict> {
        let p = [policy()];
        evaluate_payment_policies(&event(owner, op, params), p.iter()).verdict
    }

    #[test]
    fn within_caps_no_verdict() {
        assert_eq!(run("alice", "pay", json!({ "amount": 4_000 })), None);
    }

    #[test]
    fn over_per_transaction_blocks() {
        assert_eq!(
            run("alice", "pay", json!({ "amount": 80_000 })),
            Some(Verdict::Block)
        );
    }

    #[test]
    fn hold_band_escalates() {
        assert_eq!(
            run("alice", "pay", json!({ "amount": 6_000 })),
            Some(Verdict::Escalate)
        );
    }

    #[test]
    fn non_positive_amount_blocks() {
        // A negative amount must not slip under the `amount > cap` checks.
        assert_eq!(
            run("alice", "pay", json!({ "amount": -999_999 })),
            Some(Verdict::Block)
        );
        assert_eq!(
            run("alice", "pay", json!({ "amount": 0 })),
            Some(Verdict::Block)
        );
    }

    #[test]
    fn missing_amount_escalates() {
        assert_eq!(run("alice", "pay", json!({})), Some(Verdict::Escalate));
        assert_eq!(
            run("alice", "pay", json!({ "amount": "oops" })),
            Some(Verdict::Escalate)
        );
    }

    #[test]
    fn other_owner_not_in_scope() {
        assert_eq!(run("bob", "pay", json!({ "amount": 80_000 })), None);
    }

    #[test]
    fn other_operation_not_in_scope() {
        assert_eq!(run("alice", "refund", json!({ "amount": 80_000 })), None);
    }

    fn windowed_policy() -> PaymentPolicy {
        PaymentPolicy {
            id: "caps".into(),
            description: None,
            severity: tl_core::Severity::High,
            when: PaymentWhen {
                agents: vec![],
                operations: vec!["pay".into()],
            },
            per_transaction_minor: None,
            hold_above_minor: None,
            daily_minor: Some(10_000),
            monthly_minor: Some(50_000),
            on_breach: Action::Block,
        }
    }

    #[test]
    fn daily_window_blocks_over_inclusive_cap() {
        let p = windowed_policy();
        // 8_000 spent today + 3_000 = 11_000 > 10_000 → block.
        assert!(windowed_verdict(&p, 8_000, 0, 3_000).is_some());
        // + 2_000 = 10_000, exactly the cap → allowed (inclusive).
        assert!(windowed_verdict(&p, 8_000, 0, 2_000).is_none());
    }

    #[test]
    fn monthly_window_blocks() {
        let p = windowed_policy();
        assert!(windowed_verdict(&p, 0, 49_000, 2_000).is_some());
        assert!(windowed_verdict(&p, 0, 40_000, 2_000).is_none());
    }
}
