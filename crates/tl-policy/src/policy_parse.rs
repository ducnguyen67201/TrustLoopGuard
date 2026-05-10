use crate::policy_ast::Policy;

#[derive(Debug, thiserror::Error)]
pub enum PolicyError {
    #[error("yaml parse: {0}")]
    Yaml(#[from] serde_yaml::Error),
    #[error("policy validation: {0}")]
    Validation(String),
}

pub fn load_str(src: &str) -> Result<Policy, PolicyError> {
    let policy: Policy = serde_yaml::from_str(src)?;
    if policy.id.is_empty() {
        return Err(PolicyError::Validation("id is required".into()));
    }
    Ok(policy)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_minimal_policy() {
        let yaml = r#"
id: refund-promise
match:
  regex: "(?i)refund"
action: rewrite
rewrite: "I'll connect you with a teammate."
"#;
        let p = load_str(yaml).expect("parse");
        assert_eq!(p.id, "refund-promise");
    }
}
