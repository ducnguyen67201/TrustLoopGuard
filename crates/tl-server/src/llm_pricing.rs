//! Model → price table for LLM gateway metering.
//!
//! Prices are integers in USD minor units (cents) per **1M tokens**,
//! input and output separately. A built-in default table covers the
//! design-partner models; `TL_LLM_PRICING_PATH` points at a JSON file
//! that overrides or extends it, loaded once at state build:
//!
//! ```json
//! { "gpt-4o": { "input_per_million_minor": 250, "output_per_million_minor": 1000 } }
//! ```
//!
//! Unknown model → `None`: the caller meters tokens with cost 0 and
//! warns. Honesty beats availability of a guess — never block on a
//! missing price.

use std::collections::HashMap;

use serde::Deserialize;

/// Env var pointing at the JSON override file.
pub const TL_LLM_PRICING_PATH: &str = "TL_LLM_PRICING_PATH";

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
pub struct ModelPrice {
    /// USD minor units per 1M prompt tokens.
    pub input_per_million_minor: i64,
    /// USD minor units per 1M completion tokens.
    pub output_per_million_minor: i64,
}

/// Built-in defaults, USD cents per 1M tokens. Placeholders for the
/// design-partner model families — override via `TL_LLM_PRICING_PATH`
/// when the real contract prices differ.
/// `// ponytail: prices go stale; the override file is the real source of truth`
const DEFAULT_PRICES: &[(&str, ModelPrice)] = &[
    (
        "deepseek-chat",
        ModelPrice {
            input_per_million_minor: 27,
            output_per_million_minor: 110,
        },
    ),
    (
        "deepseek-reasoner",
        ModelPrice {
            input_per_million_minor: 55,
            output_per_million_minor: 219,
        },
    ),
    (
        "qwen-max",
        ModelPrice {
            input_per_million_minor: 160,
            output_per_million_minor: 640,
        },
    ),
    (
        "qwen-plus",
        ModelPrice {
            input_per_million_minor: 40,
            output_per_million_minor: 120,
        },
    ),
    (
        "qwen-turbo",
        ModelPrice {
            input_per_million_minor: 5,
            output_per_million_minor: 20,
        },
    ),
    (
        "qwen2.5-72b-instruct",
        ModelPrice {
            input_per_million_minor: 40,
            output_per_million_minor: 120,
        },
    ),
    (
        "gemma-2-27b-it",
        ModelPrice {
            input_per_million_minor: 20,
            output_per_million_minor: 20,
        },
    ),
    (
        "gemma-3-27b-it",
        ModelPrice {
            input_per_million_minor: 20,
            output_per_million_minor: 20,
        },
    ),
    (
        "gpt-4o",
        ModelPrice {
            input_per_million_minor: 250,
            output_per_million_minor: 1000,
        },
    ),
    (
        "gpt-4o-mini",
        ModelPrice {
            input_per_million_minor: 15,
            output_per_million_minor: 60,
        },
    ),
];

const TOKENS_PER_PRICE_UNIT: i64 = 1_000_000;

/// Characters that may separate an Azure/gateway deployment prefix from
/// the model name (`my-deploy/gpt-4o`, `azure:gpt-4o`).
const MODEL_PREFIX_SEPARATORS: &[char] = &['/', ':'];

#[derive(Debug, Clone)]
pub struct LlmPricingTable {
    prices: HashMap<String, ModelPrice>,
}

impl Default for LlmPricingTable {
    fn default() -> Self {
        Self {
            prices: DEFAULT_PRICES
                .iter()
                .map(|(model, price)| ((*model).to_string(), *price))
                .collect(),
        }
    }
}

impl LlmPricingTable {
    /// Defaults merged with the `TL_LLM_PRICING_PATH` JSON override, if
    /// configured. A broken override file is logged and ignored — the
    /// gateway must not fail to boot over a pricing typo.
    pub fn from_env() -> Self {
        let mut table = Self::default();
        let Ok(path) = std::env::var(TL_LLM_PRICING_PATH) else {
            return table;
        };
        if path.trim().is_empty() {
            return table;
        }
        match std::fs::read_to_string(&path)
            .map_err(|e| e.to_string())
            .and_then(|raw| {
                serde_json::from_str::<HashMap<String, ModelPrice>>(&raw).map_err(|e| e.to_string())
            }) {
            Ok(overrides) => {
                let count = overrides.len();
                for (model, price) in overrides {
                    table.prices.insert(normalize_model(&model), price);
                }
                tracing::info!(path, count, "llm pricing overrides loaded");
            }
            Err(error) => {
                tracing::error!(
                    path,
                    error,
                    "failed to load llm pricing overrides; using built-in defaults"
                );
            }
        }
        table
    }

    /// Price a call in USD minor units: `(tokens × price_per_million) /
    /// 1M` per side, integer math rounding down, saturating. `None`
    /// when the model has no price entry.
    pub fn cost_minor(
        &self,
        model: &str,
        prompt_tokens: i64,
        completion_tokens: i64,
    ) -> Option<i64> {
        let price = self.resolve(model)?;
        let input = prompt_tokens
            .max(0)
            .saturating_mul(price.input_per_million_minor)
            / TOKENS_PER_PRICE_UNIT;
        let output = completion_tokens
            .max(0)
            .saturating_mul(price.output_per_million_minor)
            / TOKENS_PER_PRICE_UNIT;
        Some(input.saturating_add(output))
    }

    /// Normalized lookup with longest-suffix matching so Azure-style
    /// deployment prefixes (`my-deploy/gpt-4o`) still price. The raw
    /// model string is preserved in the event row regardless.
    fn resolve(&self, model: &str) -> Option<ModelPrice> {
        let normalized = normalize_model(model);
        if let Some(price) = self.prices.get(&normalized) {
            return Some(*price);
        }
        self.prices
            .iter()
            .filter(|(key, _)| {
                normalized
                    .strip_suffix(key.as_str())
                    .and_then(|prefix| prefix.chars().last())
                    .is_some_and(|boundary| MODEL_PREFIX_SEPARATORS.contains(&boundary))
            })
            .max_by_key(|(key, _)| key.len())
            .map(|(_, price)| *price)
    }
}

fn normalize_model(model: &str) -> String {
    model.trim().to_ascii_lowercase()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn known_model_prices_exactly() {
        let table = LlmPricingTable::default();
        // 1_234_000 prompt @ 27/1M = 33.318 → 33; 567_000 completion
        // @ 110/1M = 62.37 → 62. Integer math rounds down per side.
        assert_eq!(
            table.cost_minor("deepseek-chat", 1_234_000, 567_000),
            Some(33 + 62)
        );
        // Exactly 1M/1M tokens costs the per-million price.
        assert_eq!(table.cost_minor("gpt-4o", 1_000_000, 1_000_000), Some(1250));
    }

    #[test]
    fn unknown_model_is_none() {
        let table = LlmPricingTable::default();
        assert_eq!(table.cost_minor("mystery-1", 1000, 1000), None);
        // "-mini" is not a separator boundary; gpt-4o must not price a
        // different model.
        assert_eq!(table.cost_minor("customgpt-4o", 1000, 1000), None);
    }

    #[test]
    fn zero_tokens_cost_zero() {
        let table = LlmPricingTable::default();
        assert_eq!(table.cost_minor("gpt-4o", 0, 0), Some(0));
    }

    #[test]
    fn deployment_prefixes_suffix_match() {
        let table = LlmPricingTable::default();
        let direct = table.cost_minor("gpt-4o", 1_000_000, 0);
        assert_eq!(table.cost_minor("my-deploy/gpt-4o", 1_000_000, 0), direct);
        assert_eq!(table.cost_minor("azure:GPT-4o", 1_000_000, 0), direct);
        // Longest suffix wins: gpt-4o-mini is its own price, not gpt-4o's.
        assert_eq!(table.cost_minor("prod/gpt-4o-mini", 1_000_000, 0), Some(15));
    }

    #[test]
    fn saturates_instead_of_overflowing() {
        let table = LlmPricingTable::default();
        // Both multiplications saturate to i64::MAX, then divide.
        let cost = table.cost_minor("gpt-4o", i64::MAX, i64::MAX);
        assert_eq!(cost, Some((i64::MAX / TOKENS_PER_PRICE_UNIT) * 2));
    }

    #[test]
    fn negative_tokens_clamp_to_zero() {
        let table = LlmPricingTable::default();
        assert_eq!(table.cost_minor("gpt-4o", -5, -5), Some(0));
    }
}
