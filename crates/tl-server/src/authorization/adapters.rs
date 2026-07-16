//! Typed domain adapters for the authorization coordinator.

use async_trait::async_trait;
use sha2::{Digest, Sha256};
use tl_core::{
    ActionGrantScope, AuthorityRequirement, AuthorizationCapabilityId, AuthorizationDomain,
    AuthorizationDomainEvidence, AuthorizationFinding, AuthorizationGrantScope,
    AuthorizationSubject, FinancialGrantScope,
};

mod sealed {
    pub trait Sealed {}
}

#[derive(Debug, thiserror::Error)]
pub enum AuthorizationAdapterError {
    #[error("subject does not match adapter domain")]
    DomainMismatch,
    #[error("invalid authorization subject: {0}")]
    Invalid(String),
    #[error("authorization adapter unavailable: {0}")]
    Unavailable(String),
}

#[derive(Debug, Clone, Default)]
pub struct AdapterPolicyBoundary {
    pub findings: Vec<AuthorizationFinding>,
    pub requirements: Vec<AuthorityRequirement>,
    pub policy_versions: Vec<String>,
}

#[async_trait]
pub trait AuthorizationAdapter: sealed::Sealed + Send + Sync {
    fn domain(&self) -> AuthorizationDomain;

    fn normalize(
        &self,
        subject: &AuthorizationSubject,
    ) -> Result<AuthorizationSubject, AuthorizationAdapterError> {
        self.ensure_domain(subject)?;
        Ok(subject.clone())
    }

    fn capability(
        &self,
        subject: &AuthorizationSubject,
    ) -> Result<AuthorizationCapabilityId, AuthorizationAdapterError>;

    fn fingerprint(
        &self,
        workspace_id: &str,
        environment_id: &str,
        principal_id: &str,
        subject: &AuthorizationSubject,
    ) -> Result<String, AuthorizationAdapterError> {
        self.ensure_domain(subject)?;
        subject_fingerprint(workspace_id, environment_id, principal_id, subject)
    }

    fn policy_boundary(
        &self,
        _principal_id: &str,
        subject: &AuthorizationSubject,
        _policies: &[std::sync::Arc<tl_policy::Policy>],
        _family_policies: &[std::sync::Arc<tl_policy::FamilyPolicy>],
    ) -> Result<AdapterPolicyBoundary, AuthorizationAdapterError> {
        self.ensure_domain(subject)?;
        Ok(AdapterPolicyBoundary::default())
    }

    fn proposed_scope(
        &self,
        subject: &AuthorizationSubject,
    ) -> Result<Option<AuthorizationGrantScope>, AuthorizationAdapterError>;

    fn scope_covers(
        &self,
        scope: &AuthorizationGrantScope,
        subject: &AuthorizationSubject,
    ) -> Result<bool, AuthorizationAdapterError>;

    fn pure_findings(
        &self,
        subject: &AuthorizationSubject,
    ) -> Result<Vec<AuthorizationFinding>, AuthorizationAdapterError> {
        self.ensure_domain(subject)?;
        Ok(Vec::new())
    }

    async fn stateful_findings(
        &self,
        subject: &AuthorizationSubject,
    ) -> Result<Vec<AuthorizationFinding>, AuthorizationAdapterError> {
        self.ensure_domain(subject)?;
        Ok(Vec::new())
    }

    async fn precommit(
        &self,
        subject: &AuthorizationSubject,
    ) -> Result<(), AuthorizationAdapterError> {
        self.ensure_domain(subject)
    }

    fn domain_evidence(
        &self,
        subject: &AuthorizationSubject,
    ) -> Result<AuthorizationDomainEvidence, AuthorizationAdapterError>;

    fn ensure_domain(
        &self,
        subject: &AuthorizationSubject,
    ) -> Result<(), AuthorizationAdapterError> {
        if subject.domain() == self.domain() {
            Ok(())
        } else {
            Err(AuthorizationAdapterError::DomainMismatch)
        }
    }
}

#[derive(Debug, Default)]
pub struct ContentAdapter;

impl sealed::Sealed for ContentAdapter {}

#[async_trait]
impl AuthorizationAdapter for ContentAdapter {
    fn domain(&self) -> AuthorizationDomain {
        AuthorizationDomain::Content
    }

    fn capability(
        &self,
        subject: &AuthorizationSubject,
    ) -> Result<AuthorizationCapabilityId, AuthorizationAdapterError> {
        self.ensure_domain(subject)?;
        AuthorizationCapabilityId::parse("action:content_output")
            .map_err(|message| AuthorizationAdapterError::Invalid(message.into()))
    }

    fn proposed_scope(
        &self,
        subject: &AuthorizationSubject,
    ) -> Result<Option<AuthorizationGrantScope>, AuthorizationAdapterError> {
        self.ensure_domain(subject)?;
        Ok(None)
    }

    fn scope_covers(
        &self,
        scope: &AuthorizationGrantScope,
        subject: &AuthorizationSubject,
    ) -> Result<bool, AuthorizationAdapterError> {
        self.ensure_domain(subject)?;
        Ok(matches!(scope, AuthorizationGrantScope::Action(_)))
    }

    fn domain_evidence(
        &self,
        subject: &AuthorizationSubject,
    ) -> Result<AuthorizationDomainEvidence, AuthorizationAdapterError> {
        self.ensure_domain(subject)?;
        serialize_evidence(subject).map(AuthorizationDomainEvidence::Content)
    }
}

#[derive(Debug, Default)]
pub struct ToolAdapter;

impl sealed::Sealed for ToolAdapter {}

#[async_trait]
impl AuthorizationAdapter for ToolAdapter {
    fn domain(&self) -> AuthorizationDomain {
        AuthorizationDomain::Tool
    }

    fn capability(
        &self,
        subject: &AuthorizationSubject,
    ) -> Result<AuthorizationCapabilityId, AuthorizationAdapterError> {
        let AuthorizationSubject::Tool { tool_identity, .. } = subject else {
            return Err(AuthorizationAdapterError::DomainMismatch);
        };
        AuthorizationCapabilityId::parse(format!(
            "tool:{}/{}",
            tool_identity.server_id.to_ascii_lowercase(),
            tool_identity.tool_name.to_ascii_lowercase()
        ))
        .map_err(|message| AuthorizationAdapterError::Invalid(message.into()))
    }

    fn policy_boundary(
        &self,
        principal_id: &str,
        subject: &AuthorizationSubject,
        _policies: &[std::sync::Arc<tl_policy::Policy>],
        family_policies: &[std::sync::Arc<tl_policy::FamilyPolicy>],
    ) -> Result<AdapterPolicyBoundary, AuthorizationAdapterError> {
        self.ensure_domain(subject)?;
        let outcome = tl_engine::evaluate_tool_policies(
            principal_id,
            subject,
            family_policies.iter().map(std::convert::AsRef::as_ref),
        )
        .map_err(|error| AuthorizationAdapterError::Invalid(error.to_string()))?;
        Ok(AdapterPolicyBoundary {
            findings: outcome.findings,
            requirements: outcome.requirements,
            policy_versions: outcome.policy_versions,
        })
    }

    fn proposed_scope(
        &self,
        subject: &AuthorizationSubject,
    ) -> Result<Option<AuthorizationGrantScope>, AuthorizationAdapterError> {
        let AuthorizationSubject::Tool {
            operation,
            tool_identity,
            parameters,
            side_effect,
            ..
        } = subject
        else {
            return Err(AuthorizationAdapterError::DomainMismatch);
        };
        Ok(Some(AuthorizationGrantScope::Action(ActionGrantScope {
            operations: vec![operation.clone()],
            side_effects: vec![*side_effect],
            server_id: Some(tool_identity.server_id.clone()),
            tool_name: Some(tool_identity.tool_name.clone()),
            schema_hash: Some(tool_identity.schema_hash.clone()),
            parameters: Some(parameters.clone()),
            allowed_destinations: Vec::new(),
            maximum_data_confidentiality: None,
            minimum_source_trust: None,
        })))
    }

    fn scope_covers(
        &self,
        scope: &AuthorizationGrantScope,
        subject: &AuthorizationSubject,
    ) -> Result<bool, AuthorizationAdapterError> {
        let Some(AuthorizationGrantScope::Action(required)) = self.proposed_scope(subject)? else {
            return Ok(false);
        };
        let AuthorizationGrantScope::Action(granted) = scope else {
            return Ok(false);
        };
        Ok(action_scope_covers(granted, &required))
    }

    fn domain_evidence(
        &self,
        subject: &AuthorizationSubject,
    ) -> Result<AuthorizationDomainEvidence, AuthorizationAdapterError> {
        self.ensure_domain(subject)?;
        serialize_evidence(subject).map(AuthorizationDomainEvidence::Tool)
    }
}

#[derive(Debug, Default)]
pub struct FinancialAdapter;

impl sealed::Sealed for FinancialAdapter {}

#[async_trait]
impl AuthorizationAdapter for FinancialAdapter {
    fn domain(&self) -> AuthorizationDomain {
        AuthorizationDomain::Financial
    }

    fn capability(
        &self,
        subject: &AuthorizationSubject,
    ) -> Result<AuthorizationCapabilityId, AuthorizationAdapterError> {
        let AuthorizationSubject::Financial { action, .. } = subject else {
            return Err(AuthorizationAdapterError::DomainMismatch);
        };
        AuthorizationCapabilityId::parse(format!(
            "financial:{}",
            action.operation.to_ascii_lowercase().replace(' ', "_")
        ))
        .map_err(|message| AuthorizationAdapterError::Invalid(message.into()))
    }

    fn proposed_scope(
        &self,
        subject: &AuthorizationSubject,
    ) -> Result<Option<AuthorizationGrantScope>, AuthorizationAdapterError> {
        let AuthorizationSubject::Financial { action, .. } = subject else {
            return Err(AuthorizationAdapterError::DomainMismatch);
        };
        Ok(Some(AuthorizationGrantScope::Financial(
            FinancialGrantScope {
                action_kinds: vec![action.kind],
                operation: Some(action.operation.clone()),
                rail: Some(action.rail),
                currency: Some(action.amount.currency.clone()),
                maximum_amount_minor: Some(action.amount.amount_minor),
                counterparties: action
                    .counterparty
                    .as_ref()
                    .map(|counterparty| vec![counterparty.id.clone()])
                    .unwrap_or_default(),
                x402_hosts: Vec::new(),
                x402_resources: Vec::new(),
                x402_networks: Vec::new(),
                x402_assets: Vec::new(),
                x402_payees: Vec::new(),
                required_preconditions: Vec::new(),
            },
        )))
    }

    fn scope_covers(
        &self,
        scope: &AuthorizationGrantScope,
        subject: &AuthorizationSubject,
    ) -> Result<bool, AuthorizationAdapterError> {
        let AuthorizationSubject::Financial { action, .. } = subject else {
            return Err(AuthorizationAdapterError::DomainMismatch);
        };
        let AuthorizationGrantScope::Financial(scope) = scope else {
            return Ok(false);
        };
        Ok(
            (scope.action_kinds.is_empty() || scope.action_kinds.contains(&action.kind))
                && scope
                    .operation
                    .as_ref()
                    .map_or(true, |value| value == &action.operation)
                && scope.rail.map_or(true, |value| value == action.rail)
                && scope
                    .currency
                    .as_ref()
                    .map_or(true, |value| value == &action.amount.currency)
                && scope
                    .maximum_amount_minor
                    .map_or(true, |maximum| action.amount.amount_minor <= maximum)
                && (scope.counterparties.is_empty()
                    || action.counterparty.as_ref().is_some_and(|counterparty| {
                        scope.counterparties.contains(&counterparty.id)
                    })),
        )
    }

    fn domain_evidence(
        &self,
        subject: &AuthorizationSubject,
    ) -> Result<AuthorizationDomainEvidence, AuthorizationAdapterError> {
        self.ensure_domain(subject)?;
        serialize_evidence(subject).map(AuthorizationDomainEvidence::Financial)
    }
}

#[derive(Debug, Default)]
pub struct AuthorizationAdapterRegistry {
    content: ContentAdapter,
    tool: ToolAdapter,
    financial: FinancialAdapter,
}

impl AuthorizationAdapterRegistry {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn for_subject(&self, subject: &AuthorizationSubject) -> &dyn AuthorizationAdapter {
        match subject {
            AuthorizationSubject::Content { .. } => &self.content,
            AuthorizationSubject::Tool { .. } => &self.tool,
            AuthorizationSubject::Financial { .. } => &self.financial,
        }
    }

    pub fn for_domain(&self, domain: AuthorizationDomain) -> &dyn AuthorizationAdapter {
        match domain {
            AuthorizationDomain::Content => &self.content,
            AuthorizationDomain::Tool => &self.tool,
            AuthorizationDomain::Financial => &self.financial,
        }
    }
}

fn subject_fingerprint(
    workspace_id: &str,
    environment_id: &str,
    principal_id: &str,
    subject: &AuthorizationSubject,
) -> Result<String, AuthorizationAdapterError> {
    let value = serde_json::json!({
        "version": 1,
        "workspace_id": workspace_id,
        "environment_id": environment_id,
        "principal_id": principal_id,
        "subject": subject,
    });
    let canonical = super::canonical_json(&value);
    let digest = Sha256::digest(canonical.as_bytes());
    Ok(format!(
        "sha256:v1:{}",
        digest
            .iter()
            .map(|byte| format!("{byte:02x}"))
            .collect::<String>()
    ))
}

fn serialize_evidence(
    subject: &AuthorizationSubject,
) -> Result<serde_json::Value, AuthorizationAdapterError> {
    serde_json::to_value(subject)
        .map_err(|error| AuthorizationAdapterError::Invalid(error.to_string()))
}

fn action_scope_covers(granted: &ActionGrantScope, required: &ActionGrantScope) -> bool {
    (granted.operations.is_empty()
        || required
            .operations
            .iter()
            .all(|operation| granted.operations.contains(operation)))
        && (granted.side_effects.is_empty()
            || required
                .side_effects
                .iter()
                .all(|effect| granted.side_effects.contains(effect)))
        && granted
            .server_id
            .as_ref()
            .map_or(true, |value| required.server_id.as_ref() == Some(value))
        && granted
            .tool_name
            .as_ref()
            .map_or(true, |value| required.tool_name.as_ref() == Some(value))
        && granted
            .schema_hash
            .as_ref()
            .map_or(true, |value| required.schema_hash.as_ref() == Some(value))
        && granted
            .parameters
            .as_ref()
            .map_or(true, |value| required.parameters.as_ref() == Some(value))
}

#[cfg(test)]
mod tests {
    use super::*;
    use tl_core::{FinancialAction, FinancialActionKind, FinancialRail, MoneyAmount};

    #[test]
    fn registry_dispatches_tagged_subject_without_downcasting() {
        let registry = AuthorizationAdapterRegistry::new();
        let subject = AuthorizationSubject::Financial {
            action_id: "fin-1".into(),
            action: FinancialAction {
                id: Some("fin-1".into()),
                kind: FinancialActionKind::Refund,
                operation: "refund".into(),
                principal_id: "agent-1".into(),
                amount: MoneyAmount {
                    amount_minor: 500,
                    currency: "USD".into(),
                },
                counterparty: None,
                rail: FinancialRail::Internal,
                memo: None,
                metadata: serde_json::Value::Null,
            },
        };
        assert_eq!(
            registry.for_subject(&subject).domain(),
            AuthorizationDomain::Financial
        );
        assert!(registry
            .for_domain(AuthorizationDomain::Tool)
            .normalize(&subject)
            .is_err());
    }
}
