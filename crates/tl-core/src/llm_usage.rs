//! LLM gateway usage metering wire types.
//!
//! One `LlmUsageEvent` is recorded per metered gateway chat completion.
//! `GET /v1/llm-usage` serves the raw list or grouped rollups; the
//! budget-alerts and usage-dashboard features read the same shapes.

use serde::{Deserialize, Serialize};

#[cfg(feature = "schema")]
use schemars::JsonSchema;
#[cfg(feature = "ts-export")]
use ts_rs::TS;
#[cfg(feature = "openapi")]
use utoipa::ToSchema;

/// Runtime that generated a priced LLM call. Customer inference is
/// the only kind counted by customer spending caps; guardrail calls are
/// TrustLoopGuard's semantic-policy overhead.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[cfg_attr(feature = "schema", derive(JsonSchema))]
#[cfg_attr(feature = "openapi", derive(ToSchema))]
#[cfg_attr(feature = "ts-export", derive(TS))]
#[cfg_attr(feature = "ts-export", ts(export))]
pub enum LlmUsageKind {
    #[default]
    CustomerInference,
    Guardrail,
}

/// One metered LLM gateway call.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(JsonSchema))]
#[cfg_attr(feature = "openapi", derive(ToSchema))]
#[cfg_attr(feature = "ts-export", derive(TS))]
#[cfg_attr(feature = "ts-export", ts(export))]
pub struct LlmUsageEvent {
    pub id: String,
    pub workspace_id: String,
    /// Principal the spend is attributed to. Keys without a bound
    /// principal fall back to the API key id.
    pub principal_id: String,
    pub api_key_id: String,
    #[serde(default)]
    pub kind: LlmUsageKind,
    /// Raw model string from the provider response (deployment prefixes
    /// and all); pricing normalization never rewrites it.
    pub model: String,
    pub prompt_tokens: i64,
    pub completion_tokens: i64,
    /// Priced cost in currency minor units. `0` when the model has no
    /// price table entry.
    pub cost_minor: i64,
    /// Exact USD nanos, serialized as a decimal string so JavaScript
    /// consumers do not lose integer precision.
    pub cost_usd_nanos: String,
    pub currency: String,
    /// Gateway request id — unique per workspace, makes retried
    /// metering writes idempotent.
    pub request_id: String,
    #[cfg_attr(feature = "ts-export", ts(type = "Record<string, unknown> | null"))]
    pub metadata: serde_json::Value,
    /// RFC 3339 timestamp.
    pub effective_at: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(JsonSchema))]
#[cfg_attr(feature = "openapi", derive(ToSchema))]
#[cfg_attr(feature = "ts-export", derive(TS))]
#[cfg_attr(feature = "ts-export", ts(export))]
pub struct LlmUsageListResponse {
    pub events: Vec<LlmUsageEvent>,
}

/// One `group_by` rollup row. `key` is always a string: an RFC 3339
/// date (`YYYY-MM-DD`) for day buckets, otherwise the principal or
/// model value.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(JsonSchema))]
#[cfg_attr(feature = "openapi", derive(ToSchema))]
#[cfg_attr(feature = "ts-export", derive(TS))]
#[cfg_attr(feature = "ts-export", ts(export))]
pub struct LlmUsageBucket {
    pub key: String,
    pub prompt_tokens: i64,
    pub completion_tokens: i64,
    pub cost_minor: i64,
    /// Exact accumulated USD nanos as a decimal string.
    pub cost_usd_nanos: String,
    pub calls: i64,
    /// `true` when this model bucket includes token-bearing calls that
    /// were recorded with zero cost, so `cost_minor` undercounts the
    /// selected window. Only set on `group_by=model` buckets; omitted
    /// when every call in the bucket had a nonzero price.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts-export", ts(optional))]
    pub unpriced: Option<bool>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(JsonSchema))]
#[cfg_attr(feature = "openapi", derive(ToSchema))]
#[cfg_attr(feature = "ts-export", derive(TS))]
#[cfg_attr(feature = "ts-export", ts(export))]
pub struct LlmUsageBucketsResponse {
    pub buckets: Vec<LlmUsageBucket>,
}

/// `GET /v1/llm-usage` 200 body: the raw event list, or grouped buckets
/// when `group_by` is set. Untagged — the two shapes are distinguished
/// by their sole field (`events` vs `buckets`).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
#[cfg_attr(feature = "schema", derive(JsonSchema))]
#[cfg_attr(feature = "openapi", derive(ToSchema))]
#[cfg_attr(feature = "ts-export", derive(TS))]
#[cfg_attr(feature = "ts-export", ts(export))]
pub enum LlmUsageResponse {
    List(LlmUsageListResponse),
    Buckets(LlmUsageBucketsResponse),
}
