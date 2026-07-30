//! Central LLM dispatcher.
//!
//! Tier 3 in `tl-engine` calls `LlmRouter::judge(kind, tenant, prompt, schema)`
//! and gets back an `LlmOutput`. Everything else — provider selection,
//! per-judge model choice, failover on primary error/timeout, per-tenant
//! token budgets, telemetry to `tracing` — lives in here.
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
use crate::client::{JsonSchema, LlmClient, LlmError, LlmOutput};
use crate::config::ProviderTarget;

/// Which judge is calling. String-keyed so `routes.<kind>` keys in TOML
/// match these names case-insensitively.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum JudgeKind {
    Hallucination,
    Tone,
    Authority,
    SemanticPolicy,
}

impl JudgeKind {
    pub fn as_str(self) -> &'static str {
        match self {
            JudgeKind::Hallucination => "hallucination",
            JudgeKind::Tone => "tone",
            JudgeKind::Authority => "authority",
            JudgeKind::SemanticPolicy => "semantic_policy",
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
    routes: HashMap<JudgeKind, ResolvedRoute>,
    budget: Arc<TokenBudget>,
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
        routes: HashMap<JudgeKind, ResolvedRoute>,
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
        self.routes.contains_key(&kind)
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
        let route = self.routes.get(&kind).ok_or_else(|| AuditedLlmError {
            error: LlmError::Http(format!("no route configured for judge `{}`", kind.as_str())),
            audit: failed_audit(kind, None, false, started.elapsed(), "route_missing"),
        })?;

        // Try primary.
        match self.call_target(&route.primary, prompt, schema).await {
            Ok(out) => {
                self.record(reservation, tenant, kind, &route.primary, &out, false);
                Ok(AuditedLlmOutput {
                    audit: successful_audit(kind, &route.primary, &out, false, started.elapsed()),
                    output: out,
                })
            }
            Err(primary_err) => {
                if let Some(fallback) = &route.fallback {
                    tracing::info!(
                        judge = kind.as_str(),
                        primary_provider = %route.primary.provider,
                        primary_error = %primary_err,
                        "primary failed, trying fallback"
                    );
                    match self.call_target(fallback, prompt, schema).await {
                        Ok(out) => {
                            self.record(reservation, tenant, kind, fallback, &out, true);
                            Ok(AuditedLlmOutput {
                                audit: successful_audit(
                                    kind,
                                    fallback,
                                    &out,
                                    true,
                                    started.elapsed(),
                                ),
                                output: out,
                            })
                        }
                        Err(fallback_err) => {
                            tracing::error!(
                                judge = kind.as_str(),
                                primary_error = %primary_err,
                                fallback_error = %fallback_err,
                                "both primary and fallback failed"
                            );
                            let code = error_code(&fallback_err);
                            Err(AuditedLlmError {
                                error: fallback_err,
                                audit: failed_audit(
                                    kind,
                                    Some(fallback),
                                    true,
                                    started.elapsed(),
                                    code,
                                ),
                            })
                        }
                    }
                } else {
                    let code = error_code(&primary_err);
                    Err(AuditedLlmError {
                        error: primary_err,
                        audit: failed_audit(
                            kind,
                            Some(&route.primary),
                            false,
                            started.elapsed(),
                            code,
                        ),
                    })
                }
            }
        }
    }

    async fn call_target(
        &self,
        target: &ProviderTarget,
        prompt: &str,
        schema: &JsonSchema,
    ) -> Result<LlmOutput, LlmError> {
        let client = self
            .providers
            .get(&target.provider)
            .ok_or_else(|| LlmError::Http(format!("provider `{}` not found", target.provider)))?;
        let deadline = Duration::from_millis(target.deadline_ms as u64);
        client
            .complete(&target.model, prompt, schema, deadline)
            .await
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
        span.record("llm.judge", kind.as_str());
        span.record("llm.prompt_tokens", out.prompt_tokens as u64);
        span.record("llm.completion_tokens", out.completion_tokens as u64);
        span.record("llm.fallback_used", fallback_used);
        tracing::info!(
            tenant = tenant,
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
    #[error("unknown judge kind `{0}` (expected hallucination|tone|authority|semantic_policy)")]
    UnknownJudgeKind(String),
    #[error("route references unknown provider `{0}`")]
    UnknownProvider(String),
    #[error("provider init failed: {0}")]
    Provider(String),
}
