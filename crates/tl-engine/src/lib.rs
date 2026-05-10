//! Synchronous hot-path engine. The fast path runs in microseconds; LLM
//! judges and other async work belong elsewhere.

use std::time::Instant;

use tl_core::{new_trace_id, CheckRequest, Decision, TriggeredPolicy, Verdict};
use tl_policy::Policy;

pub mod engine_match;

pub struct Engine {
    policies: Vec<Policy>,
}

impl Engine {
    pub fn new(policies: Vec<Policy>) -> Self {
        Self { policies }
    }

    pub fn empty() -> Self {
        Self::new(vec![])
    }

    /// Synchronous check. Designed to be called on the hot path.
    pub fn check(&self, req: &CheckRequest) -> Decision {
        let start = Instant::now();
        let trace_id = req
            .trace_id
            .clone()
            .unwrap_or_else(new_trace_id);

        let mut triggered: Vec<TriggeredPolicy> = vec![];
        for policy in &self.policies {
            if engine_match::policy_matches(policy, req) {
                triggered.push(TriggeredPolicy {
                    id: policy.id.clone(),
                    severity: policy.severity,
                    reason: format!("policy `{}` matched", policy.id),
                });
            }
        }

        let (verdict, safe_output, reason) = if triggered.is_empty() {
            (Verdict::Allow, None, "no policies triggered".to_string())
        } else {
            let first = &self.policies[0];
            let action = first.action;
            let v = match action {
                tl_policy::Action::Allow => Verdict::Allow,
                tl_policy::Action::Block => Verdict::Block,
                tl_policy::Action::Rewrite => Verdict::Rewrite,
                tl_policy::Action::Escalate => Verdict::Escalate,
            };
            let safe = first.rewrite.clone();
            (v, safe, format!("{} policy(ies) triggered", triggered.len()))
        };

        Decision {
            trace_id,
            verdict,
            reason,
            triggered_policies: triggered,
            safe_output,
            latency_ms: start.elapsed().as_millis() as u64,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tl_core::Channel;

    #[test]
    fn empty_engine_allows() {
        let eng = Engine::empty();
        let req = CheckRequest {
            agent_id: "a".into(),
            channel: Channel::Chat,
            input: "hi".into(),
            proposed_output: "hello".into(),
            policies: vec![],
            context: serde_json::Value::Null,
            trace_id: None,
        };
        let d = eng.check(&req);
        assert_eq!(d.verdict, Verdict::Allow);
    }
}
