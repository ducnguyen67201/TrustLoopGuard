use regex::Regex;
use tl_core::CheckRequest;
use tl_policy::{MatchClause, Matcher, Policy};

pub fn policy_matches(policy: &Policy, req: &CheckRequest) -> bool {
    if !policy_scope_matches(policy, req) {
        return false;
    }
    match &policy.r#match {
        MatchClause::Single(m) => matcher_hits(m, &req.proposed_output),
        MatchClause::Any { any } => any.iter().any(|m| matcher_hits(m, &req.proposed_output)),
        MatchClause::All { all } => all.iter().all(|m| matcher_hits(m, &req.proposed_output)),
    }
}

fn policy_scope_matches(policy: &Policy, req: &CheckRequest) -> bool {
    channel_scope_matches(policy, req)
        && agent_scope_matches(policy, req)
        && domain_scope_matches(policy, req)
        && request_policy_filter_matches(policy, req)
}

fn channel_scope_matches(policy: &Policy, req: &CheckRequest) -> bool {
    policy.when.channels.is_empty() || policy.when.channels.contains(&req.channel)
}

fn agent_scope_matches(policy: &Policy, req: &CheckRequest) -> bool {
    policy.when.agents.is_empty() || policy.when.agents.contains(&req.agent_id)
}

fn domain_scope_matches(policy: &Policy, req: &CheckRequest) -> bool {
    if policy.when.domains.is_empty() {
        return true;
    }
    let domain = req.domain.as_deref().unwrap_or("customer_support");
    policy.when.domains.iter().any(|allowed| allowed == domain)
}

fn request_policy_filter_matches(policy: &Policy, req: &CheckRequest) -> bool {
    req.policies.is_empty() || req.policies.contains(&policy.id)
}

fn matcher_hits(m: &Matcher, text: &str) -> bool {
    match m {
        Matcher::Literal(s) => text.contains(s.as_str()),
        Matcher::Regex(pat) => Regex::new(pat).map(|re| re.is_match(text)).unwrap_or(false),
        // Semantic matching is opt-in and runs out-of-process. Static engine
        // returns false for now; the LLM judge layer will fill this in later.
        Matcher::Semantic(_) => false,
    }
}

#[cfg(test)]
mod tests {
    use tl_core::Channel;
    use tl_policy::load_str;

    use super::*;

    fn req() -> CheckRequest {
        CheckRequest {
            agent_id: "acme-support-v3".into(),
            channel: Channel::Chat,
            input: "can I get a refund?".into(),
            proposed_output: "I promise a guaranteed refund.".into(),
            domain: Some("customer_support".into()),
            policies: vec![],
            context: serde_json::Value::Null,
            trace_id: None,
        }
    }

    #[test]
    fn matches_canonical_scope_fields() {
        let policy = load_str(
            r#"
id: refund-promise
when:
  domains: [customer_support]
  agents: [acme-support-v3]
  channels: [chat]
match:
  literal: "guaranteed refund"
action: block
"#,
        )
        .expect("policy");

        assert!(policy_matches(&policy, &req()));
    }

    #[test]
    fn skips_agent_scope_mismatch() {
        let policy = load_str(
            r#"
id: refund-promise
when:
  agents: [other-agent]
match:
  literal: "guaranteed refund"
action: block
"#,
        )
        .expect("policy");

        assert!(!policy_matches(&policy, &req()));
    }

    #[test]
    fn skips_domain_scope_mismatch() {
        let policy = load_str(
            r#"
id: refund-promise
when:
  domains: [sales]
match:
  literal: "guaranteed refund"
action: block
"#,
        )
        .expect("policy");

        assert!(!policy_matches(&policy, &req()));
    }

    #[test]
    fn request_policy_filter_limits_tenant_policy_set() {
        let mut request = req();
        request.policies = vec!["other-policy".into()];
        let policy = load_str(
            r#"
id: refund-promise
match:
  literal: "guaranteed refund"
action: block
"#,
        )
        .expect("policy");

        assert!(!policy_matches(&policy, &request));
    }

    #[test]
    fn absent_domain_defaults_to_customer_support() {
        let mut request = req();
        request.domain = None;
        let policy = load_str(
            r#"
id: refund-promise
when:
  domains: [customer_support]
match:
  literal: "guaranteed refund"
action: block
"#,
        )
        .expect("policy");

        assert!(policy_matches(&policy, &request));
    }
}
