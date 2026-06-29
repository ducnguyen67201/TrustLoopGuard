//! Value-limit checker: caps a tool parameter's numeric value against the
//! registry-declared `ParamLimit`.
//!
//! Where `parameter_auth` answers "did this value come from an allowed
//! source", this checker answers "is this value within the allowed
//! bounds" — the money cap (`issue_refund.amount <= 500`). It reads the
//! `ParamLimit` already resolved onto the event by the tool metadata
//! registry and is **pure**: no I/O, no clock. That purity is also the
//! boundary of what it can do — a per-call cap needs only the event in
//! hand, whereas a rate/quota ("N per day") needs state and a clock and so
//! is deliberately out of scope here.
//!
//! Conservative money posture: a limit is configured precisely because the
//! parameter is dangerous, so anything we cannot verify against it
//! escalates rather than passes. A non-integer or non-numeric value (where
//! an integer in the tool's minor units is expected) is *unverifiable*, not
//! clean — it escalates. An absent parameter supplied nothing to cap and is
//! skipped, mirroring `parameter_auth`.

use tl_core::{GuardEvent, LimitAction, ParamLimit, ToolResolution, Verdict};

use super::param_auth::lookup_param;
use crate::event_pipeline::{Checker, CheckerFinding};

pub const VALUE_LIMIT_CHECKER_ID: &str = "value_limit";

const FAILURE_OVER_LIMIT: &str = "amount_over_limit";
const FAILURE_UNDER_LIMIT: &str = "amount_under_limit";
const FAILURE_UNVERIFIABLE: &str = "unverifiable_amount";
const HARM_AUTHORIZATION: &str = "authorization";

/// Deterministic value-limit authorization over the resolved tool's
/// registry metadata:
///
/// - **parameter_value.{path}**: every parameter carrying a `ParamLimit`
///   must hold an integer within `[min, max]`. Over `max` or under `min`
///   yields the limit's `on_breach` verdict (default `Block`); a value that
///   is present but not an integer escalates as unverifiable.
/// - Unregistered or unresolved tools emit nothing — unknown-tool
///   conservatism is the flow/parameter checkers' job, and a value cap has
///   no bound to test without resolved metadata.
pub struct ValueLimitChecker;

impl Checker for ValueLimitChecker {
    fn id(&self) -> &'static str {
        VALUE_LIMIT_CHECKER_ID
    }

    fn check(&self, event: &GuardEvent) -> Vec<CheckerFinding> {
        let Some(ToolResolution::Resolved { metadata }) = &event.resolution else {
            return vec![];
        };

        let mut findings = Vec::new();
        for spec in &metadata.params {
            let Some(limit) = &spec.limit else {
                continue;
            };
            let Some(value) = lookup_param(&event.action.parameters, &spec.path) else {
                // Nothing was supplied to cap — mirror parameter_auth and skip.
                continue;
            };

            // A configured limit expects an integer in the tool's minor
            // units. A non-integer value cannot be verified against the
            // bound; for a money parameter that is a reason to escalate,
            // never to pass.
            let Some(amount) = value.as_i64() else {
                findings.push(unverifiable_finding(&spec.path));
                continue;
            };

            if let Some(finding) = bound_finding(&spec.path, amount, limit) {
                findings.push(finding);
            }
        }
        findings
    }
}

/// One finding per breached bound, or `None` when the value is within
/// `[min, max]`. `max` is checked before `min`; a single value can only sit
/// on one side of a well-formed range, so at most one finding is produced.
fn bound_finding(path: &str, amount: i64, limit: &ParamLimit) -> Option<CheckerFinding> {
    // `Block` is the safe default for money movement; `Escalate` routes to a human.
    let verdict = match limit.on_breach {
        LimitAction::Block => Verdict::Block,
        LimitAction::Escalate => Verdict::Escalate,
    };

    if let Some(max) = limit.max {
        if amount > max {
            return Some(CheckerFinding {
                checker_id: VALUE_LIMIT_CHECKER_ID.to_string(),
                verdict: Some(verdict),
                reason: format!("parameter '{path}' value {amount} exceeds the maximum of {max}"),
                violated_rule: Some(format!("parameter_value.{path}")),
                remediation: Some(format!(
                    "reduce '{path}' to at most {max}, or raise the tool's registered limit"
                )),
                source_chain: vec![],
                risk_source: None,
                failure_mode: Some(FAILURE_OVER_LIMIT.to_string()),
                harm_class: Some(HARM_AUTHORIZATION.to_string()),
            });
        }
    }

    if let Some(min) = limit.min {
        if amount < min {
            return Some(CheckerFinding {
                checker_id: VALUE_LIMIT_CHECKER_ID.to_string(),
                verdict: Some(verdict),
                reason: format!("parameter '{path}' value {amount} is below the minimum of {min}"),
                violated_rule: Some(format!("parameter_value.{path}")),
                remediation: Some(format!(
                    "raise '{path}' to at least {min}, or lower the tool's registered limit"
                )),
                source_chain: vec![],
                risk_source: None,
                failure_mode: Some(FAILURE_UNDER_LIMIT.to_string()),
                harm_class: Some(HARM_AUTHORIZATION.to_string()),
            });
        }
    }

    None
}

fn unverifiable_finding(path: &str) -> CheckerFinding {
    CheckerFinding {
        checker_id: VALUE_LIMIT_CHECKER_ID.to_string(),
        // Unverifiable is never silently allowed: a configured money cap
        // we cannot evaluate routes to a human.
        verdict: Some(Verdict::Escalate),
        reason: format!(
            "parameter '{path}' has a value limit but its value is not an integer; \
             the limit cannot be verified"
        ),
        violated_rule: Some(format!("parameter_value.{path}")),
        remediation: Some(format!(
            "send '{path}' as an integer in the tool's minor units (e.g. cents)"
        )),
        source_chain: vec![],
        risk_source: None,
        failure_mode: Some(FAILURE_UNVERIFIABLE.to_string()),
        harm_class: Some(HARM_AUTHORIZATION.to_string()),
    }
}

#[cfg(test)]
mod tests {
    use super::super::test_support::{event, source};
    use super::*;
    use tl_core::{
        EventKind, Labels, Origin, ParamLimit, ParamRole, ParamSpec, ProvenanceMap,
        SideEffectClass, ToolMetadata, ToolResolution,
    };

    /// A `payment.transfer`-style tool whose `amount` param carries `limit`.
    fn metadata(limit: Option<ParamLimit>) -> ToolMetadata {
        ToolMetadata {
            tool: "issue_refund".into(),
            side_effect: SideEffectClass::ApiMutation,
            reversible: false,
            params: vec![ParamSpec {
                path: "amount".into(),
                role: ParamRole::AuthorityBearing,
                allowed_sources: vec![],
                limit,
            }],
            approval: None,
            sandbox_hint: None,
        }
    }

    /// A resolved `tool.call.proposed` event for `issue_refund` with the
    /// given `amount` JSON value and the given registered limit.
    fn resolved_event(amount: serde_json::Value, limit: Option<ParamLimit>) -> GuardEvent {
        let mut event = event(
            EventKind::ToolCallProposed,
            Some(SideEffectClass::ApiMutation),
            vec![source("src.user", Origin::User, Labels::default())],
            ProvenanceMap::default(),
        );
        event.action.operation = "issue_refund".into();
        event.action.parameters = serde_json::json!({ "amount": amount });
        event.resolution = Some(ToolResolution::Resolved {
            metadata: metadata(limit),
        });
        event
    }

    fn max_limit(max: i64) -> ParamLimit {
        ParamLimit {
            max: Some(max),
            min: None,
            on_breach: LimitAction::Block,
        }
    }

    #[test]
    fn blocks_when_amount_exceeds_max() {
        let event = resolved_event(serde_json::json!(9999), Some(max_limit(500)));

        let findings = ValueLimitChecker.check(&event);
        assert_eq!(findings.len(), 1);
        let finding = &findings[0];
        assert_eq!(finding.verdict, Some(Verdict::Block));
        assert_eq!(
            finding.violated_rule.as_deref(),
            Some("parameter_value.amount")
        );
        assert_eq!(finding.failure_mode.as_deref(), Some("amount_over_limit"));
        assert_eq!(finding.harm_class.as_deref(), Some("authorization"));
        assert!(finding.reason.contains("9999"));
        assert!(finding.reason.contains("500"));
    }

    #[test]
    fn allows_amount_at_max_boundary() {
        let event = resolved_event(serde_json::json!(500), Some(max_limit(500)));
        assert!(ValueLimitChecker.check(&event).is_empty());
    }

    #[test]
    fn allows_amount_under_max() {
        let event = resolved_event(serde_json::json!(1), Some(max_limit(500)));
        assert!(ValueLimitChecker.check(&event).is_empty());
    }

    #[test]
    fn blocks_when_amount_below_min() {
        let limit = ParamLimit {
            max: None,
            min: Some(100),
            on_breach: LimitAction::Block,
        };
        let event = resolved_event(serde_json::json!(10), Some(limit));

        let findings = ValueLimitChecker.check(&event);
        assert_eq!(findings.len(), 1);
        assert_eq!(findings[0].verdict, Some(Verdict::Block));
        assert_eq!(
            findings[0].failure_mode.as_deref(),
            Some("amount_under_limit")
        );
    }

    #[test]
    fn allows_amount_at_min_boundary() {
        let limit = ParamLimit {
            max: None,
            min: Some(100),
            on_breach: LimitAction::Block,
        };
        let event = resolved_event(serde_json::json!(100), Some(limit));
        assert!(ValueLimitChecker.check(&event).is_empty());
    }

    #[test]
    fn escalates_when_value_is_not_an_integer() {
        for value in [
            serde_json::json!("lots"),
            serde_json::json!(1.5),
            serde_json::json!(500.0),
            serde_json::json!(null),
            serde_json::json!({ "nested": 1 }),
        ] {
            let event = resolved_event(value.clone(), Some(max_limit(500)));
            let findings = ValueLimitChecker.check(&event);
            assert_eq!(
                findings.len(),
                1,
                "value {value:?} should yield one finding"
            );
            assert_eq!(
                findings[0].verdict,
                Some(Verdict::Escalate),
                "value {value:?}"
            );
            assert_eq!(
                findings[0].failure_mode.as_deref(),
                Some("unverifiable_amount")
            );
        }
    }

    #[test]
    fn on_breach_escalate_yields_escalate_not_block() {
        let limit = ParamLimit {
            max: Some(500),
            min: None,
            on_breach: LimitAction::Escalate,
        };
        let event = resolved_event(serde_json::json!(9999), Some(limit));

        let findings = ValueLimitChecker.check(&event);
        assert_eq!(findings.len(), 1);
        assert_eq!(findings[0].verdict, Some(Verdict::Escalate));
        assert_eq!(
            findings[0].failure_mode.as_deref(),
            Some("amount_over_limit")
        );
    }

    #[test]
    fn param_without_limit_is_ignored() {
        let event = resolved_event(serde_json::json!(9999), None);
        assert!(ValueLimitChecker.check(&event).is_empty());
    }

    #[test]
    fn absent_param_is_skipped() {
        let mut event = resolved_event(serde_json::json!(9999), Some(max_limit(500)));
        // Limit is on `amount`, but the call supplies a different key.
        event.action.parameters = serde_json::json!({ "note": "no amount here" });
        assert!(ValueLimitChecker.check(&event).is_empty());
    }

    #[test]
    fn unregistered_tool_emits_nothing() {
        let mut event = resolved_event(serde_json::json!(9999), Some(max_limit(500)));
        event.resolution = Some(ToolResolution::Unregistered);
        assert!(ValueLimitChecker.check(&event).is_empty());

        event.resolution = Some(ToolResolution::ResolutionFailed);
        assert!(ValueLimitChecker.check(&event).is_empty());

        event.resolution = None;
        assert!(ValueLimitChecker.check(&event).is_empty());
    }

    #[test]
    fn negative_amount_below_zero_min_is_blocked() {
        // Guards against a sign-flip bug: a negative refund must not pass a
        // `min: 0` floor.
        let limit = ParamLimit {
            max: Some(500),
            min: Some(0),
            on_breach: LimitAction::Block,
        };
        let event = resolved_event(serde_json::json!(-100), Some(limit));

        let findings = ValueLimitChecker.check(&event);
        assert_eq!(findings.len(), 1);
        assert_eq!(
            findings[0].failure_mode.as_deref(),
            Some("amount_under_limit")
        );
    }
}
