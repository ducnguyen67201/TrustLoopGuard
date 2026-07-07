use serde_json::json;
use tl_core::{
    CounterpartyRef, FinancialAction, FinancialActionKind, FinancialRail, MandateRef, MoneyAmount,
    Verdict,
};
use tl_engine::{evaluate_financial_policies, financial_windowed_verdict};
use tl_policy::{Action, FamilyPolicy, FinancialPolicy, FinancialWhen};

fn action(
    agent_id: &str,
    kind: FinancialActionKind,
    amount_minor: i64,
    counterparty_id: Option<&str>,
    mandate_id: Option<&str>,
) -> FinancialAction {
    FinancialAction {
        id: None,
        kind,
        operation: "issue_refund".into(),
        principal_id: agent_id.into(),
        amount: MoneyAmount {
            amount_minor,
            currency: "USD".into(),
        },
        counterparty: counterparty_id.map(|id| CounterpartyRef {
            id: id.into(),
            display_name: None,
            kind: "customer".into(),
            country: Some("US".into()),
            metadata: serde_json::Value::Null,
        }),
        rail: FinancialRail::PaymentHttp,
        mandate: mandate_id.map(|id| MandateRef {
            id: id.into(),
            version: Some(1),
        }),
        memo: None,
        metadata: json!({}),
    }
}

fn policy() -> FamilyPolicy {
    FamilyPolicy::Financial(FinancialPolicy {
        id: "refund-controls".into(),
        description: None,
        severity: tl_core::Severity::High,
        when: FinancialWhen {
            agents: vec!["refund-bot".into()],
            action_kinds: vec![FinancialActionKind::Refund],
            operations: vec!["issue_refund".into()],
            currencies: vec!["USD".into()],
            rails: vec![FinancialRail::PaymentHttp],
        },
        per_transaction_minor: Some(10_000),
        hold_above_minor: Some(5_000),
        daily_minor: None,
        weekly_minor: None,
        monthly_minor: None,
        allowed_counterparty_ids: vec![],
        denied_counterparty_ids: vec!["blocked_customer".into()],
        hold_new_counterparty: false,
        mandate_required: true,
        approval_threshold_minor: None,
        approver_roles: vec![],
        refund_original_method_only: false,
        required_preconditions: vec![],
        missing_evidence_action: Action::Escalate,
        failed_precondition_action: Action::Block,
        on_breach: Action::Block,
    })
}

#[test]
fn financial_window_caps_use_supplied_ledger_spend() {
    let FamilyPolicy::Financial(policy) = policy() else {
        unreachable!("financial policy")
    };
    let policy = FinancialPolicy {
        daily_minor: Some(5_000),
        approver_roles: vec![],
        ..policy
    };
    let verdict = financial_windowed_verdict(&policy, 4_500, 4_500, 6_000, 750);

    assert_eq!(
        verdict,
        Some((
            tl_core::Verdict::Block,
            "financial policy `refund-controls`: daily spend would exceed cap 5000".into()
        ))
    );
}

#[test]
fn financial_weekly_cap_blocks_when_window_spend_would_exceed() {
    let FamilyPolicy::Financial(policy) = policy() else {
        unreachable!("financial policy")
    };
    let policy = FinancialPolicy {
        weekly_minor: Some(5_000),
        ..policy
    };
    let verdict = financial_windowed_verdict(&policy, 0, 4_500, 4_500, 750);

    assert_eq!(
        verdict,
        Some((
            tl_core::Verdict::Block,
            "financial policy `refund-controls`: weekly spend would exceed cap 5000".into()
        ))
    );
}

#[test]
fn financial_weekly_cap_allows_spend_that_exactly_reaches_cap() {
    let FamilyPolicy::Financial(policy) = policy() else {
        unreachable!("financial policy")
    };
    let policy = FinancialPolicy {
        weekly_minor: Some(5_000),
        ..policy
    };
    let verdict = financial_windowed_verdict(&policy, 0, 4_250, 4_250, 750);

    assert_eq!(verdict, None);
}

#[test]
fn financial_weekly_cap_is_skipped_when_unset() {
    let FamilyPolicy::Financial(policy) = policy() else {
        unreachable!("financial policy")
    };
    let verdict = financial_windowed_verdict(&policy, 0, i64::MAX, 0, 750);

    assert_eq!(verdict, None);
}

#[test]
fn financial_weekly_cap_saturates_instead_of_overflowing() {
    let FamilyPolicy::Financial(policy) = policy() else {
        unreachable!("financial policy")
    };
    let policy = FinancialPolicy {
        weekly_minor: Some(5_000),
        ..policy
    };
    let verdict = financial_windowed_verdict(&policy, 0, i64::MAX, 0, 750);

    assert_eq!(
        verdict,
        Some((
            tl_core::Verdict::Block,
            "financial policy `refund-controls`: weekly spend would exceed cap 5000".into()
        ))
    );
}

#[test]
fn financial_policy_blocks_non_positive_amounts() {
    let outcome = evaluate_financial_policies(
        &action(
            "refund-bot",
            FinancialActionKind::Refund,
            0,
            Some("cust_123"),
            Some("mandate_1"),
        ),
        [&policy()],
    );

    assert_eq!(outcome.verdict, Some(Verdict::Block));
    assert!(outcome.reason.unwrap().contains("non-positive amount"));
}

#[test]
fn financial_policy_holds_above_threshold_before_execution() {
    let outcome = evaluate_financial_policies(
        &action(
            "refund-bot",
            FinancialActionKind::Refund,
            7_500,
            Some("cust_123"),
            Some("mandate_1"),
        ),
        [&policy()],
    );

    assert_eq!(outcome.verdict, Some(Verdict::Escalate));
    assert_eq!(outcome.triggered[0].id, "refund-controls");
}

#[test]
fn financial_policy_uses_first_class_operation_selector() {
    let mut candidate = action(
        "refund-bot",
        FinancialActionKind::Refund,
        7_500,
        Some("cust_123"),
        Some("mandate_1"),
    );
    candidate.operation = "quote_refund".into();
    candidate.metadata = json!({ "operation": "issue_refund" });

    let outcome = evaluate_financial_policies(&candidate, [&policy()]);

    assert_eq!(outcome.verdict, None);
    assert!(outcome.triggered.is_empty());
}

#[test]
fn financial_policy_blocks_per_transaction_breach() {
    let outcome = evaluate_financial_policies(
        &action(
            "refund-bot",
            FinancialActionKind::Refund,
            15_000,
            Some("cust_123"),
            Some("mandate_1"),
        ),
        [&policy()],
    );

    assert_eq!(outcome.verdict, Some(Verdict::Block));
    assert!(outcome.reason.unwrap().contains("per-transaction cap"));
}

#[test]
fn financial_policy_enforces_counterparty_and_mandate_controls() {
    let missing_mandate = evaluate_financial_policies(
        &action(
            "refund-bot",
            FinancialActionKind::Refund,
            1_000,
            Some("cust_123"),
            None,
        ),
        [&policy()],
    );
    assert_eq!(missing_mandate.verdict, Some(Verdict::Escalate));
    assert!(missing_mandate.reason.unwrap().contains("mandate"));

    let denied_counterparty = evaluate_financial_policies(
        &action(
            "refund-bot",
            FinancialActionKind::Refund,
            1_000,
            Some("blocked_customer"),
            Some("mandate_1"),
        ),
        [&policy()],
    );
    assert_eq!(denied_counterparty.verdict, Some(Verdict::Block));
    assert!(denied_counterparty
        .reason
        .unwrap()
        .contains("denied counterparty"));
}

#[test]
fn financial_policy_ignores_non_matching_actions() {
    let outcome = evaluate_financial_policies(
        &action(
            "other-agent",
            FinancialActionKind::Payment,
            15_000,
            Some("cust_123"),
            Some("mandate_1"),
        ),
        [&policy()],
    );

    assert_eq!(outcome.verdict, None);
    assert!(outcome.triggered.is_empty());
}
