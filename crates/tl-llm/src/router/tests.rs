use std::collections::HashMap;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use std::time::Duration;

use async_trait::async_trait;
use serde_json::json;
use tokio::sync::Notify;

use crate::budget::TokenBudget;
use crate::client::{JsonSchema, LlmClient, LlmError, LlmOutput};
use crate::config::{ProviderTarget, RouterConfig};

use super::*;

fn schema() -> JsonSchema {
    JsonSchema {
        name: "AuthorizationEffect".into(),
        schema: json!({"type":"object"}),
    }
}

/// Mock client whose behaviour is configured at construction time.
/// Tracks call count so tests can assert primary/fallback call behavior.
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
        Ok(self.out.clone().expect("mock out"))
    }
}

struct BlockingClient {
    calls: Arc<AtomicUsize>,
    entered: Arc<Notify>,
    release: Arc<Notify>,
}

#[async_trait]
impl LlmClient for BlockingClient {
    async fn complete(
        &self,
        _model: &str,
        _prompt: &str,
        _schema: &JsonSchema,
        _deadline: Duration,
    ) -> Result<LlmOutput, LlmError> {
        self.calls.fetch_add(1, Ordering::SeqCst);
        self.entered.notify_one();
        self.release.notified().await;
        Ok(LlmOutput {
            json: json!({"ok": true}),
            prompt_tokens: 1,
            completion_tokens: 0,
        })
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
    let budget = Arc::new(TokenBudget::new(0));
    let router = LlmRouter::new(providers, routes, budget);

    let out = router
        .judge(JudgeKind::Hallucination, "acme", "prompt", &schema())
        .await
        .expect("ok");
    assert_eq!(out.prompt_tokens, 7);
    assert_eq!(p_calls.load(Ordering::SeqCst), 1);
    assert_eq!(f_calls.load(Ordering::SeqCst), 0);
    assert_eq!(router.budget().used("acme"), 10);
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
    let router = LlmRouter::new(providers, routes, Arc::new(TokenBudget::new(0)));

    let out = router
        .judge(JudgeKind::Hallucination, "acme", "prompt", &schema())
        .await
        .expect("fallback ok");
    assert_eq!(out.prompt_tokens, 2);
    assert_eq!(p_calls.load(Ordering::SeqCst), 1);
    assert_eq!(f_calls.load(Ordering::SeqCst), 1);
    assert_eq!(router.budget().used("acme"), 3);
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
    let router = LlmRouter::new(providers, routes, Arc::new(TokenBudget::new(0)));
    let err = router
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
    budget.record("acme", 11);
    let router = LlmRouter::new(providers, routes, Arc::new(budget));

    let err = router
        .judge(JudgeKind::Hallucination, "acme", "p", &schema())
        .await
        .unwrap_err();
    assert!(matches!(err, LlmError::BudgetExceeded));
    assert_eq!(
        p_calls.load(Ordering::SeqCst),
        0,
        "provider must not be called"
    );
}

#[tokio::test]
async fn concurrent_requests_cannot_claim_the_same_remaining_budget() {
    let calls = Arc::new(AtomicUsize::new(0));
    let entered = Arc::new(Notify::new());
    let release = Arc::new(Notify::new());
    let mut providers: HashMap<String, Arc<dyn LlmClient>> = HashMap::new();
    providers.insert(
        "p".into(),
        Arc::new(BlockingClient {
            calls: calls.clone(),
            entered: entered.clone(),
            release: release.clone(),
        }),
    );
    let mut routes = HashMap::new();
    routes.insert(
        JudgeKind::Hallucination,
        ResolvedRoute {
            primary: target("p", "m1"),
            fallback: None,
        },
    );
    let budget = TokenBudget::new(10);
    budget.record("acme", 9);
    let router = Arc::new(LlmRouter::new(providers, routes, Arc::new(budget)));

    let first_router = router.clone();
    let first = tokio::spawn(async move {
        first_router
            .judge(JudgeKind::Hallucination, "acme", "first", &schema())
            .await
    });
    entered.notified().await;

    let second = tokio::time::timeout(
        Duration::from_millis(100),
        router.judge(JudgeKind::Hallucination, "acme", "second", &schema()),
    )
    .await
    .expect("budget rejection must not wait for the provider")
    .unwrap_err();
    assert!(matches!(second, LlmError::BudgetExceeded));
    assert_eq!(
        calls.load(Ordering::SeqCst),
        1,
        "only the admitted request may reach the provider"
    );

    release.notify_one();
    first.await.expect("first task").expect("first request");
    assert_eq!(router.budget().used("acme"), 10);
}

#[tokio::test]
async fn missing_route_yields_http_error() {
    let providers: HashMap<String, Arc<dyn LlmClient>> = HashMap::new();
    let routes: HashMap<JudgeKind, ResolvedRoute> = HashMap::new();
    let router = LlmRouter::new(providers, routes, Arc::new(TokenBudget::new(0)));
    let err = router
        .judge(JudgeKind::Authority, "acme", "p", &schema())
        .await
        .unwrap_err();
    assert!(matches!(err, LlmError::Http(_)));
}

#[tokio::test]
async fn semantic_policy_route_uses_configured_provider() {
    let (primary, calls) = MockClient::ok(4, 2);
    let mut providers: HashMap<String, Arc<dyn LlmClient>> = HashMap::new();
    providers.insert("p".into(), Arc::new(primary));
    let mut routes = HashMap::new();
    routes.insert(
        JudgeKind::SemanticPolicy,
        ResolvedRoute {
            primary: target("p", "semantic-model"),
            fallback: None,
        },
    );
    let router = LlmRouter::new(providers, routes, Arc::new(TokenBudget::new(0)));

    let out = router
        .judge(JudgeKind::SemanticPolicy, "acme", "prompt", &schema())
        .await
        .expect("semantic policy route");

    assert_eq!(out.prompt_tokens, 4);
    assert_eq!(out.completion_tokens, 2);
    assert_eq!(calls.load(Ordering::SeqCst), 1);
    assert_eq!(router.budget().used("acme"), 6);
}

#[test]
fn build_from_config_validates_referenced_providers() {
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

#[test]
fn build_from_config_accepts_semantic_policy_route() {
    let src = r#"
[providers.openai]
kind = "openai"
api_key_env = "OPENAI_API_KEY"

[routes.semantic_policy]
primary = { provider = "openai", model = "gpt-4o-mini", deadline_ms = 700 }
"#;
    std::env::set_var("OPENAI_API_KEY", "test-key");
    let cfg = RouterConfig::parse(src).unwrap();
    let router = LlmRouter::from_config(&cfg).expect("semantic policy route parses");

    assert!(router.has_route(JudgeKind::SemanticPolicy));
}
