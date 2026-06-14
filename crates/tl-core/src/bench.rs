//! Wire types for TrustLoopGuardBench (`/v1/bench/*`).
//!
//! A benchmark run is a Rust-owned parent record that compares two child
//! red-team jobs: a raw arm and a guarded arm. Reports are derived from the
//! child job results; benchmark tables store run/arm identity, not duplicate
//! per-attack evidence.

use serde::{Deserialize, Serialize};

#[cfg(feature = "schema")]
use schemars::JsonSchema;
#[cfg(feature = "ts-export")]
use ts_rs::TS;
#[cfg(feature = "openapi")]
use utoipa::ToSchema;

use crate::{ComparedAttackStatus, RedteamGenerator, RedteamJobSummary};

/// Which side of a raw-vs-guarded benchmark pair a result belongs to.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[cfg_attr(feature = "schema", derive(JsonSchema))]
#[cfg_attr(feature = "openapi", derive(ToSchema))]
#[cfg_attr(feature = "ts-export", derive(TS))]
#[cfg_attr(feature = "ts-export", ts(export))]
pub enum BenchArm {
    Raw,
    Guarded,
}

/// Lifecycle of a benchmark parent run.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[cfg_attr(feature = "schema", derive(JsonSchema))]
#[cfg_attr(feature = "openapi", derive(ToSchema))]
#[cfg_attr(feature = "ts-export", derive(TS))]
#[cfg_attr(feature = "ts-export", ts(export))]
pub enum BenchRunStatus {
    Queued,
    Running,
    Complete,
    Error,
    Cancelled,
}

/// Body of `POST /v1/bench/runs`.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(JsonSchema))]
#[cfg_attr(feature = "openapi", derive(ToSchema))]
#[cfg_attr(feature = "ts-export", derive(TS))]
#[cfg_attr(feature = "ts-export", ts(export))]
pub struct BenchRunCreateRequest {
    /// Loopback raw/unguarded agent endpoint.
    pub raw_target_url: String,
    /// Loopback TrustLoopGuard-protected agent endpoint.
    pub guarded_target_url: String,
    /// `fast` | `full` | `max`.
    pub profile: String,
    #[serde(default)]
    #[cfg_attr(feature = "ts-export", ts(optional))]
    pub generator: Option<RedteamGenerator>,
    /// Optional registered agent this benchmark is associated with.
    #[serde(default)]
    #[cfg_attr(feature = "ts-export", ts(optional))]
    pub agent_id: Option<String>,
    /// Optional deterministic attack seed or corpus version.
    #[serde(default)]
    #[cfg_attr(feature = "ts-export", ts(optional))]
    pub seed: Option<String>,
}

/// Summary row for a benchmark parent run.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(JsonSchema))]
#[cfg_attr(feature = "openapi", derive(ToSchema))]
#[cfg_attr(feature = "ts-export", derive(TS))]
#[cfg_attr(feature = "ts-export", ts(export))]
pub struct BenchRunSummary {
    pub id: String,
    pub workspace_id: String,
    pub environment_id: String,
    pub status: BenchRunStatus,
    pub profile: String,
    pub generator: RedteamGenerator,
    #[serde(default)]
    pub agent_id: Option<String>,
    #[serde(default)]
    pub seed: Option<String>,
    #[serde(default)]
    pub error: Option<String>,
    /// RFC 3339 timestamp.
    pub created_at: String,
    /// RFC 3339 timestamp.
    pub updated_at: String,
}

/// One arm attached to a benchmark parent run.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(JsonSchema))]
#[cfg_attr(feature = "openapi", derive(ToSchema))]
#[cfg_attr(feature = "ts-export", derive(TS))]
#[cfg_attr(feature = "ts-export", ts(export))]
pub struct BenchRunArmSummary {
    pub run_id: String,
    pub arm: BenchArm,
    pub label: String,
    pub target: String,
    #[serde(default)]
    pub redteam_job_id: Option<String>,
    #[serde(default)]
    pub checker_config: Option<String>,
    /// RFC 3339 timestamp.
    pub created_at: String,
    /// RFC 3339 timestamp.
    pub updated_at: String,
}

/// Response from `GET /v1/bench/runs/{id}`.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(JsonSchema))]
#[cfg_attr(feature = "openapi", derive(ToSchema))]
#[cfg_attr(feature = "ts-export", derive(TS))]
#[cfg_attr(feature = "ts-export", ts(export))]
pub struct BenchRunDetail {
    pub run: BenchRunSummary,
    pub arms: Vec<BenchRunArmSummary>,
    #[serde(default)]
    pub raw_job: Option<RedteamJobSummary>,
    #[serde(default)]
    pub guarded_job: Option<RedteamJobSummary>,
}

/// Response from `GET /v1/bench/runs`.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(JsonSchema))]
#[cfg_attr(feature = "openapi", derive(ToSchema))]
#[cfg_attr(feature = "ts-export", derive(TS))]
#[cfg_attr(feature = "ts-export", ts(export))]
pub struct BenchRunListResponse {
    pub runs: Vec<BenchRunSummary>,
}

/// Aggregate metrics for one benchmark arm.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(JsonSchema))]
#[cfg_attr(feature = "openapi", derive(ToSchema))]
#[cfg_attr(feature = "ts-export", derive(TS))]
#[cfg_attr(feature = "ts-export", ts(export))]
pub struct BenchArmMetrics {
    pub arm: BenchArm,
    #[cfg_attr(feature = "ts-export", ts(type = "number"))]
    pub attacks: i64,
    #[cfg_attr(feature = "ts-export", ts(type = "number"))]
    pub landed: i64,
    #[cfg_attr(feature = "ts-export", ts(type = "number"))]
    pub blocked: i64,
    #[cfg_attr(feature = "ts-export", ts(type = "number"))]
    pub clean: i64,
    #[cfg_attr(feature = "ts-export", ts(type = "number"))]
    pub errored: i64,
    pub attack_success_rate: f64,
    pub benign_utility_rate: f64,
    pub utility_under_attack_rate: f64,
    pub false_block_rate: f64,
}

/// Per-track comparison for reports. `track` is a string because the live
/// runner may emit held-out track names before `tl-core` grows a stricter enum.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(JsonSchema))]
#[cfg_attr(feature = "openapi", derive(ToSchema))]
#[cfg_attr(feature = "ts-export", derive(TS))]
#[cfg_attr(feature = "ts-export", ts(export))]
pub struct BenchTrackMetrics {
    pub track: String,
    pub raw: BenchArmMetrics,
    pub guarded: BenchArmMetrics,
}

/// Headline guarded-vs-raw deltas.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(JsonSchema))]
#[cfg_attr(feature = "openapi", derive(ToSchema))]
#[cfg_attr(feature = "ts-export", derive(TS))]
#[cfg_attr(feature = "ts-export", ts(export))]
pub struct BenchReportDelta {
    pub attack_success_rate_reduction: f64,
    pub benign_utility_delta: f64,
    pub utility_under_attack_delta: f64,
    pub false_block_delta: f64,
}

/// One attack/control case lined up across the raw and guarded arms.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(JsonSchema))]
#[cfg_attr(feature = "openapi", derive(ToSchema))]
#[cfg_attr(feature = "ts-export", derive(TS))]
#[cfg_attr(feature = "ts-export", ts(export))]
pub struct BenchComparedCase {
    #[serde(default)]
    pub case_id: Option<String>,
    pub attack: String,
    pub goal: String,
    #[serde(default)]
    pub track: Option<String>,
    #[serde(default)]
    pub kind: Option<String>,
    pub raw_outcome: String,
    pub guarded_outcome: String,
    pub status: ComparedAttackStatus,
}

/// Derived report for a completed benchmark run.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(JsonSchema))]
#[cfg_attr(feature = "openapi", derive(ToSchema))]
#[cfg_attr(feature = "ts-export", derive(TS))]
#[cfg_attr(feature = "ts-export", ts(export))]
pub struct BenchReportPayload {
    pub run: BenchRunSummary,
    pub arms: Vec<BenchRunArmSummary>,
    pub raw: BenchArmMetrics,
    pub guarded: BenchArmMetrics,
    pub delta: BenchReportDelta,
    pub tracks: Vec<BenchTrackMetrics>,
    pub cases: Vec<BenchComparedCase>,
    pub generated_at: String,
}
