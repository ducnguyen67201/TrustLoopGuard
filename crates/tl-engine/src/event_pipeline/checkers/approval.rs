//! Approval checker: tools whose registry metadata requires human
//! approval return `require_approval` instead of executing.
//!
//! Reads the `ApprovalRule` already resolved onto the event by the tool
//! metadata registry. Unregistered or unresolved tools emit nothing —
//! unknown-tool conservatism is the flow/parameter checkers' job, and a
//! second approval requirement here would only duplicate their evidence.

use tl_core::{ApprovalRule, AuthorizationEffect, GuardEvent, ToolResolution};

use crate::event_pipeline::{Checker, CheckerFinding};

pub const APPROVAL_CHECKER_ID: &str = "approval";

const FAILURE_APPROVAL_REQUIRED: &str = "approval_required";

pub struct ApprovalChecker;

impl Checker for ApprovalChecker {
    fn id(&self) -> &'static str {
        APPROVAL_CHECKER_ID
    }

    fn check(&self, event: &GuardEvent) -> Vec<CheckerFinding> {
        let Some(ToolResolution::Resolved { metadata }) = &event.resolution else {
            return vec![];
        };
        let Some(rule) = &metadata.approval else {
            return vec![];
        };
        if !rule.required {
            return vec![];
        }
        vec![CheckerFinding {
            checker_id: APPROVAL_CHECKER_ID.to_string(),
            effect: Some(AuthorizationEffect::RequireApproval),
            reason: format!(
                "tool '{}' requires human approval before execution",
                metadata.tool
            ),
            violated_rule: Some(format!("approval.{}", metadata.tool)),
            remediation: Some(remediation(rule)),
            source_chain: vec![],
            risk_source: None,
            risk_code: Some(FAILURE_APPROVAL_REQUIRED.to_string()),
            harm_class: Some("authorization".to_string()),
        }]
    }
}

fn remediation(rule: &ApprovalRule) -> String {
    if let Some(reason) = rule
        .reason
        .as_deref()
        .map(str::trim)
        .filter(|r| !r.is_empty())
    {
        return reason.to_string();
    }
    if rule.approver_roles.is_empty() {
        "request approval from an approver before retrying this action".to_string()
    } else {
        format!(
            "request approval from roles: {} before retrying this action",
            rule.approver_roles.join(", ")
        )
    }
}

#[cfg(test)]
mod tests {
    use super::super::test_support::{event, source};
    use super::*;
    use tl_core::{
        EventKind, Labels, Origin, ProvenanceMap, SideEffectClass, ToolMetadata, ToolResolution,
    };

    fn metadata(approval: Option<ApprovalRule>) -> ToolMetadata {
        ToolMetadata {
            tool: "payment.transfer".into(),
            side_effect: SideEffectClass::ApiMutation,
            reversible: false,
            params: vec![],
            approval,
            sandbox_hint: None,
        }
    }

    fn resolved_event(approval: Option<ApprovalRule>) -> tl_core::GuardEvent {
        let mut event = event(
            EventKind::ToolCallProposed,
            Some(SideEffectClass::ApiMutation),
            vec![source("src.user", Origin::User, Labels::default())],
            ProvenanceMap::default(),
        );
        event.resolution = Some(ToolResolution::Resolved {
            metadata: metadata(approval),
        });
        event
    }

    #[test]
    fn escalates_when_tool_requires_approval() {
        let event = resolved_event(Some(ApprovalRule {
            required: true,
            approver_roles: vec!["admin".into()],
            reason: None,
        }));

        let findings = ApprovalChecker.check(&event);
        assert_eq!(findings.len(), 1);
        let finding = &findings[0];
        assert_eq!(finding.effect, Some(AuthorizationEffect::RequireApproval));
        assert_eq!(
            finding.violated_rule.as_deref(),
            Some("approval.payment.transfer")
        );
        assert_eq!(
            finding.remediation.as_deref(),
            Some("request approval from roles: admin before retrying this action")
        );
        assert_eq!(finding.risk_code.as_deref(), Some("approval_required"));
        assert_eq!(finding.harm_class.as_deref(), Some("authorization"));
        assert!(finding.source_chain.is_empty());
        assert!(finding.risk_source.is_none());
    }

    #[test]
    fn registry_reason_wins_over_generated_remediation() {
        let event = resolved_event(Some(ApprovalRule {
            required: true,
            approver_roles: vec!["admin".into()],
            reason: Some("Irreversible money movement.".into()),
        }));

        let findings = ApprovalChecker.check(&event);
        assert_eq!(
            findings[0].remediation.as_deref(),
            Some("Irreversible money movement.")
        );
    }

    #[test]
    fn empty_roles_fall_back_to_generic_remediation() {
        let event = resolved_event(Some(ApprovalRule {
            required: true,
            approver_roles: vec![],
            reason: None,
        }));

        let findings = ApprovalChecker.check(&event);
        assert_eq!(
            findings[0].remediation.as_deref(),
            Some("request approval from an approver before retrying this action")
        );
    }

    #[test]
    fn not_required_emits_nothing() {
        let event = resolved_event(Some(ApprovalRule {
            required: false,
            approver_roles: vec!["admin".into()],
            reason: None,
        }));

        assert!(ApprovalChecker.check(&event).is_empty());
    }

    #[test]
    fn no_approval_rule_emits_nothing() {
        let event = resolved_event(None);

        assert!(ApprovalChecker.check(&event).is_empty());
    }

    #[test]
    fn unresolved_tool_emits_nothing() {
        let mut event = resolved_event(Some(ApprovalRule {
            required: true,
            approver_roles: vec![],
            reason: None,
        }));
        event.resolution = Some(ToolResolution::Unregistered);
        assert!(ApprovalChecker.check(&event).is_empty());

        event.resolution = None;
        assert!(ApprovalChecker.check(&event).is_empty());
    }
}
