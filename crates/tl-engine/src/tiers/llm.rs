//! Tier 3 — LLM judges. Real implementation as of PR 9.
//!
//! Three independent judges fan out via `tokio::join!`:
//! - **Hallucination** — every claim in the draft must be grounded in
//!   the agent's knowledge sources or in per-request docs.
//! - **Tone** — draft tone must match the agent's configured target and
//!   avoid the forbidden list.
//! - **Authority** — every commitment in the draft must be on the
//!   agent's `can_promise` list.
//!
//! Each judge is gated on the `LlmRouter` having a route configured for
//! its `JudgeKind`. If no route is configured for any of the three, the
//! tier returns `Skipped`. If the agent profile can't be resolved, the
//! tier also returns `Skipped` — judges need profile context to be
//! grounded.
//!
//! Verdict aggregation:
//! - Hallucination not grounded → Block
//! - Authority not within → Block
//! - Tone mismatch → Revise (or Escalate if no profile-level rewrite)
//! - First non-Allow signal wins.

use std::time::Instant;

use tl_core::{CheckRequest, DEFAULT_WORKSPACE_ID};
use tl_llm::prompts::{authority, hallucination, tone};
use tl_llm::JudgeKind;
use tokio_util::sync::CancellationToken;

use crate::context::{HandlerCtx, KnowledgeRetrievalRequest};
use crate::pipeline::TierOutput;

mod judge_runtime;
mod outcome;
pub(crate) mod prompt_context;
mod status;
#[cfg(test)]
mod tests;

use judge_runtime::{run_judges, JudgeOutcomes};
use outcome::aggregate;
use prompt_context::{bulleted, extract_docs, format_knowledge_snippets, summarise_profile};
use status::{cancelled, skipped};

pub async fn run(req: &CheckRequest, ctx: &HandlerCtx, cancel: CancellationToken) -> TierOutput {
    let start = Instant::now();
    if cancel.is_cancelled() {
        return cancelled();
    }

    // Resolve agent profile. Without it we have no grounding context to
    // give the judges, so we skip rather than running with empty inputs.
    let workspace_id = req.workspace_id.as_deref().unwrap_or(DEFAULT_WORKSPACE_ID);
    let profile = match ctx
        .profile_resolver
        .resolve(workspace_id, &req.agent_id)
        .await
    {
        Some(p) => p,
        None => return skipped(start),
    };

    // Decide which judges have routes configured. If none → skip.
    let do_hallu = ctx.llm.has_route(JudgeKind::Hallucination);
    let do_tone = ctx.llm.has_route(JudgeKind::Tone);
    let do_auth = ctx.llm.has_route(JudgeKind::Authority);
    if !do_hallu && !do_tone && !do_auth {
        return skipped(start);
    }

    // Gather per-request docs from `req.context.docs`. Missing or wrong
    // shape is fine — judges receive an empty doc set.
    let mut docs = extract_docs(&req.context);
    if do_hallu {
        let source_ids = profile
            .knowledge_sources
            .iter()
            .map(|source| source.kb_id.clone())
            .collect::<Vec<_>>();
        let snippets = ctx
            .knowledge
            .retrieve(KnowledgeRetrievalRequest {
                workspace_id: workspace_id.to_string(),
                agent_id: req.agent_id.clone(),
                source_ids,
                input: req.input.clone(),
                proposed_output: req.proposed_output.clone(),
            })
            .await;
        let knowledge_docs = format_knowledge_snippets(&snippets);
        let knowledge_chars = knowledge_docs.iter().map(String::len).sum::<usize>();
        tracing::info!(
            workspace_id,
            agent_id = %req.agent_id,
            knowledge_snippet_count = snippets.len(),
            knowledge_chars,
            knowledge_est_tokens = estimate_prompt_tokens(knowledge_chars),
            knowledge_source_ids = %snippets
                .iter()
                .map(|snippet| snippet.source_id.as_str())
                .collect::<Vec<_>>()
                .join(","),
            "knowledge grounding prompt contribution"
        );
        docs.extend(knowledge_docs);
    }

    // Build prompts up front so the join body is just IO.
    let hallu_prompt = hallucination::build(
        &summarise_profile(&profile),
        &docs.join("\n---\n"),
        &req.input,
        &req.proposed_output,
    );
    let tone_prompt = tone::build(
        &profile.tone.target,
        &profile.tone.forbidden.join(", "),
        &req.input,
        &req.proposed_output,
    );
    let auth_prompt = authority::build(
        &bulleted(&profile.authority.can_promise),
        &bulleted(&profile.authority.cannot_promise),
        &req.input,
        &req.proposed_output,
    );

    // Tenant id lives outside this trait surface in v0; default to the
    // agent_id which is unique per agent and sufficient for budget
    // bucketing until proper multi-tenancy lands.
    let tenant = workspace_id;

    // Cancellation is honored at this select level: if the orchestrator
    // fires the cancel token, abort immediately rather than waiting for
    // the LLM round-trips.
    let result = tokio::select! {
        _ = cancel.cancelled() => return cancelled(),
        out = run_judges(
            &ctx.llm,
            tenant,
            do_hallu, &hallu_prompt,
            do_tone, &tone_prompt,
            do_auth, &auth_prompt,
        ) => out,
    };

    let JudgeOutcomes { hallu, tone, auth } = result;
    aggregate(start, &profile, hallu, tone, auth)
}

fn estimate_prompt_tokens(chars: usize) -> usize {
    chars.div_ceil(4)
}
