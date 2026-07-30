use std::time::Instant;

use tl_core::{
    AgentProfile, AuthorizationEffect, Severity, Tier, TierResult, TierStatus, TriggeredPolicy,
};

use super::judge_runtime::JudgeResult;
use crate::pipeline::{BlockSignal, TierOutput};

pub(super) fn aggregate(
    start: Instant,
    profile: &AgentProfile,
    hallu: JudgeResult,
    tone: JudgeResult,
    auth: JudgeResult,
) -> TierOutput {
    let mut reasons: Vec<TriggeredPolicy> = vec![];
    let mut block: Option<BlockSignal> = None;

    if let Some(effect) = interpret_hallucination(&hallu) {
        apply_hallucination_effect(effect, &mut reasons, &mut block);
    }

    if let Some(effect) = interpret_authority(&auth) {
        apply_authority_effect(effect, &mut reasons, &mut block);
    }

    if let Some(effect) = interpret_tone(&tone, profile) {
        apply_tone_effect(effect, &mut reasons, &mut block);
    }

    TierOutput {
        result: TierResult {
            tier: Tier::Llm,
            status: TierStatus::Completed,
            reasons,
            elapsed_ms: start.elapsed().as_millis() as u64,
        },
        block,
    }
}

enum JudgeEffect {
    Permit,
    DenyGrounded(Vec<String>),
    Transform(String, Option<String>),
    Defer(String),
}

fn apply_hallucination_effect(
    effect: JudgeEffect,
    reasons: &mut Vec<TriggeredPolicy>,
    block: &mut Option<BlockSignal>,
) {
    match effect {
        JudgeEffect::Permit => {}
        JudgeEffect::DenyGrounded(violations) => {
            let reason = format!("hallucination: {}", violations.join("; "));
            reasons.push(TriggeredPolicy {
                id: "tl:hallucination".into(),
                severity: Severity::High,
                reason: reason.clone(),
            });
            block.get_or_insert(BlockSignal {
                effect: AuthorizationEffect::Deny,
                reason,
                safe_output: None,
            });
        }
        JudgeEffect::Defer(reason) => {
            reasons.push(TriggeredPolicy {
                id: "tl:hallucination_unavailable".into(),
                severity: Severity::Medium,
                reason: reason.clone(),
            });
            block.get_or_insert(BlockSignal {
                effect: AuthorizationEffect::Defer,
                reason,
                safe_output: None,
            });
        }
        JudgeEffect::Transform(_, _) => {}
    }
}

fn apply_authority_effect(
    effect: JudgeEffect,
    reasons: &mut Vec<TriggeredPolicy>,
    block: &mut Option<BlockSignal>,
) {
    match effect {
        JudgeEffect::Permit => {}
        JudgeEffect::DenyGrounded(violations) => {
            let reason = format!("authority violation: {}", violations.join("; "));
            reasons.push(TriggeredPolicy {
                id: "tl:authority".into(),
                severity: Severity::High,
                reason: reason.clone(),
            });
            block.get_or_insert(BlockSignal {
                effect: AuthorizationEffect::Deny,
                reason,
                safe_output: None,
            });
        }
        JudgeEffect::Defer(reason) => {
            reasons.push(TriggeredPolicy {
                id: "tl:authority_unavailable".into(),
                severity: Severity::Medium,
                reason: reason.clone(),
            });
            block.get_or_insert(BlockSignal {
                effect: AuthorizationEffect::Defer,
                reason,
                safe_output: None,
            });
        }
        JudgeEffect::Transform(_, _) => {}
    }
}

fn apply_tone_effect(
    effect: JudgeEffect,
    reasons: &mut Vec<TriggeredPolicy>,
    block: &mut Option<BlockSignal>,
) {
    match effect {
        JudgeEffect::Permit => {}
        JudgeEffect::Transform(reason, fallback) => {
            reasons.push(TriggeredPolicy {
                id: "tl:tone".into(),
                severity: Severity::Low,
                reason: reason.clone(),
            });
            block.get_or_insert(BlockSignal {
                effect: AuthorizationEffect::Transform,
                reason,
                safe_output: fallback,
            });
        }
        JudgeEffect::Defer(reason) => {
            reasons.push(TriggeredPolicy {
                id: "tl:tone_unavailable".into(),
                severity: Severity::Low,
                reason: reason.clone(),
            });
            block.get_or_insert(BlockSignal {
                effect: AuthorizationEffect::Defer,
                reason,
                safe_output: None,
            });
        }
        JudgeEffect::DenyGrounded(_) => {}
    }
}

fn interpret_hallucination(judge: &JudgeResult) -> Option<JudgeEffect> {
    match judge {
        JudgeResult::Skipped => None,
        JudgeResult::Err(error) => {
            Some(JudgeEffect::Defer(format!("hallucination judge: {error}")))
        }
        JudgeResult::Ok(out) => {
            let grounded = match required_bool(&out.json, "grounded", "hallucination") {
                Ok(grounded) => grounded,
                Err(reason) => return Some(JudgeEffect::Defer(reason)),
            };
            if grounded {
                Some(JudgeEffect::Permit)
            } else {
                Some(JudgeEffect::DenyGrounded(json_string_array(
                    &out.json["violations"],
                )))
            }
        }
    }
}

fn interpret_authority(judge: &JudgeResult) -> Option<JudgeEffect> {
    match judge {
        JudgeResult::Skipped => None,
        JudgeResult::Err(error) => Some(JudgeEffect::Defer(format!("authority judge: {error}"))),
        JudgeResult::Ok(out) => {
            let within = match required_bool(&out.json, "within_authority", "authority") {
                Ok(within) => within,
                Err(reason) => return Some(JudgeEffect::Defer(reason)),
            };
            if within {
                Some(JudgeEffect::Permit)
            } else {
                Some(JudgeEffect::DenyGrounded(json_string_array(
                    &out.json["forbidden_promises"],
                )))
            }
        }
    }
}

fn interpret_tone(judge: &JudgeResult, _profile: &AgentProfile) -> Option<JudgeEffect> {
    match judge {
        JudgeResult::Skipped => None,
        JudgeResult::Err(error) => Some(JudgeEffect::Defer(format!("tone judge: {error}"))),
        JudgeResult::Ok(out) => {
            let matches = match required_bool(&out.json, "matches_target", "tone") {
                Ok(matches) => matches,
                Err(reason) => return Some(JudgeEffect::Defer(reason)),
            };
            if matches {
                return Some(JudgeEffect::Permit);
            }

            let issues = json_string_array(&out.json["issues"]);
            let detected = out.json["detected_tone"]
                .as_str()
                .unwrap_or("unknown")
                .to_string();
            let reason = if issues.is_empty() {
                format!("tone mismatch (detected: {detected})")
            } else {
                format!(
                    "tone mismatch (detected: {detected}): {}",
                    issues.join("; ")
                )
            };
            Some(JudgeEffect::Transform(reason, None))
        }
    }
}

fn required_bool(json: &serde_json::Value, field: &str, judge_name: &str) -> Result<bool, String> {
    json.get(field)
        .and_then(serde_json::Value::as_bool)
        .ok_or_else(|| format!("{judge_name} judge returned invalid `{field}` verdict"))
}

fn json_string_array(value: &serde_json::Value) -> Vec<String> {
    value
        .as_array()
        .map(|items| {
            items
                .iter()
                .filter_map(|value| value.as_str().map(String::from))
                .collect()
        })
        .unwrap_or_default()
}
