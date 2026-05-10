//! Verify the canonical `config/llm-routing.toml` we ship parses
//! cleanly via `RouterConfig::from_path`. If the committed file drifts
//! out of sync with the schema this test fails loudly.

use tl_llm::RouterConfig;

#[test]
fn committed_example_config_parses() {
    // CARGO_MANIFEST_DIR points at crates/tl-llm during this test.
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(std::path::Path::parent)
        .expect("workspace root")
        .join("config/llm-routing.toml");
    let cfg = RouterConfig::from_path(&path).expect("parse");

    assert!(cfg.providers.contains_key("openai"));
    assert!(cfg.providers.contains_key("openrouter"));
    assert!(cfg.routes.contains_key("hallucination"));
    assert!(cfg.routes.contains_key("tone"));
    assert!(cfg.routes.contains_key("authority"));
    assert!(cfg.budgets.default_monthly_tokens > 0);
}
