use regex::Regex;
use tl_core::CheckRequest;
use tl_policy::{MatchClause, Matcher, Policy};

pub fn policy_matches(policy: &Policy, req: &CheckRequest) -> bool {
    if !policy.when.channel.is_empty() && !policy.when.channel.contains(&req.channel) {
        return false;
    }
    match &policy.r#match {
        MatchClause::Single(m) => matcher_hits(m, &req.proposed_output),
        MatchClause::Any { any } => any.iter().any(|m| matcher_hits(m, &req.proposed_output)),
        MatchClause::All { all } => all.iter().all(|m| matcher_hits(m, &req.proposed_output)),
    }
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
