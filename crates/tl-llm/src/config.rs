//! Versioned JSON configuration for [`crate::LlmRouter`].
//!
//! The canonical manifest is `config/llm-routing.json`. It is embedded in the
//! crate at compile time so production deployments cannot lose routing because
//! a runtime file was omitted from an image.

use std::collections::HashMap;

use serde::{Deserialize, Serialize};

pub const ROUTER_CONFIG_SCHEMA_VERSION: u32 = 1;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RouterConfig {
    pub schema_version: u32,
    pub providers: HashMap<String, ProviderConfig>,
    pub routes: HashMap<String, RouteConfig>,
    #[serde(default)]
    pub budgets: BudgetConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ProviderConfig {
    /// One of `"openai"`, `"openrouter"`. Future: `"tenant:<id>"` for BYOK.
    pub kind: String,
    /// Environment variable that contains the provider credential.
    pub api_key_env: String,
    /// Optional override; defaults to the provider's canonical URL.
    #[serde(default)]
    pub base_url: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RouteConfig {
    #[serde(default)]
    pub description: Option<String>,
    pub primary: ProviderTarget,
    #[serde(default)]
    pub fallback: Option<ProviderTarget>,
    #[serde(default)]
    pub cache_ttl_seconds: Option<u32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ProviderTarget {
    pub provider: String,
    pub model: String,
    pub deadline_ms: u32,
    #[serde(default)]
    pub reasoning_effort: Option<ReasoningEffort>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ReasoningEffort {
    None,
    Low,
    Medium,
    High,
    #[serde(rename = "xhigh")]
    XHigh,
    Max,
}

impl ReasoningEffort {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::None => "none",
            Self::Low => "low",
            Self::Medium => "medium",
            Self::High => "high",
            Self::XHigh => "xhigh",
            Self::Max => "max",
        }
    }
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct BudgetConfig {
    /// Tokens per tenant per month when no override is set. `0` = unlimited.
    #[serde(default)]
    pub default_monthly_tokens: u64,
    /// Per-tenant overrides keyed by tenant id.
    #[serde(default)]
    pub tenants: HashMap<String, u64>,
}

#[derive(Debug, thiserror::Error)]
pub enum ConfigError {
    #[error("json parse: {0}")]
    Parse(#[from] serde_json::Error),
    #[error("unsupported llm-routing schema version {actual}; expected {expected}")]
    UnsupportedSchemaVersion { actual: u32, expected: u32 },
}

impl RouterConfig {
    pub fn parse(src: &str) -> Result<Self, ConfigError> {
        #[derive(Deserialize)]
        struct SchemaHeader {
            schema_version: u32,
        }

        let header: SchemaHeader = serde_json::from_str(src)?;
        if header.schema_version != ROUTER_CONFIG_SCHEMA_VERSION {
            return Err(ConfigError::UnsupportedSchemaVersion {
                actual: header.schema_version,
                expected: ROUTER_CONFIG_SCHEMA_VERSION,
            });
        }
        Ok(serde_json::from_str(src)?)
    }

    pub fn bundled() -> Result<Self, ConfigError> {
        Self::parse(include_str!("../../../config/llm-routing.json"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE: &str = r#"{
      "schema_version": 1,
      "providers": {
        "openai": { "kind": "openai", "api_key_env": "OPENAI_API_KEY" }
      },
      "routes": {
        "hallucination": {
          "primary": {
            "provider": "openai",
            "model": "gpt-4o-mini",
            "deadline_ms": 600,
            "reasoning_effort": "low"
          }
        }
      },
      "budgets": {
        "default_monthly_tokens": 10000000,
        "tenants": { "acme": 100000000 }
      }
    }"#;

    #[test]
    fn parses_versioned_json_config() {
        let config = RouterConfig::parse(SAMPLE).expect("parse");
        assert_eq!(config.schema_version, ROUTER_CONFIG_SCHEMA_VERSION);
        assert_eq!(config.providers["openai"].kind, "openai");
        let target = &config.routes["hallucination"].primary;
        assert_eq!(target.model, "gpt-4o-mini");
        assert_eq!(target.reasoning_effort, Some(ReasoningEffort::Low));
        assert_eq!(config.budgets.tenants["acme"], 100_000_000);
    }

    #[test]
    fn rejects_unknown_fields() {
        let invalid = SAMPLE.replace(
            "\"schema_version\": 1",
            "\"schema_version\": 1, \"typo\": true",
        );
        assert!(matches!(
            RouterConfig::parse(&invalid),
            Err(ConfigError::Parse(_))
        ));
    }

    #[test]
    fn rejects_unsupported_schema_version() {
        let invalid = SAMPLE.replace("\"schema_version\": 1", "\"schema_version\": 2");
        assert!(matches!(
            RouterConfig::parse(&invalid),
            Err(ConfigError::UnsupportedSchemaVersion { actual: 2, .. })
        ));
    }

    #[test]
    fn rejects_future_schema_before_future_fields() {
        let invalid = SAMPLE.replace(
            "\"schema_version\": 1",
            "\"schema_version\": 2, \"future_option\": true",
        );
        assert!(matches!(
            RouterConfig::parse(&invalid),
            Err(ConfigError::UnsupportedSchemaVersion { actual: 2, .. })
        ));
    }

    #[test]
    fn bundled_manifest_parses() {
        let config = RouterConfig::bundled().expect("bundled manifest");
        assert_eq!(config.schema_version, ROUTER_CONFIG_SCHEMA_VERSION);
    }
}
