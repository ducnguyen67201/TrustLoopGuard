//! Verify the embedded canonical routing manifest stays in sync with the schema.

use tl_llm::{ReasoningEffort, RouterConfig, ROUTER_CONFIG_SCHEMA_VERSION};

#[test]
fn committed_example_config_parses() {
    let cfg = RouterConfig::bundled().expect("parse bundled manifest");

    assert_eq!(cfg.schema_version, ROUTER_CONFIG_SCHEMA_VERSION);
    assert_eq!(cfg.providers["openai"].kind, "openai");
    assert_eq!(cfg.providers["openai"].api_key_env, "OPENAI_API_KEY");
    assert_eq!(cfg.budgets.default_monthly_tokens, 10_000_000);
    assert!(cfg.budgets.tenants.is_empty());

    let expected = [
        ("hallucination", "gpt-4o-mini", 600),
        ("tone", "gpt-4o-mini", 300),
        ("authority", "gpt-4o", 700),
        ("semantic_policy", "gpt-4o-mini", 700),
        ("policy_draft", "gpt-4o-mini", 30_000),
        ("policy_ai_edit", "gpt-4o-mini", 30_000),
        ("guardrail_generation", "gpt-4o-mini", 60_000),
        ("github_integration", "gpt-4o-mini", 60_000),
        ("demo_default", "gpt-4.1-mini", 30_000),
        ("demo_dispute", "gpt-4o-mini", 30_000),
        ("demo_livekit", "gpt-4o-mini", 30_000),
    ];
    assert_eq!(cfg.routes.len(), expected.len());
    for (name, model, deadline_ms) in expected {
        let route = &cfg.routes[name];
        assert_eq!(route.primary.provider, "openai", "provider for {name}");
        assert_eq!(route.primary.model, model, "model for {name}");
        assert_eq!(
            route.primary.deadline_ms, deadline_ms,
            "deadline for {name}"
        );
        assert!(
            route
                .description
                .as_deref()
                .is_some_and(|description| !description.trim().is_empty()),
            "description for {name}"
        );
        if name != "authority" {
            assert!(route.fallback.is_none(), "unexpected fallback for {name}");
        }
    }

    let authority_fallback = cfg.routes["authority"]
        .fallback
        .as_ref()
        .expect("authority fallback");
    assert_eq!(authority_fallback.provider, "openai");
    assert_eq!(authority_fallback.model, "gpt-4o-mini");
    assert_eq!(authority_fallback.deadline_ms, 700);
    assert!(cfg
        .routes
        .values()
        .all(|route| route.primary.reasoning_effort.is_none()));
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
