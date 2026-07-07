use tl_core::{PolicyDocument, PolicySummary};
use tl_policy::{Action, AnyPolicy, Policy};

pub(super) fn policy_document(policy: &Policy, source_yaml: &str, enabled: bool) -> PolicyDocument {
    PolicyDocument {
        id: policy.id.clone(),
        family: tl_core::PolicyFamily::Content,
        description: policy.description.clone(),
        severity: policy.severity,
        enabled,
        source_yaml: source_yaml.to_string(),
    }
}

pub(super) fn policy_summary(policy: &Policy, enabled: bool) -> PolicySummary {
    PolicySummary {
        id: policy.id.clone(),
        family: tl_core::PolicyFamily::Content,
        description: policy.description.clone(),
        severity: policy.severity,
        action: Some(policy_action(&policy.action)),
        enabled,
        owner_agent_id: policy.owner_agent_id.clone(),
    }
}

pub(crate) fn any_policy_document(
    policy: &AnyPolicy,
    source_yaml: &str,
    enabled: bool,
) -> PolicyDocument {
    PolicyDocument {
        id: policy.id().to_string(),
        family: policy.family(),
        description: policy.description().map(str::to_string),
        severity: policy.severity(),
        enabled,
        source_yaml: source_yaml.to_string(),
    }
}

pub(crate) fn any_policy_summary(policy: &AnyPolicy, enabled: bool) -> PolicySummary {
    PolicySummary {
        id: policy.id().to_string(),
        family: policy.family(),
        description: policy.description().map(str::to_string),
        severity: policy.severity(),
        action: policy.action().map(|action| policy_action(&action)),
        enabled,
        owner_agent_id: policy.owner_agent_id().map(str::to_string),
    }
}

pub(super) fn normalize_policy_ids(ids: Vec<String>) -> Result<Vec<String>, String> {
    let mut normalized = Vec::new();
    for id in ids {
        let id = id.trim();
        if id.is_empty() {
            return Err("policy ids must not be empty".into());
        }
        if !normalized.iter().any(|existing: &String| existing == id) {
            normalized.push(id.to_string());
        }
    }
    if normalized.is_empty() {
        return Err("at least one policy id is required".into());
    }
    Ok(normalized)
}

fn policy_action(action: &Action) -> String {
    match action {
        Action::Allow => "allow",
        Action::Block => "block",
        Action::Rewrite => "rewrite",
        Action::Escalate => "escalate",
    }
    .to_string()
}
