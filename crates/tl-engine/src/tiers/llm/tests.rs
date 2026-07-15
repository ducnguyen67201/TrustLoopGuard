use std::collections::HashMap;
use std::sync::Arc;

use async_trait::async_trait;
use serde_json::json;
use tl_core::{
    AgentAuthority, AgentProfile, AgentScope, AgentTone, AuthorizationEffect, Channel,
    CheckRequest, KnowledgeSource, KnowledgeSourceKind, TierStatus,
};
use tl_llm::{
    JsonSchema, JudgeKind, LlmClient, LlmOutput, LlmRouter, ProviderTarget, ResolvedRoute,
    TokenBudget,
};
use tokio_util::sync::CancellationToken;

use super::prompt_context::summarise_profile;
use super::run;
use crate::context::HandlerCtx;

struct CannedClient {
    out: serde_json::Value,
}

struct FailingClient;

#[async_trait]
impl LlmClient for CannedClient {
    async fn complete(
        &self,
        _model: &str,
        _prompt: &str,
        _schema: &JsonSchema,
        _deadline: std::time::Duration,
    ) -> Result<LlmOutput, tl_llm::LlmError> {
        Ok(LlmOutput {
            json: self.out.clone(),
            prompt_tokens: 5,
            completion_tokens: 5,
        })
    }
}

#[async_trait]
impl LlmClient for FailingClient {
    async fn complete(
        &self,
        _model: &str,
        _prompt: &str,
        _schema: &JsonSchema,
        _deadline: std::time::Duration,
    ) -> Result<LlmOutput, tl_llm::LlmError> {
        Err(tl_llm::LlmError::Http("judge unavailable".into()))
    }
}

fn router_returning(json: serde_json::Value) -> LlmRouter {
    let mut providers: HashMap<String, Arc<dyn LlmClient>> = HashMap::new();
    providers.insert("p".into(), Arc::new(CannedClient { out: json }));
    let target = ProviderTarget {
        provider: "p".into(),
        model: "m".into(),
        deadline_ms: 1_000,
    };
    let mut routes = HashMap::new();
    for kind in [
        JudgeKind::Hallucination,
        JudgeKind::Tone,
        JudgeKind::Authority,
    ] {
        routes.insert(
            kind,
            ResolvedRoute {
                primary: target.clone(),
                fallback: None,
            },
        );
    }
    LlmRouter::new(providers, routes, Arc::new(TokenBudget::new(0)))
}

fn failing_router() -> LlmRouter {
    let mut providers: HashMap<String, Arc<dyn LlmClient>> = HashMap::new();
    providers.insert("p".into(), Arc::new(FailingClient));
    let target = ProviderTarget {
        provider: "p".into(),
        model: "m".into(),
        deadline_ms: 1_000,
    };
    let mut routes = HashMap::new();
    for kind in [
        JudgeKind::Hallucination,
        JudgeKind::Tone,
        JudgeKind::Authority,
    ] {
        routes.insert(
            kind,
            ResolvedRoute {
                primary: target.clone(),
                fallback: None,
            },
        );
    }
    LlmRouter::new(providers, routes, Arc::new(TokenBudget::new(0)))
}

struct FixedResolver(Arc<AgentProfile>);

#[async_trait]
impl crate::context::ProfileResolver for FixedResolver {
    async fn resolve(&self, _workspace_id: &str, _agent_id: &str) -> Option<Arc<AgentProfile>> {
        Some(self.0.clone())
    }
}

fn sample_profile() -> Arc<AgentProfile> {
    Arc::new(AgentProfile {
        agent_id: "a".into(),
        display_name: "Test Agent".into(),
        scope: AgentScope {
            in_scope: vec!["billing".into()],
            out_of_scope: vec![],
        },
        authority: AgentAuthority {
            can_promise: vec!["respond within 24h".into()],
            cannot_promise: vec!["refunds".into()],
        },
        tone: AgentTone {
            target: "warm-professional".into(),
            forbidden: vec!["sarcastic".into()],
        },
        knowledge_sources: vec![],
        escalation_triggers: vec![],
        workflow_requirements: vec![],
        system_prompt: None,
        workflow_definition: None,
        target_url: None,
    })
}

fn sample_req() -> CheckRequest {
    CheckRequest {
        workspace_id: None,
        run_id: None,
        run_event_id: None,
        run_event: None,
        session_id: None,
        agent_id: "a".into(),
        channel: Channel::Chat,
        input: "hello".into(),
        proposed_output: "hi there".into(),
        domain: None,
        policies: vec![],
        context: serde_json::Value::Null,
        trace_id: None,
        redaction: None,
    }
}

fn ctx_with(router: LlmRouter) -> HandlerCtx {
    let mut context = HandlerCtx::no_op();
    context.profile_resolver = Arc::new(FixedResolver(sample_profile()));
    context.llm = Arc::new(router);
    context
}

#[tokio::test]
async fn no_profile_yields_skipped() {
    let context = HandlerCtx::no_op();
    let out = run(&sample_req(), &context, CancellationToken::new()).await;
    assert_eq!(out.result.status, TierStatus::Skipped);
    assert!(out.block.is_none());
}

#[tokio::test]
async fn empty_router_yields_skipped() {
    let mut context = HandlerCtx::no_op();
    context.profile_resolver = Arc::new(FixedResolver(sample_profile()));
    let out = run(&sample_req(), &context, CancellationToken::new()).await;
    assert_eq!(out.result.status, TierStatus::Skipped);
}

#[tokio::test]
async fn unavailable_judge_defers_instead_of_requiring_approval() {
    let context = ctx_with(failing_router());
    let out = run(&sample_req(), &context, CancellationToken::new()).await;
    let stop = out.block.expect("unavailable evidence must stop delivery");
    assert_eq!(stop.effect, AuthorizationEffect::Defer);
    assert!(stop.reason.contains("judge unavailable"));
}

#[tokio::test]
async fn three_clean_verdicts_yield_completed_with_no_block() {
    let json = json!({
        "grounded": true,
        "violations": [],
        "matches_target": true,
        "detected_tone": "warm-professional",
        "issues": [],
        "within_authority": true,
        "forbidden_promises": []
    });
    let context = ctx_with(router_returning(json));
    let out = run(&sample_req(), &context, CancellationToken::new()).await;
    assert_eq!(out.result.status, TierStatus::Completed);
    assert!(out.block.is_none(), "no judge fired, block should be None");
    assert!(out.result.reasons.is_empty());
}

#[test]
fn profile_summary_includes_web_knowledge_source_metadata() {
    let mut profile = (*sample_profile()).clone();
    profile.knowledge_sources = vec![KnowledgeSource {
        kb_id: "acme-docs".into(),
        kind: KnowledgeSourceKind::Web,
        url: Some("https://docs.acme.test/help".into()),
        description: Some("Public support docs".into()),
    }];

    let summary = summarise_profile(&profile);

    assert!(summary.contains("acme-docs"));
    assert!(summary.contains("web"));
    assert!(summary.contains("Public support docs"));
    assert!(summary.contains("https://docs.acme.test/help"));
}

#[tokio::test]
async fn hallucination_violation_blocks() {
    let json = json!({
        "grounded": false,
        "violations": ["agent claimed 24/7 phone support but docs say email only"],
        "matches_target": true,
        "detected_tone": "ok",
        "issues": [],
        "within_authority": true,
        "forbidden_promises": []
    });
    let context = ctx_with(router_returning(json));
    let out = run(&sample_req(), &context, CancellationToken::new()).await;
    let block = out.block.expect("block set");
    assert_eq!(block.effect, AuthorizationEffect::Deny);
    assert!(block.reason.contains("hallucination"));
    assert!(out
        .result
        .reasons
        .iter()
        .any(|reason| reason.id == "tl:hallucination"));
}

#[tokio::test]
async fn authority_violation_blocks() {
    let json = json!({
        "grounded": true,
        "violations": [],
        "matches_target": true,
        "detected_tone": "ok",
        "issues": [],
        "within_authority": false,
        "forbidden_promises": ["promised a full refund"]
    });
    let context = ctx_with(router_returning(json));
    let out = run(&sample_req(), &context, CancellationToken::new()).await;
    let block = out.block.expect("block set");
    assert_eq!(block.effect, AuthorizationEffect::Deny);
    assert!(block.reason.contains("authority"));
    assert!(out
        .result
        .reasons
        .iter()
        .any(|reason| reason.id == "tl:authority"));
}

#[tokio::test]
async fn tone_mismatch_yields_rewrite_verdict() {
    let json = json!({
        "grounded": true,
        "violations": [],
        "matches_target": false,
        "detected_tone": "curt",
        "issues": ["too clipped, no acknowledgement"],
        "within_authority": true,
        "forbidden_promises": []
    });
    let context = ctx_with(router_returning(json));
    let out = run(&sample_req(), &context, CancellationToken::new()).await;
    let block = out.block.expect("block set");
    assert_eq!(block.effect, AuthorizationEffect::Transform);
    assert!(block.reason.contains("tone"));
}

#[tokio::test]
async fn pre_cancelled_token_short_circuits() {
    let context = ctx_with(router_returning(json!({})));
    let cancel = CancellationToken::new();
    cancel.cancel();
    let out = run(&sample_req(), &context, cancel).await;
    assert_eq!(out.result.status, TierStatus::Cancelled);
}
