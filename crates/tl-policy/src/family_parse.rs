//! YAML loader for policy families. Mirrors `policy_parse::load_str` for
//! the family documents introduced in event-engine phase 6.
//!
//! A document without a `family:` field (or with `family: content`)
//! parses through the legacy content path unchanged; any other family
//! parses into `FamilyPolicy` and gets family-specific validation.

use serde::Deserialize;

use tl_core::{AuthorizationEffect, SpendMeter};

use crate::family_ast::{
    AnyPolicy, ApprovalPolicy, FamilyPolicy, FinancialPolicy, FinancialWhen, FlowPolicy, FlowRule,
    SourceLabelFamilyPolicy,
};
use crate::policy_parse::{format_issues, load_str, PolicyError, ValidationIssue};

/// Every recognized `family:` tag value. `content` selects the legacy
/// `Policy` shape; the rest select `FamilyPolicy` variants.
pub const KNOWN_FAMILIES: [&str; 7] = [
    "content",
    "flow",
    "parameter_source",
    "approval",
    "memory",
    "financial",
    "source_label",
];

#[derive(Debug, Deserialize)]
struct FamilyProbe {
    #[serde(default)]
    family: Option<String>,
}

/// Parse one policy document of any family from YAML.
///
/// Documents without a `family:` field keep the exact legacy content
/// behavior of `load_str`, including its validation and error messages.
pub fn load_any_str(src: &str) -> Result<AnyPolicy, PolicyError> {
    let probe: FamilyProbe = serde_yaml::from_str(src)?;
    match probe.family.as_deref() {
        None | Some("content") => Ok(AnyPolicy::Content(load_str(src)?)),
        Some(family) if KNOWN_FAMILIES.contains(&family) => {
            let policy: FamilyPolicy = serde_yaml::from_str(src)?;
            if let Err(issues) = validate_family_policy(&policy) {
                return Err(PolicyError::Validation(format_issues(&issues)));
            }
            Ok(AnyPolicy::Family(policy))
        }
        Some(other) => {
            // Truncate before echoing: the family value is caller-supplied
            // and this error can reach API responses.
            let display: String = other.chars().take(64).collect();
            Err(PolicyError::Validation(format!(
                "family: unknown policy family `{display}` (expected one of: {})",
                KNOWN_FAMILIES.join(", ")
            )))
        }
    }
}

pub fn validate_family_policy(policy: &FamilyPolicy) -> Result<(), Vec<ValidationIssue>> {
    let mut issues = vec![];

    validate_id(policy.id(), &mut issues);
    match policy {
        FamilyPolicy::Flow(flow) => validate_flow(flow, &mut issues),
        FamilyPolicy::ParameterSource(param) => {
            validate_required_text("tool", &param.tool, &mut issues);
            validate_required_text("param", &param.param, &mut issues);
            validate_enforcing_action("action", param.action, &mut issues);
        }
        FamilyPolicy::Approval(approval) => validate_approval(approval, &mut issues),
        FamilyPolicy::Memory(memory) => {
            validate_enforcing_action("action", memory.action, &mut issues);
        }
        FamilyPolicy::Financial(financial) => validate_financial(financial, &mut issues),
        FamilyPolicy::SourceLabel(source_label) => validate_source_label(source_label, &mut issues),
    }

    if issues.is_empty() {
        Ok(())
    } else {
        Err(issues)
    }
}

fn validate_source_label(
    source_label: &SourceLabelFamilyPolicy,
    issues: &mut Vec<ValidationIssue>,
) {
    if source_label.trust.is_none()
        && source_label.confidentiality.is_none()
        && source_label.integrity.is_none()
    {
        issues.push(ValidationIssue::new(
            "labels",
            "must set at least one of trust, confidentiality, integrity",
        ));
    }
}

fn validate_flow(flow: &FlowPolicy, issues: &mut Vec<ValidationIssue>) {
    if let FlowRule::DestinationPermission { sinks } = &flow.rule {
        if sinks.is_empty() {
            issues.push(ValidationIssue::new(
                "rule.sinks",
                "must contain at least one sink side-effect class",
            ));
        }
    }
    validate_enforcing_action("action", flow.action, issues);
}

fn validate_approval(approval: &ApprovalPolicy, issues: &mut Vec<ValidationIssue>) {
    if approval.when.tools.is_empty() && approval.when.side_effects.is_empty() {
        issues.push(ValidationIssue::new(
            "when",
            "must set at least one of tools or side_effects",
        ));
    }
    for (idx, tool) in approval.when.tools.iter().enumerate() {
        if tool.trim().is_empty() {
            issues.push(ValidationIssue::new(
                format!("when.tools[{idx}]"),
                "must not be empty",
            ));
        }
    }
    for (idx, role) in approval.approver_roles.iter().enumerate() {
        if role.trim().is_empty() {
            issues.push(ValidationIssue::new(
                format!("approver_roles[{idx}]"),
                "must not be empty",
            ));
        }
    }
    validate_enforcing_action("action", approval.action, issues);
}

fn validate_financial(financial: &FinancialPolicy, issues: &mut Vec<ValidationIssue>) {
    match financial.meter {
        SpendMeter::Actions => validate_financial_when(&financial.when, issues),
        // An llm_usage budget with no selector is the common case (one
        // cap for every principal), so the at-least-one-selector rule
        // does not apply. Action-only selectors can never match a
        // gateway call and would silently disable the budget — reject
        // them loudly instead.
        SpendMeter::LlmUsage => {
            if !financial.when.action_kinds.is_empty() {
                issues.push(ValidationIssue::new(
                    "when.action_kinds",
                    "action_kinds do not apply to the llm_usage meter",
                ));
            }
            if !financial.when.rails.is_empty() {
                issues.push(ValidationIssue::new(
                    "when.rails",
                    "rails do not apply to the llm_usage meter",
                ));
            }
        }
    }

    let has_amount_control = financial.per_transaction_minor.is_some()
        || financial.approval_threshold_minor.is_some()
        || financial.daily_minor.is_some()
        || financial.weekly_minor.is_some()
        || financial.monthly_minor.is_some();
    let has_rule_control = !financial.allowed_counterparty_ids.is_empty()
        || !financial.denied_counterparty_ids.is_empty()
        || financial.require_approval_for_new_counterparty
        || financial.grant_required
        || financial.refund_original_method_only
        || !financial.required_preconditions.is_empty();
    if !has_amount_control && !has_rule_control {
        issues.push(ValidationIssue::new(
            "controls",
            "financial policy must set at least one cap or control",
        ));
    }

    for (field, value) in [
        ("per_transaction_minor", financial.per_transaction_minor),
        (
            "approval_threshold_minor",
            financial.approval_threshold_minor,
        ),
        ("daily_minor", financial.daily_minor),
        ("weekly_minor", financial.weekly_minor),
        ("monthly_minor", financial.monthly_minor),
    ] {
        if matches!(value, Some(v) if v < 0) {
            issues.push(ValidationIssue::new(field, "amount must be non-negative"));
        }
    }

    validate_non_empty_strings("when.agents", &financial.when.agents, issues);
    validate_non_empty_strings("when.operations", &financial.when.operations, issues);
    validate_non_empty_strings("when.currencies", &financial.when.currencies, issues);
    validate_non_empty_strings("approver_roles", &financial.approver_roles, issues);
    validate_non_empty_strings(
        "allowed_counterparty_ids",
        &financial.allowed_counterparty_ids,
        issues,
    );
    validate_non_empty_strings(
        "denied_counterparty_ids",
        &financial.denied_counterparty_ids,
        issues,
    );
    validate_enforcing_action(
        "missing_evidence_effect",
        financial.missing_evidence_effect,
        issues,
    );
    validate_enforcing_action(
        "failed_precondition_effect",
        financial.failed_precondition_effect,
        issues,
    );
    validate_enforcing_action("on_breach", financial.on_breach, issues);
}

fn validate_financial_when(when: &FinancialWhen, issues: &mut Vec<ValidationIssue>) {
    let has_selector = !when.agents.is_empty()
        || !when.action_kinds.is_empty()
        || !when.operations.is_empty()
        || !when.currencies.is_empty()
        || !when.rails.is_empty();
    if !has_selector {
        issues.push(ValidationIssue::new(
            "when",
            "financial policy must set at least one selector",
        ));
    }
}

/// Same slug rule as content policies (`policy_parse::validate_id`).
fn validate_id(id: &str, issues: &mut Vec<ValidationIssue>) {
    let id = id.trim();
    if id.is_empty() {
        issues.push(ValidationIssue::new("id", "id is required"));
        return;
    }
    let valid = id
        .chars()
        .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-' || c == '_');
    if !valid {
        issues.push(ValidationIssue::new(
            "id",
            "id must use lowercase letters, numbers, '-' or '_'",
        ));
    }
}

fn validate_required_text(path: &str, value: &str, issues: &mut Vec<ValidationIssue>) {
    if value.trim().is_empty() {
        issues.push(ValidationIssue::new(path, "must not be empty"));
    }
}

fn validate_non_empty_strings(path: &str, values: &[String], issues: &mut Vec<ValidationIssue>) {
    for (idx, value) in values.iter().enumerate() {
        if value.trim().is_empty() {
            issues.push(ValidationIssue::new(
                format!("{path}[{idx}]"),
                "must not be empty",
            ));
        }
    }
}

/// Non-content families describe action safety: `allow` and `rewrite`
/// are content-policy effects and make no sense here.
fn validate_enforcing_action(
    path: &str,
    effect: AuthorizationEffect,
    issues: &mut Vec<ValidationIssue>,
) {
    if !matches!(
        effect,
        AuthorizationEffect::Deny
            | AuthorizationEffect::RequireApproval
            | AuthorizationEffect::Defer
    ) {
        issues.push(ValidationIssue::new(
            path,
            "must be deny, defer, or require_approval",
        ));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::family_ast::ParameterSourcePolicy;
    use tl_core::{Origin, SideEffectClass};

    fn family(src: &str) -> FamilyPolicy {
        match load_any_str(src).expect("parse") {
            AnyPolicy::Family(policy) => policy,
            AnyPolicy::Content(_) => panic!("expected a family policy"),
        }
    }

    #[test]
    fn family_less_yaml_parses_as_content_identical_to_load_str() {
        let yaml = r#"
id: refund-promise
match:
  literal: "refund"
action: deny
"#;
        let via_any = match load_any_str(yaml).expect("parse") {
            AnyPolicy::Content(policy) => policy,
            AnyPolicy::Family(_) => panic!("expected content"),
        };
        let via_legacy = load_str(yaml).expect("parse");
        assert_eq!(
            serde_yaml::to_string(&via_any).unwrap(),
            serde_yaml::to_string(&via_legacy).unwrap()
        );
    }

    #[test]
    fn explicit_content_family_parses_as_content() {
        let yaml = r#"
family: content
id: refund-promise
match:
  literal: "refund"
action: deny
"#;
        assert!(matches!(
            load_any_str(yaml).expect("parse"),
            AnyPolicy::Content(_)
        ));
    }

    #[test]
    fn unknown_family_fails_with_clear_error() {
        let yaml = r#"
family: bananas
id: nonsense
action: deny
"#;
        let err = load_any_str(yaml).unwrap_err().to_string();
        assert!(err.contains("unknown policy family `bananas`"), "{err}");
        assert!(err.contains("content, flow, parameter_source, approval, memory"));
    }

    #[test]
    fn parses_flow_destination_permission_policy() {
        let yaml = r#"
family: flow
id: no-private-to-external
description: Sensitive data must not reach external sinks.
severity: critical
rule: destination_permission
sinks: [external_communication, network_call]
action: deny
"#;
        let FamilyPolicy::Flow(flow) = family(yaml) else {
            panic!("expected flow");
        };
        assert_eq!(flow.id, "no-private-to-external");
        let FlowRule::DestinationPermission { sinks } = flow.rule else {
            panic!("expected destination_permission");
        };
        assert_eq!(
            sinks,
            vec![
                SideEffectClass::ExternalCommunication,
                SideEffectClass::NetworkCall
            ]
        );
    }

    #[test]
    fn parses_flow_action_integrity_policy() {
        let yaml = r#"
family: flow
id: trusted-control-only
rule: action_integrity
action: require_approval
"#;
        let FamilyPolicy::Flow(flow) = family(yaml) else {
            panic!("expected flow");
        };
        assert!(matches!(flow.rule, FlowRule::ActionIntegrity));
    }

    #[test]
    fn flow_destination_permission_requires_sinks() {
        let yaml = r#"
family: flow
id: empty-sinks
rule: destination_permission
sinks: []
action: deny
"#;
        let err = load_any_str(yaml).unwrap_err().to_string();
        assert!(err.contains("rule.sinks"), "{err}");
    }

    #[test]
    fn parses_parameter_source_policy() {
        let yaml = r#"
family: parameter_source
id: email-recipient-from-user
tool: send_email
param: to
allowed_sources:
  - origin: user
  - origin: tool
    kind: contact_lookup
action: deny
"#;
        let FamilyPolicy::ParameterSource(param) = family(yaml) else {
            panic!("expected parameter_source");
        };
        assert_eq!(param.tool, "send_email");
        assert_eq!(param.param, "to");
        assert_eq!(param.allowed_sources.len(), 2);
        assert_eq!(param.allowed_sources[0].origin, Origin::User);
        assert_eq!(
            param.allowed_sources[1].kind.as_deref(),
            Some("contact_lookup")
        );
    }

    #[test]
    fn parameter_source_requires_tool_and_param() {
        let yaml = r#"
family: parameter_source
id: incomplete
tool: ""
param: " "
allowed_sources: []
action: deny
"#;
        let err = load_any_str(yaml).unwrap_err().to_string();
        assert!(err.contains("tool: must not be empty"), "{err}");
        assert!(err.contains("param: must not be empty"), "{err}");
    }

    #[test]
    fn payment_family_is_no_longer_supported() {
        let yaml = r#"
family: payment
id: alice-caps
"#;
        let err = load_any_str(yaml).unwrap_err().to_string();
        assert!(err.contains("unknown policy family `payment`"), "{err}");
        assert!(!err.contains("payment, financial"), "{err}");
    }

    #[test]
    fn parses_financial_policy() {
        let yaml = r#"
family: financial
id: refund-financial-controls
description: Refund controls for support agents.
severity: high
when:
  agents: [refund-bot]
  action_kinds: [refund]
  operations: [issue_refund]
  currencies: [USD]
  rails: [payment_http]
per_transaction_minor: 10000
daily_minor: 50000
monthly_minor: 500000
allowed_counterparty_ids: [cust_123]
denied_counterparty_ids: [cust_blocked]
require_approval_for_new_counterparty: true
grant_required: true
approval_threshold_minor: 5000
approver_roles: [finance_admin]
refund_original_method_only: true
required_preconditions:
  - order_exists
  - amount_lte_refundable_balance
missing_evidence_effect: defer
failed_precondition_effect: deny
on_breach: deny
"#;
        let FamilyPolicy::Financial(financial) = family(yaml) else {
            panic!("expected financial");
        };
        assert_eq!(financial.when.agents, vec!["refund-bot"]);
        assert_eq!(
            financial.when.action_kinds,
            vec![tl_core::FinancialActionKind::Refund]
        );
        assert_eq!(financial.when.operations, vec!["issue_refund"]);
        assert_eq!(financial.when.currencies, vec!["USD"]);
        assert_eq!(
            financial.when.rails,
            vec![tl_core::FinancialRail::PaymentHttp]
        );
        assert_eq!(financial.per_transaction_minor, Some(10000));
        assert_eq!(financial.daily_minor, Some(50000));
        assert_eq!(financial.monthly_minor, Some(500000));
        assert_eq!(financial.allowed_counterparty_ids, vec!["cust_123"]);
        assert_eq!(financial.denied_counterparty_ids, vec!["cust_blocked"]);
        assert!(financial.require_approval_for_new_counterparty);
        assert!(financial.grant_required);
        assert_eq!(financial.approval_threshold_minor, Some(5000));
        assert_eq!(financial.approver_roles, vec!["finance_admin"]);
        assert!(financial.refund_original_method_only);
        assert_eq!(
            financial.required_preconditions,
            vec![
                tl_core::FinancialActionPrecondition::OrderExists,
                tl_core::FinancialActionPrecondition::AmountLteRefundableBalance,
            ]
        );
        assert_eq!(
            financial.missing_evidence_effect,
            AuthorizationEffect::Defer
        );
        assert_eq!(
            financial.failed_precondition_effect,
            AuthorizationEffect::Deny
        );
        assert_eq!(financial.on_breach, AuthorizationEffect::Deny);
    }

    /// Every financial policy stored before the meter existed must keep
    /// deserializing — and land on the actions meter (backward compat
    /// for the unified policy registry, YAML and JSON alike).
    #[test]
    fn financial_policy_without_meter_defaults_to_actions() {
        let yaml = r#"
family: financial
id: refund-financial-controls
when:
  action_kinds: [refund]
per_transaction_minor: 10000
"#;
        let FamilyPolicy::Financial(financial) = family(yaml) else {
            panic!("expected financial");
        };
        assert_eq!(financial.meter, SpendMeter::Actions);

        // Postgres stores the parsed policy as JSON; a stored payload
        // without the field must default the same way.
        let stored = serde_json::json!({
            "family": "financial",
            "id": "refund-financial-controls",
            "when": { "action_kinds": ["refund"] },
            "per_transaction_minor": 10000
        });
        let reparsed: FamilyPolicy = serde_json::from_value(stored).expect("stored deserialize");
        let FamilyPolicy::Financial(financial) = reparsed else {
            panic!("expected financial");
        };
        assert_eq!(financial.meter, SpendMeter::Actions);
    }

    /// An llm_usage budget commonly has no selectors at all (one cap
    /// for every principal); it must parse, validate, and round-trip
    /// with its meter intact.
    #[test]
    fn llm_usage_meter_allows_empty_when_and_round_trips() {
        let yaml = r#"
family: financial
id: llm-weekly-budget
meter: llm_usage
weekly_minor: 500000
"#;
        let FamilyPolicy::Financial(financial) = family(yaml) else {
            panic!("expected financial");
        };
        assert_eq!(financial.meter, SpendMeter::LlmUsage);

        let serialized =
            serde_yaml::to_string(&FamilyPolicy::Financial(financial)).expect("serialize");
        let reparsed: FamilyPolicy = serde_yaml::from_str(&serialized).expect("reparse");
        let FamilyPolicy::Financial(financial) = reparsed else {
            panic!("expected financial after round trip");
        };
        assert_eq!(financial.meter, SpendMeter::LlmUsage);
    }

    #[test]
    fn llm_usage_meter_rejects_action_only_selectors() {
        let yaml = r#"
family: financial
id: llm-budget-bad-selectors
meter: llm_usage
when:
  action_kinds: [refund]
  rails: [card]
weekly_minor: 500000
"#;
        let err = load_any_str(yaml).unwrap_err().to_string();
        assert!(
            err.contains("action_kinds do not apply to the llm_usage meter"),
            "{err}"
        );
        assert!(
            err.contains("rails do not apply to the llm_usage meter"),
            "{err}"
        );
    }

    #[test]
    fn unknown_meter_value_fails_to_parse() {
        let yaml = r#"
family: financial
id: llm-budget-typo
meter: llm_usag
weekly_minor: 500000
"#;
        let err = load_any_str(yaml).unwrap_err().to_string();
        assert!(
            err.contains("llm_usag") || err.contains("unknown variant"),
            "{err}"
        );
    }

    #[test]
    fn financial_policy_requires_selector_and_control() {
        let yaml = r#"
family: financial
id: empty-financial
when:
  agents: []
"#;
        let err = load_any_str(yaml).unwrap_err().to_string();
        assert!(err.contains("when"), "{err}");
        assert!(err.contains("control"), "{err}");
    }

    #[test]
    fn financial_policy_rejects_negative_amounts_and_non_enforcing_actions() {
        let yaml = r#"
family: financial
id: bad-financial
when:
  action_kinds: [refund]
per_transaction_minor: -1
approver_roles: [""]
missing_evidence_effect: permit
failed_precondition_effect: transform
on_breach: permit
"#;
        let err = load_any_str(yaml).unwrap_err().to_string();
        assert!(err.contains("per_transaction_minor"), "{err}");
        assert!(err.contains("approver_roles"), "{err}");
        assert!(err.contains("missing_evidence_effect"), "{err}");
        assert!(err.contains("failed_precondition_effect"), "{err}");
        assert!(err.contains("on_breach"), "{err}");
    }

    #[test]
    fn parses_approval_policy() {
        let yaml = r#"
family: approval
id: payments-need-admin
description: Payments require human sign-off.
severity: critical
when:
  side_effects: [api_mutation]
  tools: [payment.transfer]
approver_roles: [admin]
reason: "Irreversible money movement."
action: require_approval
"#;
        let FamilyPolicy::Approval(approval) = family(yaml) else {
            panic!("expected approval");
        };
        assert_eq!(approval.when.tools, vec!["payment.transfer"]);
        assert_eq!(
            approval.when.side_effects,
            vec![SideEffectClass::ApiMutation]
        );
        assert_eq!(approval.approver_roles, vec!["admin"]);
    }

    #[test]
    fn approval_requires_at_least_one_condition() {
        let yaml = r#"
family: approval
id: unconditional
when: {}
action: require_approval
"#;
        let err = load_any_str(yaml).unwrap_err().to_string();
        assert!(
            err.contains("when: must set at least one of tools or side_effects"),
            "{err}"
        );
    }

    #[test]
    fn parses_memory_policy() {
        let yaml = r#"
family: memory
id: no-untrusted-authority-writes
deny_untrusted_authority_writes: true
action: require_approval
"#;
        let FamilyPolicy::Memory(memory) = family(yaml) else {
            panic!("expected memory");
        };
        assert!(memory.deny_untrusted_authority_writes);
    }

    #[test]
    fn non_content_families_reject_non_enforcing_effects() {
        let yaml = r#"
family: memory
id: allow-is-meaningless
deny_untrusted_authority_writes: true
action: permit
"#;
        let err = load_any_str(yaml).unwrap_err().to_string();
        assert!(
            err.contains("action: must be deny, defer, or require_approval"),
            "{err}"
        );
    }

    #[test]
    fn family_id_uses_content_slug_rule() {
        let yaml = r#"
family: memory
id: "Bad Id"
deny_untrusted_authority_writes: true
action: deny
"#;
        let err = load_any_str(yaml).unwrap_err().to_string();
        assert!(err.contains("lowercase letters"), "{err}");
    }

    #[test]
    fn family_policies_round_trip_through_yaml_with_family_tag() {
        let yaml = r#"
family: parameter_source
id: email-recipient-from-user
tool: send_email
param: to
allowed_sources:
  - origin: user
action: deny
"#;
        let parsed = family(yaml);
        let serialized = serde_yaml::to_string(&parsed).expect("serialize");
        assert!(
            serialized.contains("family: parameter_source"),
            "{serialized}"
        );
        let reparsed: FamilyPolicy = serde_yaml::from_str(&serialized).expect("reparse");
        let FamilyPolicy::ParameterSource(ParameterSourcePolicy { id, .. }) = reparsed else {
            panic!("expected parameter_source after round trip");
        };
        assert_eq!(id, "email-recipient-from-user");
    }

    #[test]
    fn documented_family_examples_parse() {
        for (name, yaml) in [
            (
                "flow-private-external",
                include_str!("../../../docs/policies/examples/flow-private-external.yaml"),
            ),
            (
                "parameter-source-email-recipient",
                include_str!(
                    "../../../docs/policies/examples/parameter-source-email-recipient.yaml"
                ),
            ),
            (
                "approval-payments",
                include_str!("../../../docs/policies/examples/approval-payments.yaml"),
            ),
            (
                "memory-untrusted-writes",
                include_str!("../../../docs/policies/examples/memory-untrusted-writes.yaml"),
            ),
        ] {
            match load_any_str(yaml) {
                Ok(AnyPolicy::Family(_)) => {}
                Ok(AnyPolicy::Content(_)) => {
                    panic!("documented family example `{name}` parsed as content")
                }
                Err(e) => panic!("documented family example `{name}` failed: {e}"),
            }
        }
    }

    #[test]
    fn existing_content_examples_parse_via_load_any_str() {
        for (name, yaml) in [
            (
                "refund-guarantee",
                include_str!("../../../docs/policies/examples/refund-guarantee.yaml"),
            ),
            (
                "pii-block",
                include_str!("../../../docs/policies/examples/pii-block.yaml"),
            ),
            (
                "legal-escalation",
                include_str!("../../../docs/policies/examples/legal-escalation.yaml"),
            ),
            (
                "voice-disclosure",
                include_str!("../../../docs/policies/examples/voice-disclosure.yaml"),
            ),
        ] {
            match load_any_str(yaml) {
                Ok(AnyPolicy::Content(_)) => {}
                Ok(AnyPolicy::Family(_)) => {
                    panic!("content example `{name}` parsed as a family policy")
                }
                Err(e) => panic!("content example `{name}` failed via load_any_str: {e}"),
            }
        }
    }
}
