//! Central LLM dispatcher.
//!
//! Tier 3 in `tl-engine` calls `LlmRouter::judge(kind, tenant, prompt, schema)`
//! and gets back an `LlmOutput`. Everything else — provider selection,
//! per-judge model choice, failover on primary error/timeout, per-tenant
//! token budgets, telemetry to `tracing` — lives in here.
//!
//! See `docs/concept/v0-design-decisions.md §9` for the rationale.

use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;

use tracing::Span;

use crate::budget::TokenBudget;
use crate::client::{JsonSchema, LlmClient, LlmError, LlmOutput};
use crate::config::{ProviderTarget, RouterConfig};
use crate::{OpenAiClient, OpenRouterClient};

/// Which judge is calling. String-keyed so `routes.<kind>` keys in TOML
/// match these names case-insensitively (`hallucination`, `tone`, `authority`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum JudgeKind {
    Hallucination,
    Tone,
    Authority,
}

impl JudgeKind {
    pub fn as_str(self) -> &'static str {
        match self {
            JudgeKind::Hallucination => "hallucination",
            JudgeKind::Tone => "tone",
            JudgeKind::Authority => "authority",
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

    /// Build a router from parsed TOML config. Reads API keys from the
    /// env vars named in each provider's `api_key_env` at this point —
    /// missing keys produce a `RouterBuildError::MissingEnv`.
    pub fn from_config(config: &RouterConfig) -> Result<Self, RouterBuildError> {
        let mut providers: HashMap<String, Arc<dyn LlmClient>> = HashMap::new();
        for (id, p) in &config.providers {
            let key = std::env::var(&p.api_key_env)
                .map_err(|_| RouterBuildError::MissingEnv(p.api_key_env.clone()))?;
            let client: Arc<dyn LlmClient> = match p.kind.as_str() {
                "openai" => {
                    let mut c = OpenAiClient::new(key)
                        .map_err(|e| RouterBuildError::Provider(e.to_string()))?;
                    if let Some(base) = &p.base_url {
                        c = c.with_base_url(base);
                    }
                    Arc::new(c)
                }
                "openrouter" => {
                    let mut c = OpenRouterClient::new(key)
                        .map_err(|e| RouterBuildError::Provider(e.to_string()))?;
                    if let Some(base) = &p.base_url {
                        c = c.with_base_url(base);
                    }
                    Arc::new(c)
                }
                other => return Err(RouterBuildError::UnknownProviderKind(other.into())),
            };
            providers.insert(id.clone(), client);
        }

        let mut routes = HashMap::new();
        for (name, route) in &config.routes {
            let kind = match name.as_str() {
                "hallucination" => JudgeKind::Hallucination,
                "tone" => JudgeKind::Tone,
                "authority" => JudgeKind::Authority,
                other => return Err(RouterBuildError::UnknownJudgeKind(other.into())),
            };
            // Validate referenced providers exist now so misconfigs fail
            // at boot, not on the first request.
            if !providers.contains_key(&route.primary.provider) {
                return Err(RouterBuildError::UnknownProvider(
                    route.primary.provider.clone(),
                ));
            }
            if let Some(f) = &route.fallback {
                if !providers.contains_key(&f.provider) {
                    return Err(RouterBuildError::UnknownProvider(f.provider.clone()));
                }
            }
            routes.insert(
                kind,
                ResolvedRoute {
                    primary: route.primary.clone(),
                    fallback: route.fallback.clone(),
                },
            );
        }

        let budget = TokenBudget::new(config.budgets.default_monthly_tokens);
        for (tenant, limit) in &config.budgets.tenants {
            budget.set_tenant_limit(tenant.clone(), *limit);
        }

        Ok(Self::new(providers, routes, Arc::new(budget)))
    }

    pub fn budget(&self) -> &TokenBudget {
        &self.budget
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
        client.complete(&target.model, prompt, schema, deadline).await
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
    #[error("unknown judge kind `{0}` (expected hallucination|tone|authority)")]
    UnknownJudgeKind(String),
    #[error("route references unknown provider `{0}`")]
    UnknownProvider(String),
    #[error("provider init failed: {0}")]
    Provider(String),
}

#[cfg(test)]
mod tests {
    use super::*;
    use async_trait::async_trait;
    use serde_json::json;
    use std::sync::atomic::{AtomicUsize, Ordering};

    fn schema() -> JsonSchema {
        JsonSchema {
            name: "Verdict".into(),
            schema: json!({"type":"object"}),
        }
    }

    /// Mock client whose behaviour is configured at construction time.
    /// Tracks call count so tests can assert "primary called once,
    /// fallback called zero times" and similar.
    struct MockClient {
        out: Option<LlmOutput>,
        err: Option<LlmError>,
        calls: Arc<AtomicUsize>,
    }

    impl MockClient {
        fn ok(prompt_tokens: u32, completion_tokens: u32) -> (Self, Arc<AtomicUsize>) {
            let calls = Arc::new(AtomicUsize::new(0));
            (
                Self {
                    out: Some(LlmOutput {
                        json: json!({"ok": true}),
                        prompt_tokens,
                        completion_tokens,
                    }),
                    err: None,
                    calls: calls.clone(),
                },
                calls,
            )
        }

        fn fail() -> (Self, Arc<AtomicUsize>) {
            let calls = Arc::new(AtomicUsize::new(0));
            (
                Self {
                    out: None,
                    err: Some(LlmError::Status(500, "boom".into())),
                    calls: calls.clone(),
                },
                calls,
            )
        }
    }

    #[async_trait]
    impl LlmClient for MockClient {
        async fn complete(
            &self,
            _model: &str,
            _prompt: &str,
            _schema: &JsonSchema,
            _deadline: Duration,
        ) -> Result<LlmOutput, LlmError> {
            self.calls.fetch_add(1, Ordering::SeqCst);
            if let Some(e) = &self.err {
                return Err(match e {
                    LlmError::Status(c, b) => LlmError::Status(*c, b.clone()),
                    LlmError::Http(s) => LlmError::Http(s.clone()),
                    _ => LlmError::Http("mock".into()),
                });
            }
            Ok(self.out.clone().expect("mock out").clone_for_test())
        }
    }

    impl LlmOutput {
        fn clone_for_test(self) -> Self {
            self
        }
    }

    fn target(provider: &str, model: &str) -> ProviderTarget {
        ProviderTarget {
            provider: provider.into(),
            model: model.into(),
            deadline_ms: 1_000,
        }
    }

    #[tokio::test]
    async fn primary_success_records_budget_and_skips_fallback() {
        let (primary, p_calls) = MockClient::ok(7, 3);
        let (fallback, f_calls) = MockClient::ok(0, 0);
        let mut providers: HashMap<String, Arc<dyn LlmClient>> = HashMap::new();
        providers.insert("p".into(), Arc::new(primary));
        providers.insert("f".into(), Arc::new(fallback));
        let mut routes = HashMap::new();
        routes.insert(
            JudgeKind::Hallucination,
            ResolvedRoute {
                primary: target("p", "m1"),
                fallback: Some(target("f", "m2")),
            },
        );
        let budget = Arc::new(TokenBudget::new(0)); // unlimited
        let r = LlmRouter::new(providers, routes, budget);

        let out = r
            .judge(JudgeKind::Hallucination, "acme", "prompt", &schema())
            .await
            .expect("ok");
        assert_eq!(out.prompt_tokens, 7);
        assert_eq!(p_calls.load(Ordering::SeqCst), 1);
        assert_eq!(f_calls.load(Ordering::SeqCst), 0);
        assert_eq!(r.budget().used("acme"), 10);
    }

    #[tokio::test]
    async fn primary_failure_falls_back_to_secondary() {
        let (primary, p_calls) = MockClient::fail();
        let (fallback, f_calls) = MockClient::ok(2, 1);
        let mut providers: HashMap<String, Arc<dyn LlmClient>> = HashMap::new();
        providers.insert("p".into(), Arc::new(primary));
        providers.insert("f".into(), Arc::new(fallback));
        let mut routes = HashMap::new();
        routes.insert(
            JudgeKind::Hallucination,
            ResolvedRoute {
                primary: target("p", "m1"),
                fallback: Some(target("f", "m2")),
            },
        );
        let r = LlmRouter::new(providers, routes, Arc::new(TokenBudget::new(0)));

        let out = r
            .judge(JudgeKind::Hallucination, "acme", "prompt", &schema())
            .await
            .expect("fallback ok");
        assert_eq!(out.prompt_tokens, 2);
        assert_eq!(p_calls.load(Ordering::SeqCst), 1);
        assert_eq!(f_calls.load(Ordering::SeqCst), 1);
        assert_eq!(r.budget().used("acme"), 3);
    }

    #[tokio::test]
    async fn no_fallback_propagates_primary_error() {
        let (primary, _) = MockClient::fail();
        let mut providers: HashMap<String, Arc<dyn LlmClient>> = HashMap::new();
        providers.insert("p".into(), Arc::new(primary));
        let mut routes = HashMap::new();
        routes.insert(
            JudgeKind::Tone,
            ResolvedRoute {
                primary: target("p", "m1"),
                fallback: None,
            },
        );
        let r = LlmRouter::new(providers, routes, Arc::new(TokenBudget::new(0)));
        let err = r
            .judge(JudgeKind::Tone, "acme", "p", &schema())
            .await
            .unwrap_err();
        assert!(matches!(err, LlmError::Status(500, _)));
    }

    #[tokio::test]
    async fn over_budget_blocks_request_before_calling_provider() {
        let (primary, p_calls) = MockClient::ok(100, 100);
        let mut providers: HashMap<String, Arc<dyn LlmClient>> = HashMap::new();
        providers.insert("p".into(), Arc::new(primary));
        let mut routes = HashMap::new();
        routes.insert(
            JudgeKind::Hallucination,
            ResolvedRoute {
                primary: target("p", "m1"),
                fallback: None,
            },
        );
        let budget = TokenBudget::new(10);
        budget.record("acme", 11); // already over
        let r = LlmRouter::new(providers, routes, Arc::new(budget));

        let err = r
            .judge(JudgeKind::Hallucination, "acme", "p", &schema())
            .await
            .unwrap_err();
        assert!(matches!(err, LlmError::BudgetExceeded));
        assert_eq!(p_calls.load(Ordering::SeqCst), 0, "provider must not be called");
    }

    #[tokio::test]
    async fn missing_route_yields_http_error() {
        let providers: HashMap<String, Arc<dyn LlmClient>> = HashMap::new();
        let routes: HashMap<JudgeKind, ResolvedRoute> = HashMap::new();
        let r = LlmRouter::new(providers, routes, Arc::new(TokenBudget::new(0)));
        let err = r
            .judge(JudgeKind::Authority, "acme", "p", &schema())
            .await
            .unwrap_err();
        assert!(matches!(err, LlmError::Http(_)));
    }

    #[test]
    fn build_from_config_validates_referenced_providers() {
        // A route references a provider that wasn't declared in [providers].
        let bad = r#"
[providers.openai]
kind = "openai"
api_key_env = "OPENAI_API_KEY"

[routes.hallucination]
primary = { provider = "ghost", model = "x", deadline_ms = 100 }
"#;
        std::env::set_var("OPENAI_API_KEY", "test-key");
        let cfg = RouterConfig::parse(bad).unwrap();
        let err = LlmRouter::from_config(&cfg).unwrap_err();
        assert!(matches!(err, RouterBuildError::UnknownProvider(_)));
    }
}
