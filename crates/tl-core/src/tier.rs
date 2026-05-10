//! Tier result types — one entry per pipeline stage in the parallel-cancel
//! orchestrator. Returned alongside the final `Decision` so callers can see
//! which tier produced which reason and how long it took.
//!
//! See `docs/concept/v0-design-decisions.md` §4 for the orchestration model.

use crate::TriggeredPolicy;
use serde::{Deserialize, Serialize};

#[cfg(feature = "schema")]
use schemars::JsonSchema;
#[cfg(feature = "ts-export")]
use ts_rs::TS;
#[cfg(feature = "openapi")]
use utoipa::ToSchema;

/// Which pipeline tier produced a result.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
#[cfg_attr(feature = "schema", derive(JsonSchema))]
#[cfg_attr(feature = "openapi", derive(ToSchema))]
#[cfg_attr(feature = "ts-export", derive(TS))]
#[cfg_attr(feature = "ts-export", ts(export))]
pub enum Tier {
    /// Static matchers — regex, literal, PII patterns. Microsecond-scale.
    Deterministic,
    /// Embedding similarity + edit-distance bypass detection. ~5-20 ms.
    Fuzzy,
    /// LLM judges (hallucination, tone, authority). ~200-600 ms.
    Llm,
}

/// How a tier finished.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[cfg_attr(feature = "schema", derive(JsonSchema))]
#[cfg_attr(feature = "openapi", derive(ToSchema))]
#[cfg_attr(feature = "ts-export", derive(TS))]
#[cfg_attr(feature = "ts-export", ts(export))]
pub enum TierStatus {
    /// Tier finished and contributed to the aggregated decision.
    Completed,
    /// Tier was cancelled because an earlier tier produced a hard block.
    Cancelled,
    /// Tier exceeded its deadline and was treated as `Escalate`.
    TimedOut,
    /// Tier was not run (e.g. cache hit, or feature disabled by config).
    Skipped,
}

/// One row of evidence per tier in the final `Decision`.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(JsonSchema))]
#[cfg_attr(feature = "openapi", derive(ToSchema))]
#[cfg_attr(feature = "ts-export", derive(TS))]
#[cfg_attr(feature = "ts-export", ts(export))]
pub struct TierResult {
    pub tier: Tier,
    pub status: TierStatus,
    #[serde(default)]
    pub reasons: Vec<TriggeredPolicy>,
    pub elapsed_ms: u64,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Severity;

    #[test]
    fn serializes_with_lowercase_tags() {
        let r = TierResult {
            tier: Tier::Deterministic,
            status: TierStatus::Completed,
            reasons: vec![TriggeredPolicy {
                id: "p1".into(),
                severity: Severity::High,
                reason: "matched".into(),
            }],
            elapsed_ms: 1,
        };
        let json = serde_json::to_string(&r).unwrap();
        assert!(json.contains(r#""tier":"deterministic""#));
        assert!(json.contains(r#""status":"completed""#));
    }
}
