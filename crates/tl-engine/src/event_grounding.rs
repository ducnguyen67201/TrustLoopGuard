use tl_core::{EventKind, GuardEvent, Severity, SignalEvidence, TriggeredPolicy, Verdict};
use tl_llm::{prompts::hallucination, JudgeKind};

use crate::context::{HandlerCtx, KnowledgeRetrievalRequest, KnowledgeSnippet};
use crate::tiers::llm::prompt_context::{
    extract_docs, format_knowledge_snippets, summarise_profile,
};

const PROVIDER_ID: &str = "knowledge_grounding";
const POLICY_ID: &str = "tl:knowledge_grounding";
const UNAVAILABLE_POLICY_ID: &str = "tl:knowledge_grounding_unavailable";

#[derive(Debug, Clone)]
pub struct EventGroundingOutcome {
    pub triggered: Vec<TriggeredPolicy>,
    pub verdict: Option<Verdict>,
    pub reason: Option<String>,
    pub safe_output: Option<String>,
    pub signal: Option<SignalEvidence>,
}

impl EventGroundingOutcome {
    pub fn empty() -> Self {
        Self {
            triggered: Vec::new(),
            verdict: None,
            reason: None,
            safe_output: None,
            signal: None,
        }
    }
}

pub async fn evaluate_event_grounding(
    event: &GuardEvent,
    workspace_id: &str,
    ctx: &HandlerCtx,
) -> EventGroundingOutcome {
    if event.kind != EventKind::OutputProposed {
        return EventGroundingOutcome::empty();
    }
    if !ctx.llm.has_route(JudgeKind::Hallucination) {
        return EventGroundingOutcome::empty();
    }

    let Some(proposed_output) = output_text(event) else {
        return EventGroundingOutcome::empty();
    };

    let profile = match ctx
        .profile_resolver
        .resolve(workspace_id, &event.principal.agent_id)
        .await
    {
        Some(profile) => profile,
        None => return EventGroundingOutcome::empty(),
    };
    if profile.knowledge_sources.is_empty() {
        return EventGroundingOutcome::empty();
    }

    let input = input_text(event);
    let source_ids = profile
        .knowledge_sources
        .iter()
        .map(|source| source.kb_id.clone())
        .collect::<Vec<_>>();
    let snippets = ctx
        .knowledge
        .retrieve(KnowledgeRetrievalRequest {
            workspace_id: workspace_id.to_string(),
            agent_id: event.principal.agent_id.clone(),
            source_ids,
            input: input.clone(),
            proposed_output: proposed_output.to_string(),
        })
        .await;

    if snippets.is_empty() {
        tracing::debug!(
            workspace_id,
            agent_id = %event.principal.agent_id,
            "event knowledge grounding skipped: no managed snippets returned"
        );
        return EventGroundingOutcome::empty();
    }

    let mut docs = extract_docs(&event.context);
    let knowledge_docs = format_knowledge_snippets(&snippets);
    let knowledge_chars = knowledge_docs.iter().map(String::len).sum::<usize>();
    let knowledge_est_tokens = estimate_prompt_tokens(knowledge_chars);
    let source_ids = source_ids_from_snippets(&snippets);
    tracing::info!(
        workspace_id,
        agent_id = %event.principal.agent_id,
        knowledge_snippet_count = snippets.len(),
        knowledge_chars,
        knowledge_est_tokens,
        knowledge_source_ids = %source_ids.join(","),
        "event knowledge grounding prompt contribution"
    );
    docs.extend(knowledge_docs);

    let prompt = hallucination::build(
        &summarise_profile(&profile),
        &docs.join("\n---\n"),
        &input,
        proposed_output,
    );
    match ctx
        .llm
        .judge(
            JudgeKind::Hallucination,
            workspace_id,
            &prompt,
            &hallucination::schema(),
        )
        .await
    {
        Ok(output) => outcome_from_hallucination_json(
            &output.json,
            snippets.len(),
            knowledge_est_tokens,
            &source_ids,
        ),
        Err(error) => {
            let reason = format!("knowledge grounding judge: {error}");
            EventGroundingOutcome {
                triggered: vec![TriggeredPolicy {
                    id: UNAVAILABLE_POLICY_ID.into(),
                    severity: Severity::Medium,
                    reason: reason.clone(),
                }],
                verdict: Some(Verdict::Escalate),
                reason: Some(reason.clone()),
                safe_output: None,
                signal: Some(SignalEvidence {
                    provider_id: PROVIDER_ID.into(),
                    message: format!(
                        "{reason}; snippets={}, estimated_prompt_tokens={knowledge_est_tokens}",
                        snippets.len()
                    ),
                    severity: Some(Severity::Medium),
                }),
            }
        }
    }
}

fn outcome_from_hallucination_json(
    json: &serde_json::Value,
    snippet_count: usize,
    knowledge_est_tokens: usize,
    source_ids: &[String],
) -> EventGroundingOutcome {
    let grounded = json["grounded"].as_bool().unwrap_or(true);
    let source_text = if source_ids.is_empty() {
        "none".to_string()
    } else {
        source_ids.join(",")
    };
    if grounded {
        return EventGroundingOutcome {
            signal: Some(SignalEvidence {
                provider_id: PROVIDER_ID.into(),
                message: format!(
                    "grounded against managed knowledge; snippets={snippet_count}, \
                     estimated_prompt_tokens={knowledge_est_tokens}, sources={source_text}"
                ),
                severity: None,
            }),
            ..EventGroundingOutcome::empty()
        };
    }

    let violations = json_string_array(&json["violations"]);
    let reason = if violations.is_empty() {
        "knowledge grounding: proposed output is not supported by managed knowledge".to_string()
    } else {
        format!("knowledge grounding: {}", violations.join("; "))
    };

    EventGroundingOutcome {
        triggered: vec![TriggeredPolicy {
            id: POLICY_ID.into(),
            severity: Severity::High,
            reason: reason.clone(),
        }],
        verdict: Some(Verdict::Block),
        reason: Some(reason.clone()),
        safe_output: None,
        signal: Some(SignalEvidence {
            provider_id: PROVIDER_ID.into(),
            message: format!(
                "{reason}; snippets={snippet_count}, estimated_prompt_tokens={knowledge_est_tokens}, \
                 sources={source_text}"
            ),
            severity: Some(Severity::High),
        }),
    }
}

fn output_text(event: &GuardEvent) -> Option<&str> {
    if event.kind != EventKind::OutputProposed {
        return None;
    }
    event.action.parameters.get("text")?.as_str()
}

fn input_text(event: &GuardEvent) -> String {
    event
        .context
        .get("input")
        .or_else(|| event.context.get("user_input"))
        .and_then(serde_json::Value::as_str)
        .unwrap_or_default()
        .to_string()
}

fn source_ids_from_snippets(snippets: &[KnowledgeSnippet]) -> Vec<String> {
    let mut source_ids = snippets
        .iter()
        .map(|snippet| snippet.source_id.clone())
        .collect::<Vec<_>>();
    source_ids.sort();
    source_ids.dedup();
    source_ids
}

fn estimate_prompt_tokens(chars: usize) -> usize {
    chars.div_ceil(4)
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
