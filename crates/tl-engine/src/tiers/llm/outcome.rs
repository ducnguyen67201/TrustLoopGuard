use std::time::Instant;

use tl_core::{AgentProfile, Severity, Tier, TierResult, TierStatus, TriggeredPolicy, Verdict};

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

    if let Some(verdict) = interpret_hallucination(&hallu) {
        apply_hallucination_verdict(verdict, &mut reasons, &mut block);
    }

    if let Some(verdict) = interpret_authority(&auth) {
        apply_authority_verdict(verdict, &mut reasons, &mut block);
    }

    if let Some(verdict) = interpret_tone(&tone, profile) {
        apply_tone_verdict(verdict, &mut reasons, &mut block);
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

enum JudgeVerdict {
    Allow,
    BlockGrounded(Vec<String>),
    Revise(String, Option<String>),
    Escalate(String),
}

fn apply_hallucination_verdict(
    verdict: JudgeVerdict,
    reasons: &mut Vec<TriggeredPolicy>,
    block: &mut Option<BlockSignal>,
) {
    match verdict {
        JudgeVerdict::Allow => {}
        JudgeVerdict::BlockGrounded(violations) => {
            let reason = format!("hallucination: {}", violations.join("; "));
            reasons.push(TriggeredPolicy {
                id: "tl:hallucination".into(),
                severity: Severity::High,
                reason: reason.clone(),
            });
            block.get_or_insert(BlockSignal {
                verdict: Verdict::Block,
                reason,
                safe_output: None,
            });
        }
        JudgeVerdict::Escalate(reason) => {
            reasons.push(TriggeredPolicy {
                id: "tl:hallucination_unavailable".into(),
                severity: Severity::Medium,
                reason: reason.clone(),
            });
            block.get_or_insert(BlockSignal {
                verdict: Verdict::Escalate,
                reason,
                safe_output: None,
            });
        }
        JudgeVerdict::Revise(_, _) => {}
    }
}

fn apply_authority_verdict(
    verdict: JudgeVerdict,
    reasons: &mut Vec<TriggeredPolicy>,
    block: &mut Option<BlockSignal>,
) {
    match verdict {
        JudgeVerdict::Allow => {}
        JudgeVerdict::BlockGrounded(violations) => {
            let reason = format!("authority violation: {}", violations.join("; "));
            reasons.push(TriggeredPolicy {
                id: "tl:authority".into(),
                severity: Severity::High,
                reason: reason.clone(),
            });
            block.get_or_insert(BlockSignal {
                verdict: Verdict::Block,
                reason,
                safe_output: None,
            });
        }
        JudgeVerdict::Escalate(reason) => {
            reasons.push(TriggeredPolicy {
                id: "tl:authority_unavailable".into(),
                severity: Severity::Medium,
                reason: reason.clone(),
            });
            block.get_or_insert(BlockSignal {
                verdict: Verdict::Escalate,
                reason,
                safe_output: None,
            });
        }
        JudgeVerdict::Revise(_, _) => {}
    }
}

fn apply_tone_verdict(
    verdict: JudgeVerdict,
    reasons: &mut Vec<TriggeredPolicy>,
    block: &mut Option<BlockSignal>,
) {
    match verdict {
        JudgeVerdict::Allow => {}
        JudgeVerdict::Revise(reason, fallback) => {
            reasons.push(TriggeredPolicy {
                id: "tl:tone".into(),
                severity: Severity::Low,
                reason: reason.clone(),
            });
            block.get_or_insert(BlockSignal {
                verdict: Verdict::Rewrite,
                reason,
                safe_output: fallback,
            });
        }
        JudgeVerdict::Escalate(reason) => {
            reasons.push(TriggeredPolicy {
                id: "tl:tone_unavailable".into(),
                severity: Severity::Low,
                reason: reason.clone(),
            });
            block.get_or_insert(BlockSignal {
                verdict: Verdict::Escalate,
                reason,
                safe_output: None,
            });
        }
        JudgeVerdict::BlockGrounded(_) => {}
    }
}

fn interpret_hallucination(judge: &JudgeResult) -> Option<JudgeVerdict> {
    match judge {
        JudgeResult::Skipped => None,
        JudgeResult::Err(error) => Some(JudgeVerdict::Escalate(format!(
            "hallucination judge: {error}"
        ))),
        JudgeResult::Ok(out) => {
            let grounded = out.json["grounded"].as_bool().unwrap_or(true);
            if grounded {
                Some(JudgeVerdict::Allow)
            } else {
                Some(JudgeVerdict::BlockGrounded(json_string_array(
                    &out.json["violations"],
                )))
            }
        }
    }
}

fn interpret_authority(judge: &JudgeResult) -> Option<JudgeVerdict> {
    match judge {
        JudgeResult::Skipped => None,
        JudgeResult::Err(error) => {
            Some(JudgeVerdict::Escalate(format!("authority judge: {error}")))
        }
        JudgeResult::Ok(out) => {
            let within = out.json["within_authority"].as_bool().unwrap_or(true);
            if within {
                Some(JudgeVerdict::Allow)
            } else {
                Some(JudgeVerdict::BlockGrounded(json_string_array(
                    &out.json["forbidden_promises"],
                )))
            }
        }
    }
}

fn interpret_tone(judge: &JudgeResult, _profile: &AgentProfile) -> Option<JudgeVerdict> {
    match judge {
        JudgeResult::Skipped => None,
        JudgeResult::Err(error) => Some(JudgeVerdict::Escalate(format!("tone judge: {error}"))),
        JudgeResult::Ok(out) => {
            let matches = out.json["matches_target"].as_bool().unwrap_or(true);
            if matches {
                return Some(JudgeVerdict::Allow);
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
            Some(JudgeVerdict::Revise(reason, None))
        }
    }
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
