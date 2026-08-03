//! Verify the embedded canonical routing manifest stays in sync with the schema.

use tl_llm::{LlmRouter, ReasoningEffort, RouterConfig, ROUTER_CONFIG_SCHEMA_VERSION};

#[test]
fn committed_example_config_parses() {
    let cfg = RouterConfig::bundled().expect("parse bundled manifest");

    assert_eq!(cfg.schema_version, ROUTER_CONFIG_SCHEMA_VERSION);
    assert_eq!(cfg.providers["openai"].kind, "openai");
    assert_eq!(cfg.providers["openai"].api_key_env, "OPENAI_API_KEY");
    assert!(cfg.budgets.default_monthly_tokens > 0);
    assert!(cfg.budgets.tenants.is_empty());

    let route_names = [
        "hallucination",
        "tone",
        "authority",
        "semantic_policy",
        "policy_draft",
        "policy_ai_edit",
        "guardrail_generation",
        "github_integration",
        "demo_default",
        "demo_dispute",
        "demo_livekit",
    ];
    assert_eq!(cfg.routes.len(), route_names.len());
    for name in route_names {
        let route = &cfg.routes[name];
        assert!(cfg.providers.contains_key(&route.primary.provider));
        assert!(!route.primary.model.trim().is_empty());
        assert!(route.primary.deadline_ms > 0);
        assert!(route
            .description
            .as_deref()
            .is_some_and(|description| !description.trim().is_empty()));
        if let Some(fallback) = &route.fallback {
            assert!(cfg.providers.contains_key(&fallback.provider));
            assert!(!fallback.model.trim().is_empty());
            assert!(fallback.deadline_ms > 0);
        }
    }
}

#[test]
fn committed_example_config_builds_the_router() {
    let previous_key = std::env::var_os("OPENAI_API_KEY");
    std::env::set_var("OPENAI_API_KEY", "bundled-manifest-test-key");

    let config = RouterConfig::bundled().expect("parse bundled manifest");
    let result = LlmRouter::from_config(&config);

    if let Some(previous_key) = previous_key {
        std::env::set_var("OPENAI_API_KEY", previous_key);
    } else {
        std::env::remove_var("OPENAI_API_KEY");
    }
    result.expect("build router from bundled manifest");
}

#[test]
fn invalid_reasoning_effort_is_rejected() {
    let source = r#"{
      "schema_version": 1,
      "providers": {},
      "routes": {
        "demo_default": {
          "primary": {
            "provider": "openai",
            "model": "gpt-5.6-luna",
            "deadline_ms": 30000,
            "reasoning_effort": "fast"
          }
        }
      }
    }"#;

    assert!(RouterConfig::parse(source).is_err());
}

#[test]
fn reasoning_effort_values_are_typed() {
    let source = r#"{
      "schema_version": 1,
      "providers": {},
      "routes": {
        "demo_default": {
          "primary": {
            "provider": "openai",
            "model": "gpt-5.6-luna",
            "deadline_ms": 30000,
            "reasoning_effort": "xhigh"
          }
        }
      }
    }"#;
    let config = RouterConfig::parse(source).expect("typed effort");
    assert_eq!(
        config.routes["demo_default"].primary.reasoning_effort,
        Some(ReasoningEffort::XHigh)
    );
}
