//! Workspace-editable LLM model pricing wire types.
//!
//! `GET /v1/llm-pricing` returns the *effective* price table — workspace
//! rows merged over the built-in defaults; `PUT`/`DELETE
//! /v1/llm-pricing/{model}` manage the workspace rows. Prices are
//! exact USD nanos per 1M tokens, input and output separately. Legacy
//! minor-unit projections remain additive compatibility fields.

use serde::{Deserialize, Serialize};

#[cfg(feature = "schema")]
use schemars::JsonSchema;
#[cfg(feature = "ts-export")]
use ts_rs::TS;
#[cfg(feature = "openapi")]
use utoipa::ToSchema;

/// Where an effective model price comes from: a workspace-edited row or
/// the built-in default table.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[cfg_attr(feature = "schema", derive(JsonSchema))]
#[cfg_attr(feature = "openapi", derive(ToSchema))]
#[cfg_attr(feature = "ts-export", derive(TS))]
#[cfg_attr(feature = "ts-export", ts(export))]
pub enum LlmPriceSource {
    Workspace,
    Default,
}

/// One effective model price row.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(JsonSchema))]
#[cfg_attr(feature = "openapi", derive(ToSchema))]
#[cfg_attr(feature = "ts-export", derive(TS))]
#[cfg_attr(feature = "ts-export", ts(export))]
pub struct LlmModelPrice {
    /// Normalized model key (trimmed, lowercase).
    pub model: String,
    /// USD minor units per 1M prompt tokens.
    pub input_per_million_minor: i64,
    /// USD minor units per 1M completion tokens.
    pub output_per_million_minor: i64,
    /// Exact USD nanos per 1M prompt tokens. Decimal string for safe
    /// JavaScript transport.
    pub input_per_million_usd_nanos: String,
    /// Exact USD nanos per 1M completion tokens.
    pub output_per_million_usd_nanos: String,
    pub currency: String,
    pub source: LlmPriceSource,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(JsonSchema))]
#[cfg_attr(feature = "openapi", derive(ToSchema))]
#[cfg_attr(feature = "ts-export", derive(TS))]
#[cfg_attr(feature = "ts-export", ts(export))]
pub struct LlmPricingListResponse {
    pub prices: Vec<LlmModelPrice>,
}

/// `PUT /v1/llm-pricing/{model}` body. Prices must be non-negative — a
/// negative price would subtract from accumulated spend and quietly
/// defeat the budget gate.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(JsonSchema))]
#[cfg_attr(feature = "openapi", derive(ToSchema))]
#[cfg_attr(feature = "ts-export", derive(TS))]
#[cfg_attr(feature = "ts-export", ts(export))]
pub struct UpsertLlmModelPriceRequest {
    /// USD minor units per 1M prompt tokens.
    pub input_per_million_minor: i64,
    /// USD minor units per 1M completion tokens.
    pub output_per_million_minor: i64,
    /// Optional exact USD nanos per 1M prompt tokens. When omitted the
    /// legacy minor-unit value is converted exactly.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts-export", ts(optional))]
    pub input_per_million_usd_nanos: Option<String>,
    /// Optional exact USD nanos per 1M completion tokens.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts-export", ts(optional))]
    pub output_per_million_usd_nanos: Option<String>,
}
