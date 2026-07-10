//! Model → price resolution for LLM gateway metering, plus the
//! `/v1/llm-pricing` CRUD surface.
//!
//! Prices are integers in USD minor units (cents) per **1M tokens**,
//! input and output separately. Workspace-edited rows in
//! `llm_model_prices` win; the built-in default table below is the
//! day-one seed/fallback for models with no workspace row.
//!
//! Unknown model → `None`: the caller meters tokens with cost 0 and
//! warns. Honesty beats availability of a guess — never block on a
//! missing price.

use std::collections::HashMap;
use std::sync::OnceLock;

use async_trait::async_trait;

pub(crate) mod handlers;
mod memory_store;

pub use handlers::{delete_llm_price, list_llm_pricing, put_llm_price, LlmPricingState};
pub use memory_store::MemoryLlmPricingStore;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ModelPrice {
    /// USD minor units per 1M prompt tokens.
    pub input_per_million_minor: i64,
    /// USD minor units per 1M completion tokens.
    pub output_per_million_minor: i64,
}

/// One workspace price row: normalized model key + price.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorkspaceModelPrice {
    pub model: String,
    pub price: ModelPrice,
}

#[derive(Debug, thiserror::Error)]
pub enum LlmPricingStoreError {
    #[error("internal: {0}")]
    Internal(String),
}

/// Workspace-editable model price store. Mirrors the llm_usage trio:
/// trait + memory impl + tl-storage repo behind a postgres adapter.
/// Model keys are normalized (trimmed, lowercase) before they reach the
/// store — see `normalize_model`.
#[async_trait]
pub trait LlmPricingStore: Send + Sync {
    /// Insert or update one workspace model price.
    async fn upsert_price(
        &self,
        workspace_id: &str,
        model: &str,
        input_per_million_minor: i64,
        output_per_million_minor: i64,
    ) -> Result<(), LlmPricingStoreError>;

    /// Delete one workspace model price. Returns whether a row existed.
    async fn delete_price(
        &self,
        workspace_id: &str,
        model: &str,
    ) -> Result<bool, LlmPricingStoreError>;

    /// All workspace price rows, model ascending.
    async fn list_prices(
        &self,
        workspace_id: &str,
    ) -> Result<Vec<WorkspaceModelPrice>, LlmPricingStoreError>;

    /// Exact-match lookup on the normalized model key — one indexed PK
    /// read.
    async fn get_price(
        &self,
        workspace_id: &str,
        model: &str,
    ) -> Result<Option<ModelPrice>, LlmPricingStoreError>;
}

/// Built-in defaults, USD cents per 1M tokens. Placeholders for the
/// design-partner model families — workspaces override per model via
/// `PUT /v1/llm-pricing/{model}` when the real contract prices differ.
/// `// ponytail: prices go stale; workspace rows are the real source of truth`
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
pub(crate) const NANOS_PER_MINOR: i64 = 10_000_000;

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

/// The built-in default table, built once (`OnceLock`: MSRV 1.78 rules
/// out `LazyLock`).
static DEFAULT_TABLE: OnceLock<LlmPricingTable> = OnceLock::new();

pub fn default_table() -> &'static LlmPricingTable {
    DEFAULT_TABLE.get_or_init(LlmPricingTable::default)
}

impl LlmPricingTable {
    /// Price a call in USD minor units. `None` when the model has no
    /// price entry.
    pub fn cost_minor(
        &self,
        model: &str,
        prompt_tokens: i64,
        completion_tokens: i64,
    ) -> Option<i64> {
        self.resolve(model)
            .map(|price| price_tokens(price, prompt_tokens, completion_tokens))
    }

    /// Normalized lookup with longest-suffix matching so Azure-style
    /// deployment prefixes (`my-deploy/gpt-4o`) still price. The raw
    /// model string is preserved in the event row regardless.
    fn resolve(&self, model: &str) -> Option<ModelPrice> {
        let normalized = normalize_model(model);
        if let Some(price) = self.prices.get(&normalized) {
            return Some(*price);
        }
        resolve_suffix(
            &normalized,
            self.prices.iter().map(|(key, price)| (key.as_str(), price)),
        )
    }
}

/// Price a metered gateway call in USD minor units against workspace
/// prices first, then the built-in defaults. `None` when no price
/// matches anywhere — the caller meters tokens with cost 0 and warns.
///
/// Lookup order: workspace exact match (one indexed PK read — the hot
/// path), then the same normalized/suffix matching over the workspace's
/// rows, then the default table. A store read failure logs and falls
/// through to the defaults — pricing must never fail a metered
/// response.
/// `// ponytail: per-call PK lookup; cache if it ever shows up in a profile`
pub async fn cost_minor(
    store: &dyn LlmPricingStore,
    workspace_id: &str,
    model: &str,
    prompt_tokens: i64,
    completion_tokens: i64,
) -> Option<i64> {
    let price = model_price(store, workspace_id, model).await?;
    Some(cost_nanos(price, prompt_tokens, completion_tokens) / NANOS_PER_MINOR)
}

/// Resolve the authoritative price used by both preflight reservations
/// and post-response settlement.
pub(crate) async fn model_price(
    store: &dyn LlmPricingStore,
    workspace_id: &str,
    model: &str,
) -> Option<ModelPrice> {
    resolve_workspace_price(store, workspace_id, model)
        .await
        .or_else(|| default_table().resolve(model))
}

/// Exact USD-nano cost for integer-cent per-million prices. One token
/// at one cent / 1M costs ten USD nanos, so this avoids per-call cent
/// rounding while remaining integer-only.
pub(crate) fn cost_nanos(price: ModelPrice, prompt_tokens: i64, completion_tokens: i64) -> i64 {
    let component = |tokens: i64, price_minor: i64| {
        i128::from(tokens.max(0))
            .saturating_mul(i128::from(price_minor.max(0)))
            .saturating_mul(i128::from(NANOS_PER_MINOR))
            / i128::from(TOKENS_PER_PRICE_UNIT)
    };
    component(prompt_tokens, price.input_per_million_minor)
        .saturating_add(component(completion_tokens, price.output_per_million_minor))
        .min(i128::from(i64::MAX)) as i64
}

async fn resolve_workspace_price(
    store: &dyn LlmPricingStore,
    workspace_id: &str,
    model: &str,
) -> Option<ModelPrice> {
    let normalized = normalize_model(model);
    match store.get_price(workspace_id, &normalized).await {
        Ok(Some(price)) => return Some(price),
        Ok(None) => {}
        Err(error) => {
            tracing::warn!(workspace_id, model, error = %error, "workspace price lookup failed; falling back to defaults");
            return None;
        }
    }
    // Exact miss: suffix-match over the workspace's rows so deployment
    // prefixes price against workspace overrides too.
    match store.list_prices(workspace_id).await {
        Ok(rows) => resolve_suffix(
            &normalized,
            rows.iter().map(|row| (row.model.as_str(), &row.price)),
        ),
        Err(error) => {
            tracing::warn!(workspace_id, model, error = %error, "workspace price list failed; falling back to defaults");
            None
        }
    }
}

/// `(tokens × price_per_million) / 1M` per side, integer math rounding
/// down, saturating.
fn price_tokens(price: ModelPrice, prompt_tokens: i64, completion_tokens: i64) -> i64 {
    cost_nanos(price, prompt_tokens, completion_tokens) / NANOS_PER_MINOR
}

/// Longest-suffix match over `(model_key, price)` entries: a key
/// matches when `normalized` ends with it right after a deployment
/// separator (`/` or `:`).
fn resolve_suffix<'a>(
    normalized: &str,
    entries: impl Iterator<Item = (&'a str, &'a ModelPrice)>,
) -> Option<ModelPrice> {
    entries
        .filter(|(key, _)| {
            normalized
                .strip_suffix(key)
                .and_then(|prefix| prefix.chars().last())
                .is_some_and(|boundary| MODEL_PREFIX_SEPARATORS.contains(&boundary))
        })
        .max_by_key(|(key, _)| key.len())
        .map(|(_, price)| *price)
}

pub(crate) fn normalize_model(model: &str) -> String {
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
    fn nano_pricing_preserves_sub_cent_calls() {
        let price = ModelPrice {
            input_per_million_minor: 5,
            output_per_million_minor: 45,
        };
        assert_eq!(cost_nanos(price, 1, 1), 500);
        assert_eq!(cost_nanos(price, 1, 1) / NANOS_PER_MINOR, 0);
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
        // Nanos are the authoritative precision and clamp at i64::MAX;
        // the public minor-unit projection must remain non-negative.
        let cost = table.cost_minor("gpt-4o", i64::MAX, i64::MAX);
        assert_eq!(cost, Some(i64::MAX / NANOS_PER_MINOR));
        assert_eq!(
            cost_nanos(
                ModelPrice {
                    input_per_million_minor: 250,
                    output_per_million_minor: 1_000,
                },
                i64::MAX,
                i64::MAX,
            ),
            i64::MAX
        );
    }

    #[test]
    fn negative_tokens_clamp_to_zero() {
        let table = LlmPricingTable::default();
        assert_eq!(table.cost_minor("gpt-4o", -5, -5), Some(0));
    }

    #[tokio::test]
    async fn workspace_price_overrides_built_in() {
        let store = MemoryLlmPricingStore::new();
        store.upsert_price("ws", "gpt-4o", 500, 2000).await.unwrap();
        // Workspace row wins over the built-in 250/1000.
        assert_eq!(
            cost_minor(&store, "ws", "gpt-4o", 1_000_000, 1_000_000).await,
            Some(2500)
        );
        // Another workspace still sees the built-in default.
        assert_eq!(
            cost_minor(&store, "ws_other", "gpt-4o", 1_000_000, 1_000_000).await,
            Some(1250)
        );
    }

    #[tokio::test]
    async fn workspace_price_covers_model_unknown_to_builtins() {
        let store = MemoryLlmPricingStore::new();
        store
            .upsert_price("ws", "mystery-1", 100, 300)
            .await
            .unwrap();
        assert_eq!(
            cost_minor(&store, "ws", "mystery-1", 1_000_000, 1_000_000).await,
            Some(400)
        );
        // Deployment prefixes suffix-match workspace rows too.
        assert_eq!(
            cost_minor(&store, "ws", "prod/Mystery-1", 1_000_000, 1_000_000).await,
            Some(400)
        );
    }

    #[tokio::test]
    async fn delete_restores_built_in_fallback() {
        let store = MemoryLlmPricingStore::new();
        store.upsert_price("ws", "gpt-4o", 500, 2000).await.unwrap();
        assert!(store.delete_price("ws", "gpt-4o").await.unwrap());
        assert_eq!(
            cost_minor(&store, "ws", "gpt-4o", 1_000_000, 1_000_000).await,
            Some(1250)
        );
        // Deleting a row that never existed reports false.
        assert!(!store.delete_price("ws", "gpt-4o").await.unwrap());
    }

    #[tokio::test]
    async fn unknown_model_everywhere_is_none() {
        let store = MemoryLlmPricingStore::new();
        assert_eq!(
            cost_minor(&store, "ws", "mystery-1", 1000, 1000).await,
            None
        );
    }
}
