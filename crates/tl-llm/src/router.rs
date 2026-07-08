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

use tracing::Span;

use crate::budget::TokenBudget;
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
    HardenDraft,
    TrajectoryDiagnostic,
}

impl JudgeKind {
    pub fn as_str(self) -> &'static str {
        match self {
            JudgeKind::Hallucination => "hallucination",
            JudgeKind::Tone => "tone",
            JudgeKind::Authority => "authority",
            JudgeKind::SemanticPolicy => "semantic_policy",
            JudgeKind::HardenDraft => "harden_draft",
            JudgeKind::TrajectoryDiagnostic => "trajectory_diagnostic",
        }
    }

    pub const fn all() -> &'static [JudgeKind] {
        &[
            JudgeKind::Hallucination,
            JudgeKind::Tone,
            JudgeKind::Authority,
            JudgeKind::SemanticPolicy,
            JudgeKind::HardenDraft,
            JudgeKind::TrajectoryDiagnostic,
        ]
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
    ///   1. Check `tenant`'s budget. Over → `LlmError::BudgetExceeded`.
    ///   2. Resolve route for `kind`. Missing → wrapped as Http error.
    ///   3. Call primary provider with its model + deadline.
    ///   4. On error/timeout, try fallback if configured; else return err.
    ///   5. Record token usage to budget + emit a `tracing` event with
    ///      `llm.provider`, `llm.model`, `llm.judge`, `llm.prompt_tokens`,
    ///      `llm.completion_tokens`, `llm.fallback_used`.
    pub async fn judge(
        &self,
        kind: JudgeKind,
        tenant: &str,
        prompt: &str,
        schema: &JsonSchema,
    ) -> Result<LlmOutput, LlmError> {
        if let Err(b) = self.budget.check(tenant) {
            tracing::warn!(
                tenant = tenant,
                used = b.used,
                limit = b.limit,
                "tenant token budget exceeded"
            );
            return Err(LlmError::BudgetExceeded);
        }

        let route = self.routes.get(&kind).ok_or_else(|| {
            LlmError::Http(format!("no route configured for judge `{}`", kind.as_str()))
        })?;

        // Try primary.
        match self.call_target(&route.primary, prompt, schema).await {
            Ok(out) => {
                self.record(tenant, kind, &route.primary, &out, false);
                Ok(out)
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
                            self.record(tenant, kind, fallback, &out, true);
                            Ok(out)
                        }
                        Err(fallback_err) => {
                            tracing::error!(
                                judge = kind.as_str(),
                                primary_error = %primary_err,
                                fallback_error = %fallback_err,
                                "both primary and fallback failed"
                            );
                            Err(fallback_err)
                        }
                    }
                } else {
                    Err(primary_err)
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
        tenant: &str,
        kind: JudgeKind,
        target: &ProviderTarget,
        out: &LlmOutput,
        fallback_used: bool,
    ) {
        let total = out.prompt_tokens as u64 + out.completion_tokens as u64;
        self.budget.record(tenant, total);
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

#[derive(Debug, thiserror::Error)]
pub enum RouterBuildError {
    #[error("env var `{0}` not set")]
    MissingEnv(String),
    #[error("unknown provider kind `{0}` (expected openai|openrouter)")]
    UnknownProviderKind(String),
    #[error("unknown judge kind `{0}` (expected hallucination|tone|authority|semantic_policy|harden_draft|trajectory_diagnostic)")]
    UnknownJudgeKind(String),
    #[error("route references unknown provider `{0}`")]
    UnknownProvider(String),
    #[error("provider init failed: {0}")]
    Provider(String),
}
