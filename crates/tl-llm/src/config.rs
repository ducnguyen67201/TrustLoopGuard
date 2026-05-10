//! TOML config schema for `LlmRouter::from_toml`. Loaded once at server
//! boot. Hot-reload is deferred to v1 — restart to pick up changes.
//!
//! Shape:
//!
//! ```toml
//! [providers.openai]
//! kind = "openai"
//! api_key_env = "OPENAI_API_KEY"
//!
//! [providers.openrouter]
//! kind = "openrouter"
//! api_key_env = "OPENROUTER_API_KEY"
//!
//! [routes.hallucination]
//! primary  = { provider = "openai",     model = "gpt-4o-mini",       deadline_ms = 600 }
//! fallback = { provider = "openrouter", model = "openai/gpt-4o-mini", deadline_ms = 800 }
//!
//! [routes.tone]
//! primary = { provider = "openrouter", model = "openai/gpt-4o-mini", deadline_ms = 300 }
//!
//! [routes.authority]
//! primary  = { provider = "openai", model = "gpt-4o",      deadline_ms = 700 }
//! fallback = { provider = "openai", model = "gpt-4o-mini", deadline_ms = 700 }
//!
//! [budgets]
//! default_monthly_tokens = 10_000_000
//!
//! [budgets.tenants]
//! acme = 100_000_000
//! ```

use std::collections::HashMap;

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RouterConfig {
    pub providers: HashMap<String, ProviderConfig>,
    pub routes: HashMap<String, RouteConfig>,
    #[serde(default)]
    pub budgets: BudgetConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProviderConfig {
    /// One of `"openai"`, `"openrouter"`. Future: `"tenant:<id>"` for BYOK.
    pub kind: String,
    /// Env var name to read the API key from. Reading is deferred to
    /// router build time so configs are safe to commit and copy.
    pub api_key_env: String,
    /// Optional override; defaults to the provider's canonical URL.
    #[serde(default)]
    pub base_url: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RouteConfig {
    pub primary: ProviderTarget,
    #[serde(default)]
    pub fallback: Option<ProviderTarget>,
    #[serde(default)]
    pub cache_ttl_seconds: Option<u32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProviderTarget {
    pub provider: String,
    pub model: String,
    pub deadline_ms: u32,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct BudgetConfig {
    /// Tokens per tenant per month when no override is set. `0` = unlimited.
    #[serde(default)]
    pub default_monthly_tokens: u64,
    /// Per-tenant overrides. Key is the tenant id.
    #[serde(default)]
    pub tenants: HashMap<String, u64>,
}

#[derive(Debug, thiserror::Error)]
pub enum ConfigError {
    #[error("io: {0}")]
    Io(#[from] std::io::Error),
    #[error("toml parse: {0}")]
    Parse(#[from] toml::de::Error),
}

impl RouterConfig {
    pub fn parse(src: &str) -> Result<Self, ConfigError> {
        Ok(toml::from_str(src)?)
    }

    pub fn from_path(path: impl AsRef<std::path::Path>) -> Result<Self, ConfigError> {
        Self::parse(&std::fs::read_to_string(path)?)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE: &str = r#"
[providers.openai]
kind = "openai"
api_key_env = "OPENAI_API_KEY"

[providers.openrouter]
kind = "openrouter"
api_key_env = "OPENROUTER_API_KEY"

[routes.hallucination]
primary  = { provider = "openai", model = "gpt-4o-mini", deadline_ms = 600 }
fallback = { provider = "openrouter", model = "openai/gpt-4o-mini", deadline_ms = 800 }

[routes.tone]
primary = { provider = "openrouter", model = "openai/gpt-4o-mini", deadline_ms = 300 }

[budgets]
default_monthly_tokens = 10_000_000

[budgets.tenants]
acme = 100_000_000
"#;

    #[test]
    fn round_trips_sample_config() {
        let cfg = RouterConfig::parse(SAMPLE).expect("parse");
        assert_eq!(cfg.providers.len(), 2);
        assert_eq!(cfg.providers["openai"].kind, "openai");
        assert_eq!(cfg.providers["openai"].api_key_env, "OPENAI_API_KEY");

        let hallu = &cfg.routes["hallucination"];
        assert_eq!(hallu.primary.provider, "openai");
        assert_eq!(hallu.primary.model, "gpt-4o-mini");
        assert_eq!(hallu.primary.deadline_ms, 600);
        assert!(hallu.fallback.is_some());
        assert_eq!(hallu.fallback.as_ref().unwrap().provider, "openrouter");

        let tone = &cfg.routes["tone"];
        assert!(tone.fallback.is_none());

        assert_eq!(cfg.budgets.default_monthly_tokens, 10_000_000);
        assert_eq!(cfg.budgets.tenants["acme"], 100_000_000);
    }

    #[test]
    fn missing_required_field_errors() {
        let bad = r#"
[providers.openai]
kind = "openai"
"#; // api_key_env missing
        assert!(matches!(
            RouterConfig::parse(bad),
            Err(ConfigError::Parse(_))
        ));
    }

    #[test]
    fn empty_budgets_section_uses_default() {
        let minimal = r#"
[providers.openai]
kind = "openai"
api_key_env = "OPENAI_API_KEY"

[routes.hallucination]
primary = { provider = "openai", model = "gpt-4o-mini", deadline_ms = 600 }
"#;
        let cfg = RouterConfig::parse(minimal).expect("parse");
        assert_eq!(cfg.budgets.default_monthly_tokens, 0); // 0 = unlimited
        assert!(cfg.budgets.tenants.is_empty());
    }
}
