//! Pure authorization composition and typed grant matching.

use std::collections::HashSet;

use tl_core::{
    ActionGrantScope, AuthorityRequirement, AuthorizationEffect, AuthorizationFinding,
    AuthorizationGrant, AuthorizationGrantScope, AuthorizationSubject, FinancialGrantScope,
    GrantMode, GrantStatus,
};

#[derive(Debug, Clone, PartialEq)]
pub struct FindingComposition {
    pub effect: AuthorizationEffect,
    pub satisfied_requirement_ids: Vec<String>,
    pub remaining_requirement_ids: Vec<String>,
    pub applied_grant_id: Option<String>,
}

/// Compose every finding with fail-closed precedence while allowing grants to
/// satisfy only the explicit requirements they cover.
pub fn compose_findings(
    findings: &[AuthorizationFinding],
    requirements: &[AuthorityRequirement],
    grants: &[AuthorizationGrant],
    subject: &AuthorizationSubject,
    exact_fingerprint: &str,
) -> FindingComposition {
    let requirement_by_id = requirements
        .iter()
        .map(|requirement| (requirement.id.as_str(), requirement))
        .collect::<std::collections::HashMap<_, _>>();
    let mut satisfied = HashSet::new();
    let mut applied_grant_id = None;

    for finding in findings
        .iter()
        .filter(|finding| finding.effect == AuthorizationEffect::RequireApproval)
    {
        let Some(requirement_id) = finding.requirement_id.as_deref() else {
            continue;
        };
        let Some(requirement) = requirement_by_id.get(requirement_id) else {
            continue;
        };
        if let Some(grant) = grants
            .iter()
            .find(|grant| grant_satisfies(grant, requirement, subject, exact_fingerprint))
        {
            satisfied.insert(requirement_id.to_string());
            applied_grant_id.get_or_insert_with(|| grant.id.clone());
        }
    }

    let mut effect = AuthorizationEffect::Permit;
    let mut remaining = Vec::new();
    for finding in findings {
        if finding.effect == AuthorizationEffect::RequireApproval {
            let covered = finding
                .requirement_id
                .as_ref()
                .is_some_and(|id| satisfied.contains(id));
            if covered {
                continue;
            }
            if let Some(id) = &finding.requirement_id {
                if !remaining.contains(id) {
                    remaining.push(id.clone());
                }
            }
        }
        effect = effect.worst_with(finding.effect);
    }

    let mut satisfied_requirement_ids = satisfied.into_iter().collect::<Vec<_>>();
    satisfied_requirement_ids.sort();
    remaining.sort();
    FindingComposition {
        effect,
        satisfied_requirement_ids,
        remaining_requirement_ids: remaining,
        applied_grant_id,
    }
}

pub fn grant_satisfies(
    grant: &AuthorizationGrant,
    requirement: &AuthorityRequirement,
    subject: &AuthorizationSubject,
    exact_fingerprint: &str,
) -> bool {
    if grant.status != GrantStatus::Active
        || grant.domain != subject.domain()
        || grant.capability != requirement.capability
        || !grant.requirement_ids.iter().any(|id| id == &requirement.id)
        || grant.max_uses.is_some_and(|max| grant.use_count >= max)
    {
        return false;
    }

    match grant.mode {
        GrantMode::ExactOnce => grant.exact_fingerprint.as_deref() == Some(exact_fingerprint),
        GrantMode::Scoped => match (&grant.scope, &requirement.required_scope) {
            (
                Some(AuthorizationGrantScope::Action(grant)),
                AuthorizationGrantScope::Action(required),
            ) => action_scope_covers(grant, required, subject),
            (
                Some(AuthorizationGrantScope::Financial(grant)),
                AuthorizationGrantScope::Financial(required),
            ) => financial_scope_covers(grant, required, subject),
            _ => false,
        },
    }
}

fn action_scope_covers(
    grant: &ActionGrantScope,
    required: &ActionGrantScope,
    subject: &AuthorizationSubject,
) -> bool {
    let AuthorizationSubject::Tool {
        operation,
        tool_identity,
        parameters,
        side_effect,
        ..
    } = subject
    else {
        return false;
    };

    contains_or_unbounded(&grant.operations, operation)
        && contains_or_unbounded(&grant.side_effects, side_effect)
        && optional_matches(grant.server_id.as_ref(), &tool_identity.server_id)
        && optional_matches(grant.tool_name.as_ref(), &tool_identity.tool_name)
        && optional_matches(grant.schema_hash.as_ref(), &tool_identity.schema_hash)
        && optional_matches(grant.parameters.as_ref(), parameters)
        && set_covers(&grant.operations, &required.operations)
        && set_covers(&grant.side_effects, &required.side_effects)
        && optional_equal_or_broader(&grant.server_id, &required.server_id)
        && optional_equal_or_broader(&grant.tool_name, &required.tool_name)
        && optional_equal_or_broader(&grant.schema_hash, &required.schema_hash)
        && optional_equal_or_broader(&grant.parameters, &required.parameters)
}

fn financial_scope_covers(
    grant: &FinancialGrantScope,
    required: &FinancialGrantScope,
    subject: &AuthorizationSubject,
) -> bool {
    let AuthorizationSubject::Financial { action, .. } = subject else {
        return false;
    };
    let counterparty = action.counterparty.as_ref().map(|value| value.id.as_str());
    contains_or_unbounded(&grant.action_kinds, &action.kind)
        && optional_matches(grant.operation.as_ref(), &action.operation)
        && optional_matches(grant.rail.as_ref(), &action.rail)
        && optional_matches(grant.currency.as_ref(), &action.amount.currency)
        && maximum_allows(grant.maximum_amount_minor, action.amount.amount_minor)
        && (grant.counterparties.is_empty()
            || counterparty.is_some_and(|id| grant.counterparties.iter().any(|value| value == id)))
        && set_covers(&grant.action_kinds, &required.action_kinds)
        && optional_equal_or_broader(&grant.operation, &required.operation)
        && optional_equal_or_broader(&grant.rail, &required.rail)
        && optional_equal_or_broader(&grant.currency, &required.currency)
        && maximum_covers(grant.maximum_amount_minor, required.maximum_amount_minor)
        && set_covers(&grant.counterparties, &required.counterparties)
        && set_covers(
            &grant.required_preconditions,
            &required.required_preconditions,
        )
}

fn contains_or_unbounded<T: PartialEq>(values: &[T], expected: &T) -> bool {
    values.is_empty() || values.contains(expected)
}

fn optional_matches<T: PartialEq>(constraint: Option<&T>, actual: &T) -> bool {
    match constraint {
        Some(value) => value == actual,
        None => true,
    }
}

fn maximum_allows(maximum: Option<i64>, actual: i64) -> bool {
    match maximum {
        Some(maximum) => actual <= maximum,
        None => true,
    }
}

fn maximum_covers(granted: Option<i64>, required: Option<i64>) -> bool {
    match (granted, required) {
        (None, _) => true,
        (Some(granted), Some(required)) => granted >= required,
        (Some(_), None) => false,
    }
}

fn set_covers<T: PartialEq>(granted: &[T], required: &[T]) -> bool {
    granted.is_empty() || required.iter().all(|value| granted.contains(value))
}

fn optional_equal_or_broader<T: PartialEq>(granted: &Option<T>, required: &Option<T>) -> bool {
    match (granted, required) {
        (None, _) => true,
        (Some(granted), Some(required)) => granted == required,
        (Some(_), None) => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tl_core::{
        AuthorizationCapabilityId, AuthorizationDomain, AuthorizationGrantSource, GrantStatus,
        Severity, SideEffectClass, ToolIdentity,
    };

    fn finding(effect: AuthorizationEffect, requirement_id: Option<&str>) -> AuthorizationFinding {
        AuthorizationFinding {
            id: format!("finding-{effect:?}"),
            source: "test".into(),
            effect,
            reason: "test".into(),
            severity: Severity::High,
            policy_id: None,
            requirement_id: requirement_id.map(str::to_string),
            remediation: None,
            evidence: serde_json::Value::Null,
        }
    }

    fn scope() -> ActionGrantScope {
        ActionGrantScope {
            operations: vec!["mail/send".into()],
            side_effects: vec![SideEffectClass::ExternalCommunication],
            server_id: Some("mail".into()),
            tool_name: Some("send".into()),
            schema_hash: Some("sha256:v1:schema".into()),
            parameters: Some(serde_json::json!({"to": "a@example.com"})),
            allowed_destinations: vec!["a@example.com".into()],
            maximum_data_confidentiality: None,
            minimum_source_trust: None,
        }
    }

    fn subject() -> AuthorizationSubject {
        AuthorizationSubject::Tool {
            invocation_id: "inv-1".into(),
            operation: "mail/send".into(),
            tool_identity: ToolIdentity {
                server_id: "mail".into(),
                tool_name: "send".into(),
                schema_hash: "sha256:v1:schema".into(),
            },
            parameters: serde_json::json!({"to": "a@example.com"}),
            side_effect: SideEffectClass::ExternalCommunication,
        }
    }

    #[test]
    fn hard_effects_win_even_when_approval_is_satisfied() {
        let capability = AuthorizationCapabilityId::parse("tool:mail/send").unwrap();
        let requirement = AuthorityRequirement {
            id: "req-1".into(),
            capability: capability.clone(),
            required_scope: AuthorizationGrantScope::Action(scope()),
            approver_roles: vec![],
            reason: "review".into(),
            reusable_allowed: true,
            max_grant_ttl_seconds: None,
        };
        let grant = AuthorizationGrant {
            id: "grant-1".into(),
            workspace_id: "ws".into(),
            environment_id: "prod".into(),
            principal_id: "agent".into(),
            domain: AuthorizationDomain::Tool,
            capability,
            mode: GrantMode::Scoped,
            status: GrantStatus::Active,
            source: AuthorizationGrantSource::ReviewerApproval,
            scope: Some(AuthorizationGrantScope::Action(scope())),
            exact_fingerprint: None,
            fingerprint_version: 1,
            source_approval_id: Some("approval-1".into()),
            requirement_ids: vec!["req-1".into()],
            max_uses: None,
            use_count: 0,
            starts_at: None,
            expires_at: None,
            created_by: "reviewer".into(),
            created_at: String::new(),
            updated_at: String::new(),
        };
        let result = compose_findings(
            &[
                finding(AuthorizationEffect::RequireApproval, Some("req-1")),
                finding(AuthorizationEffect::Deny, None),
            ],
            &[requirement],
            &[grant],
            &subject(),
            "sha256:v1:exact",
        );
        assert_eq!(result.effect, AuthorizationEffect::Deny);
        assert_eq!(result.satisfied_requirement_ids, ["req-1"]);
    }

    #[test]
    fn unrelated_grant_does_not_remove_requirement() {
        let result = compose_findings(
            &[finding(AuthorizationEffect::RequireApproval, Some("req-1"))],
            &[],
            &[],
            &subject(),
            "sha256:v1:exact",
        );
        assert_eq!(result.effect, AuthorizationEffect::RequireApproval);
        assert_eq!(result.remaining_requirement_ids, ["req-1"]);
    }
}
