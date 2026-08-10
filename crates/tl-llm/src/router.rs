//! Central LLM dispatcher.
//!
//! Tier 3 in `tl-engine` uses the budgeted judge API. Server control-plane
//! workloads use the unbudgeted route API. Provider selection, model choice,
//! optional reasoning effort, failover, and telemetry live here.
//!
//! See `docs/concept/v0-design-decisions.md §9` for the rationale.

mod config_build;
#[cfg(test)]
mod tests;

use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;
use std::time::Instant;

use tracing::Span;

use crate::budget::{BudgetExceeded, TokenBudget, TokenBudgetReservation};
use crate::client::{JsonSchema, LlmClient, LlmCompletionOptions, LlmError, LlmOutput};
use crate::config::ProviderTarget;

/// Runtime judge identity. This remains separate from the general route key
/// because `LlmCallAudit.judge` is part of the persisted runtime trace contract.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum JudgeKind {
    Hallucination,
    Tone,
    Authority,
    SemanticPolicy,
    RunEvaluation,
}

/// A first-party model-selection workload in the canonical routing manifest.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum LlmRouteKind {
    Hallucination,
    Tone,
    Authority,
    SemanticPolicy,
    RunEvaluation,
    PolicyDraft,
    PolicyAiEdit,
    GuardrailGeneration,
    GitHubIntegration,
    DemoDefault,
    DemoDispute,
    DemoLivekit,
}

impl LlmRouteKind {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Hallucination => "hallucination",
            Self::Tone => "tone",
            Self::Authority => "authority",
            Self::SemanticPolicy => "semantic_policy",
            Self::RunEvaluation => "run_evaluation",
            Self::PolicyDraft => "policy_draft",
            Self::PolicyAiEdit => "policy_ai_edit",
            Self::GuardrailGeneration => "guardrail_generation",
            Self::GitHubIntegration => "github_integration",
            Self::DemoDefault => "demo_default",
            Self::DemoDispute => "demo_dispute",
            Self::DemoLivekit => "demo_livekit",
        }
    }

    pub(crate) fn parse(value: &str) -> Option<Self> {
        match value {
            "hallucination" => Some(Self::Hallucination),
            "tone" => Some(Self::Tone),
            "authority" => Some(Self::Authority),
            "semantic_policy" => Some(Self::SemanticPolicy),
            "run_evaluation" => Some(Self::RunEvaluation),
            "policy_draft" => Some(Self::PolicyDraft),
            "policy_ai_edit" => Some(Self::PolicyAiEdit),
            "guardrail_generation" => Some(Self::GuardrailGeneration),
            "github_integration" => Some(Self::GitHubIntegration),
            "demo_default" => Some(Self::DemoDefault),
            "demo_dispute" => Some(Self::DemoDispute),
            "demo_livekit" => Some(Self::DemoLivekit),
            _ => None,
        }
    }
}

impl From<JudgeKind> for LlmRouteKind {
    fn from(value: JudgeKind) -> Self {
        match value {
            JudgeKind::Hallucination => Self::Hallucination,
            JudgeKind::Tone => Self::Tone,
            JudgeKind::Authority => Self::Authority,
            JudgeKind::SemanticPolicy => Self::SemanticPolicy,
            JudgeKind::RunEvaluation => Self::RunEvaluation,
        }
    }
}

impl JudgeKind {
    pub fn as_str(self) -> &'static str {
        match self {
            JudgeKind::Hallucination => "hallucination",
            JudgeKind::Tone => "tone",
            JudgeKind::Authority => "authority",
            JudgeKind::SemanticPolicy => "semantic_policy",
            JudgeKind::RunEvaluation => "run_evaluation",
        }
    }
}

#[derive(Clone)]
pub struct ResolvedRoute {
    pub primary: ProviderTarget,
    pub fallback: Option<ProviderTarget>,
}

pub struct LlmRouter {
    providers: HashMap<String, Arc<dyn LlmClient>>,
    routes: HashMap<LlmRouteKind, ResolvedRoute>,
    budget: Arc<TokenBudget>,
}

struct DispatchedLlmOutput {
    output: LlmOutput,
    target: ProviderTarget,
    fallback_used: bool,
}

struct DispatchedLlmError {
    error: LlmError,
    target: Option<ProviderTarget>,
    fallback_used: bool,
}

/// One atomically admitted tenant evaluation.
///
/// Tier 3 shares this session across its configured judges so they can fan out
/// concurrently while other evaluations for the same capped tenant wait.
pub struct LlmRouterSession<'a> {
    router: &'a LlmRouter,
    tenant: String,
    reservation: TokenBudgetReservation<'a>,
}

#[derive(Debug, Clone)]
pub struct LlmCallAudit {
    pub judge: String,
    pub provider: Option<String>,
    pub model: Option<String>,
    pub status: String,
    pub prompt_tokens: Option<u32>,
    pub completion_tokens: Option<u32>,
    pub fallback_used: bool,
    pub latency_ms: u64,
    pub error_code: Option<String>,
}

#[derive(Debug, Clone)]
pub struct AuditedLlmOutput {
    pub output: LlmOutput,
    pub audit: LlmCallAudit,
}

#[derive(Debug)]
pub struct AuditedLlmError {
    pub error: LlmError,
    pub audit: LlmCallAudit,
}

impl std::fmt::Debug for LlmRouter {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("LlmRouter")
            .field("providers", &self.providers.keys().collect::<Vec<_>>())
            .field("routes", &self.routes.keys().collect::<Vec<_>>())
            .finish()
    }
}

impl LlmRouter {
    pub fn new(
        providers: HashMap<String, Arc<dyn LlmClient>>,
        routes: HashMap<LlmRouteKind, ResolvedRoute>,
        budget: Arc<TokenBudget>,
    ) -> Self {
        Self {
            providers,
            routes,
            budget,
        }
    }

    pub fn budget(&self) -> &TokenBudget {
        &self.budget
    }

    /// True when a route is configured for `kind`. Tier 3 uses this to
    /// decide whether to call a judge or report `Skipped`.
    pub fn has_route(&self, kind: JudgeKind) -> bool {
        self.has_workload_route(kind.into())
    }

    /// True when the canonical manifest contains a route for this workload.
    pub fn has_workload_route(&self, route: LlmRouteKind) -> bool {
        self.routes.contains_key(&route)
    }

    /// Atomically admit one tenant evaluation.
    pub async fn start_session(
        &self,
        tenant: &str,
    ) -> Result<LlmRouterSession<'_>, BudgetExceeded> {
        let reservation = self.budget.reserve(tenant).await?;
        Ok(LlmRouterSession {
            router: self,
            tenant: tenant.to_string(),
            reservation,
        })
    }

    /// Empty router with no providers, no routes, and an unlimited
    /// budget. Used by `HandlerCtx::no_op()` and tests that don't
    /// exercise Tier 3.
    pub fn empty() -> Self {
        Self::new(
            HashMap::new(),
            HashMap::new(),
            Arc::new(TokenBudget::new(0)),
        )
    }

    /// Issue one judge call. Steps:
    ///   1. Atomically reserve `tenant`'s budget. Over → `LlmError::BudgetExceeded`.
    ///   2. Resolve route for `kind`. Missing → wrapped as Http error.
    ///   3. Call primary provider with its model + deadline.
    ///   4. On error/timeout, try fallback if configured; else return err.
    ///   5. Settle token usage to budget + emit a `tracing` event with
    ///      `llm.provider`, `llm.model`, `llm.judge`, `llm.prompt_tokens`,
    ///      `llm.completion_tokens`, `llm.fallback_used`.
    pub async fn judge(
        &self,
        kind: JudgeKind,
        tenant: &str,
        prompt: &str,
        schema: &JsonSchema,
    ) -> Result<LlmOutput, LlmError> {
        self.judge_with_audit(kind, tenant, prompt, schema)
            .await
            .map(|result| result.output)
            .map_err(|error| error.error)
    }

    pub async fn judge_with_audit(
        &self,
        kind: JudgeKind,
        tenant: &str,
        prompt: &str,
        schema: &JsonSchema,
    ) -> Result<AuditedLlmOutput, AuditedLlmError> {
        let started = Instant::now();
        let reservation = self.budget.reserve(tenant).await.map_err(|b| {
            tracing::warn!(
                tenant = tenant,
                used = b.used,
                limit = b.limit,
                "tenant token budget exceeded"
            );
            AuditedLlmError {
                error: LlmError::BudgetExceeded,
                audit: failed_audit(kind, None, false, started.elapsed(), "budget_exceeded"),
            }
        })?;

        self.judge_with_audit_in_session(kind, tenant, prompt, schema, &reservation, started)
            .await
    }

    async fn judge_with_audit_in_session(
        &self,
        kind: JudgeKind,
        tenant: &str,
        prompt: &str,
        schema: &JsonSchema,
        reservation: &TokenBudgetReservation<'_>,
        started: Instant,
    ) -> Result<AuditedLlmOutput, AuditedLlmError> {
        let route_kind = LlmRouteKind::from(kind);
        let route = self
            .routes
            .get(&route_kind)
            .ok_or_else(|| AuditedLlmError {
                error: LlmError::Http(format!("no route configured for judge `{}`", kind.as_str())),
                audit: failed_audit(kind, None, false, started.elapsed(), "route_missing"),
            })?;

        match self.dispatch(route_kind, route, prompt, schema).await {
            Ok(dispatched) => {
                self.record(
                    reservation,
                    tenant,
                    kind,
                    &dispatched.target,
                    &dispatched.output,
                    dispatched.fallback_used,
                );
                Ok(AuditedLlmOutput {
                    audit: successful_audit(
                        kind,
                        &dispatched.target,
                        &dispatched.output,
                        dispatched.fallback_used,
                        started.elapsed(),
                    ),
                    output: dispatched.output,
                })
            }
            Err(dispatched) => {
                let code = error_code(&dispatched.error);
                Err(AuditedLlmError {
                    error: dispatched.error,
                    audit: failed_audit(
                        kind,
                        dispatched.target.as_ref(),
                        dispatched.fallback_used,
                        started.elapsed(),
                        code,
                    ),
                })
            }
        }
    }

    /// Complete a pre-existing control-plane workload through the canonical
    /// route without charging the Tier 3 runtime judge budget.
    pub async fn complete_route(
        &self,
        route_kind: LlmRouteKind,
        workspace_id: &str,
        prompt: &str,
        schema: &JsonSchema,
    ) -> Result<LlmOutput, LlmError> {
        let started = Instant::now();
        let span = Span::current();
        span.record("llm.route", route_kind.as_str());
        let route = self.routes.get(&route_kind).ok_or_else(|| {
            LlmError::Http(format!(
                "no route configured for llm workload `{}`",
                route_kind.as_str()
            ))
        })?;
        let dispatched = match self.dispatch(route_kind, route, prompt, schema).await {
            Ok(dispatched) => dispatched,
            Err(dispatched) => {
                if let Some(target) = &dispatched.target {
                    span.record("llm.provider", target.provider.as_str());
                    span.record("llm.model", target.model.as_str());
                }
                span.record("llm.fallback_used", dispatched.fallback_used);
                tracing::warn!(
                    workspace_id,
                    route = route_kind.as_str(),
                    fallback_used = dispatched.fallback_used,
                    latency_ms = started.elapsed().as_millis() as u64,
                    error = %dispatched.error,
                    "llm workload failed"
                );
                return Err(dispatched.error);
            }
        };
        span.record("llm.provider", dispatched.target.provider.as_str());
        span.record("llm.model", dispatched.target.model.as_str());
        span.record("llm.fallback_used", dispatched.fallback_used);
        tracing::info!(
            workspace_id,
            route = route_kind.as_str(),
            provider = %dispatched.target.provider,
            model = %dispatched.target.model,
            prompt_tokens = dispatched.output.prompt_tokens,
            completion_tokens = dispatched.output.completion_tokens,
            total_tokens = u64::from(dispatched.output.prompt_tokens)
                + u64::from(dispatched.output.completion_tokens),
            fallback_used = dispatched.fallback_used,
            latency_ms = started.elapsed().as_millis() as u64,
            "llm workload completed"
        );
        Ok(dispatched.output)
    }

    async fn dispatch(
        &self,
        route_kind: LlmRouteKind,
        route: &ResolvedRoute,
        prompt: &str,
        schema: &JsonSchema,
    ) -> Result<DispatchedLlmOutput, DispatchedLlmError> {
        let started = Instant::now();
        let route_deadline_ms = route
            .fallback
            .as_ref()
            .map_or(route.primary.deadline_ms, |fallback| {
                route.primary.deadline_ms.max(fallback.deadline_ms)
            });
        let route_deadline = Duration::from_millis(u64::from(route_deadline_ms));
        let primary_deadline =
            Duration::from_millis(u64::from(route.primary.deadline_ms)).min(route_deadline);
        match self
            .call_target(&route.primary, prompt, schema, primary_deadline)
            .await
        {
            Ok(output) => Ok(DispatchedLlmOutput {
                output,
                target: route.primary.clone(),
                fallback_used: false,
            }),
            Err(primary_error) => {
                let Some(fallback) = &route.fallback else {
                    return Err(DispatchedLlmError {
                        error: primary_error,
                        target: Some(route.primary.clone()),
                        fallback_used: false,
                    });
                };
                tracing::info!(
                    route = route_kind.as_str(),
                    primary_provider = %route.primary.provider,
                    primary_error = %primary_error,
                    "primary failed, trying fallback"
                );
                let Some(remaining) = route_deadline.checked_sub(started.elapsed()) else {
                    return Err(DispatchedLlmError {
                        error: LlmError::Timeout(route_deadline),
                        target: Some(fallback.clone()),
                        fallback_used: true,
                    });
                };
                let fallback_deadline =
                    Duration::from_millis(u64::from(fallback.deadline_ms)).min(remaining);
                match self
                    .call_target(fallback, prompt, schema, fallback_deadline)
                    .await
                {
                    Ok(output) => Ok(DispatchedLlmOutput {
                        output,
                        target: fallback.clone(),
                        fallback_used: true,
                    }),
                    Err(fallback_error) => {
                        tracing::error!(
                            route = route_kind.as_str(),
                            primary_error = %primary_error,
                            fallback_error = %fallback_error,
                            "both primary and fallback failed"
                        );
                        Err(DispatchedLlmError {
                            error: fallback_error,
                            target: Some(fallback.clone()),
                            fallback_used: true,
                        })
                    }
                }
            }
        }
    }

    async fn call_target(
        &self,
        target: &ProviderTarget,
        prompt: &str,
        schema: &JsonSchema,
        deadline: Duration,
    ) -> Result<LlmOutput, LlmError> {
        let client = self
            .providers
            .get(&target.provider)
            .ok_or_else(|| LlmError::Http(format!("provider `{}` not found", target.provider)))?;
        let options = LlmCompletionOptions {
            reasoning_effort: target.reasoning_effort,
        };
        tokio::time::timeout(
            deadline,
            client.complete_with_options(&target.model, prompt, schema, deadline, &options),
        )
        .await
        .map_err(|_| LlmError::Timeout(deadline))?
    }

    fn record(
        &self,
        reservation: &TokenBudgetReservation<'_>,
        tenant: &str,
        kind: JudgeKind,
        target: &ProviderTarget,
        out: &LlmOutput,
        fallback_used: bool,
    ) {
        let total = out.prompt_tokens as u64 + out.completion_tokens as u64;
        reservation.record(total);
        let span = Span::current();
        // Attach structured fields. Consumers query these in tracing logs.
        span.record("llm.provider", target.provider.as_str());
        span.record("llm.model", target.model.as_str());
        span.record("llm.route", LlmRouteKind::from(kind).as_str());
        span.record("llm.judge", kind.as_str());
        span.record("llm.prompt_tokens", out.prompt_tokens as u64);
        span.record("llm.completion_tokens", out.completion_tokens as u64);
        span.record("llm.fallback_used", fallback_used);
        tracing::info!(
            tenant = tenant,
            route = LlmRouteKind::from(kind).as_str(),
            judge = kind.as_str(),
            provider = %target.provider,
            model = %target.model,
            prompt_tokens = out.prompt_tokens,
            completion_tokens = out.completion_tokens,
            fallback_used,
            "llm judge completed"
        );
    }
}

impl LlmRouterSession<'_> {
    pub async fn judge(
        &self,
        kind: JudgeKind,
        prompt: &str,
        schema: &JsonSchema,
    ) -> Result<LlmOutput, LlmError> {
        self.router
            .judge_with_audit_in_session(
                kind,
                &self.tenant,
                prompt,
                schema,
                &self.reservation,
                Instant::now(),
            )
            .await
            .map(|result| result.output)
            .map_err(|error| error.error)
    }
}

fn successful_audit(
    kind: JudgeKind,
    target: &ProviderTarget,
    output: &LlmOutput,
    fallback_used: bool,
    elapsed: Duration,
) -> LlmCallAudit {
    LlmCallAudit {
        judge: kind.as_str().to_string(),
        provider: Some(target.provider.clone()),
        model: Some(target.model.clone()),
        status: "succeeded".to_string(),
        prompt_tokens: Some(output.prompt_tokens),
        completion_tokens: Some(output.completion_tokens),
        fallback_used,
        latency_ms: elapsed.as_millis() as u64,
        error_code: None,
    }
}

fn failed_audit(
    kind: JudgeKind,
    target: Option<&ProviderTarget>,
    fallback_used: bool,
    elapsed: Duration,
    error_code: &str,
) -> LlmCallAudit {
    LlmCallAudit {
        judge: kind.as_str().to_string(),
        provider: target.map(|target| target.provider.clone()),
        model: target.map(|target| target.model.clone()),
        status: "failed".to_string(),
        prompt_tokens: None,
        completion_tokens: None,
        fallback_used,
        latency_ms: elapsed.as_millis() as u64,
        error_code: Some(error_code.to_string()),
    }
}

fn error_code(error: &LlmError) -> &'static str {
    match error {
        LlmError::Http(_) => "http",
        LlmError::Status(_, _) => "provider_status",
        LlmError::Parse(_) => "parse",
        LlmError::MissingField(_) => "missing_field",
        LlmError::Timeout(_) => "timeout",
        LlmError::BudgetExceeded => "budget_exceeded",
    }
}

#[derive(Debug, thiserror::Error)]
pub enum RouterBuildError {
    #[error("env var `{0}` not set")]
    MissingEnv(String),
    #[error("unknown provider kind `{0}` (expected openai|openrouter)")]
    UnknownProviderKind(String),
    #[error("unknown llm route kind `{0}`")]
    UnknownRouteKind(String),
    #[error("route references unknown provider `{0}`")]
    UnknownProvider(String),
    #[error("provider init failed: {0}")]
    Provider(String),
}
