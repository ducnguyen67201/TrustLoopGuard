//! Wire types for the red-team dispatch orchestrator (`/v1/redteam/*`).
//!
//! A *dispatch* creates a durable *job* that the server runs in the background by
//! driving a compatible private runner. The server owns the job + per-attack
//! results; the runner owns nothing durable.

use std::collections::HashMap;

use serde::{Deserialize, Serialize};

use crate::PolicyDocument;

#[cfg(feature = "schema")]
use schemars::JsonSchema;
#[cfg(feature = "ts-export")]
use ts_rs::TS;
#[cfg(feature = "openapi")]
use utoipa::ToSchema;

/// Lifecycle of a dispatched red-team job.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[cfg_attr(feature = "schema", derive(JsonSchema))]
#[cfg_attr(feature = "openapi", derive(ToSchema))]
#[cfg_attr(feature = "ts-export", derive(TS))]
#[cfg_attr(feature = "ts-export", ts(export))]
pub enum JobStatus {
    Queued,
    Running,
    Complete,
    Error,
    Cancelled,
}

/// Execution mode for a red-team run.
///
/// TrustLoopGuard only routes this mode to the private runner. Learning memory,
/// retrieval, compaction, and adaptive attack planning are orchestration-owned
/// business logic.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[cfg_attr(feature = "schema", derive(JsonSchema))]
#[cfg_attr(feature = "openapi", derive(ToSchema))]
#[cfg_attr(feature = "ts-export", derive(TS))]
#[cfg_attr(feature = "ts-export", ts(export))]
pub enum RedteamRunMode {
    #[default]
    OneOff,
    Learning,
}

/// Target surface for a red-team run.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[cfg_attr(feature = "schema", derive(JsonSchema))]
#[cfg_attr(feature = "openapi", derive(ToSchema))]
#[cfg_attr(feature = "ts-export", derive(TS))]
#[cfg_attr(feature = "ts-export", ts(export))]
pub enum RedteamAttackSurface {
    #[default]
    Chat,
    DocumentWorkflow,
}

/// Optional PDF form template for document workflow attacks.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(JsonSchema))]
#[cfg_attr(feature = "openapi", derive(ToSchema))]
#[cfg_attr(feature = "ts-export", derive(TS))]
#[cfg_attr(feature = "ts-export", ts(export))]
pub struct RedteamDocumentTemplate {
    pub file_name: String,
    pub media_type: String,
    pub data_base64: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts-export", ts(optional))]
    pub fields: Option<HashMap<String, String>>,
    #[serde(default)]
    pub flatten: bool,
}

/// Body of `POST /v1/redteam/dispatch`.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(JsonSchema))]
#[cfg_attr(feature = "openapi", derive(ToSchema))]
#[cfg_attr(feature = "ts-export", derive(TS))]
#[cfg_attr(feature = "ts-export", ts(export))]
pub struct RedteamDispatchRequest {
    /// Loopback agent endpoint to attack (arena adapter contract).
    pub target_url: String,
    /// `fast` | `full` | `max`.
    pub profile: String,
    /// `one_off` runs a stateless campaign. `learning` lets the private runner
    /// use its orchestration-owned learning memory.
    #[serde(default)]
    pub mode: RedteamRunMode,
    /// Target surface to attack. `chat` is the default for backward compatibility.
    #[serde(default)]
    pub attack_surface: RedteamAttackSurface,
    /// Optional registered agent this job is associated with (for history).
    #[serde(default)]
    #[cfg_attr(feature = "ts-export", ts(optional))]
    pub agent_id: Option<String>,
    /// Optional uploaded PDF form template. Only valid for `document_workflow`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts-export", ts(optional))]
    pub document_template: Option<RedteamDocumentTemplate>,
}

/// Summary row for a dispatched job.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(JsonSchema))]
#[cfg_attr(feature = "openapi", derive(ToSchema))]
#[cfg_attr(feature = "ts-export", derive(TS))]
#[cfg_attr(feature = "ts-export", ts(export))]
pub struct RedteamJobSummary {
    pub id: String,
    pub workspace_id: String,
    pub environment_id: String,
    pub status: JobStatus,
    pub target: String,
    pub profile: String,
    // Serialized as `null` when absent (serde sends `None` as null), so the wire
    // type is `string | null`, not an omitted key. No `ts(optional)`.
    #[serde(default)]
    pub agent_id: Option<String>,
    /// Non-control attacks attempted.
    #[cfg_attr(feature = "ts-export", ts(type = "number"))]
    pub attacks: i64,
    /// Attacks that got through.
    #[cfg_attr(feature = "ts-export", ts(type = "number"))]
    pub landed: i64,
    /// Attacks the guard blocked.
    #[cfg_attr(feature = "ts-export", ts(type = "number"))]
    pub blocked: i64,
    #[serde(default)]
    pub error: Option<String>,
    /// RFC 3339 timestamp.
    pub created_at: String,
    /// RFC 3339 timestamp.
    pub updated_at: String,
}

/// One scored attack within a job.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(JsonSchema))]
#[cfg_attr(feature = "openapi", derive(ToSchema))]
#[cfg_attr(feature = "ts-export", derive(TS))]
#[cfg_attr(feature = "ts-export", ts(export))]
pub struct RedteamJobResult {
    pub seq: i32,
    /// Stable case identity for raw-vs-guarded benchmark comparison.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts-export", ts(optional))]
    pub case_id: Option<String>,
    /// Benchmark/security track, e.g. `private_data_flow`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts-export", ts(optional))]
    pub track: Option<String>,
    /// Case kind, e.g. `attack`, `benign`, or `attack_under_task`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts-export", ts(optional))]
    pub kind: Option<String>,
    /// Trial index for live repeated runs.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts-export", ts(optional))]
    pub trial_index: Option<i32>,
    pub attack: String,
    pub goal: String,
    /// `landed` | `blocked` | `clean` | `error`.
    pub outcome: String,
    pub landed: bool,
    #[serde(default)]
    pub prompt: Option<String>,
    pub reply: String,
    #[serde(default)]
    pub trace_id: Option<String>,
}

/// Response from `GET /v1/redteam/jobs/{id}`.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(JsonSchema))]
#[cfg_attr(feature = "openapi", derive(ToSchema))]
#[cfg_attr(feature = "ts-export", derive(TS))]
#[cfg_attr(feature = "ts-export", ts(export))]
pub struct RedteamJobDetail {
    pub job: RedteamJobSummary,
    pub results: Vec<RedteamJobResult>,
}

/// Response from `GET /v1/redteam/jobs`.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(JsonSchema))]
#[cfg_attr(feature = "openapi", derive(ToSchema))]
#[cfg_attr(feature = "ts-export", derive(TS))]
#[cfg_attr(feature = "ts-export", ts(export))]
pub struct RedteamJobListResponse {
    pub jobs: Vec<RedteamJobSummary>,
}

/// Response from `GET /v1/redteam/jobs/{id}/results`.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(JsonSchema))]
#[cfg_attr(feature = "openapi", derive(ToSchema))]
#[cfg_attr(feature = "ts-export", derive(TS))]
#[cfg_attr(feature = "ts-export", ts(export))]
pub struct RedteamJobResultListResponse {
    pub results: Vec<RedteamJobResult>,
}

// ---------------------------------------------------------------------------
// Vulnerability report (`GET /v1/redteam/jobs/{id}/report`,
// `GET /v1/redteam/reports/{token}`).
//
// A *report* is a presentation-ready view of one completed job (optionally
// compared against a second run of the same agent). The server classifies each
// landed attack into a severity finding and rolls up an overall risk level;
// the web layer renders the PDF. Severity classification is product semantics,
// so it lives here (Rust), not in the renderer.
// ---------------------------------------------------------------------------

/// Severity of a single report finding and the report's overall risk level.
/// Ordered most-severe-first for human reading; the builder compares via an
/// explicit rank, not declaration order.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[cfg_attr(feature = "schema", derive(JsonSchema))]
#[cfg_attr(feature = "openapi", derive(ToSchema))]
#[cfg_attr(feature = "ts-export", derive(TS))]
#[cfg_attr(feature = "ts-export", ts(export))]
pub enum ReportSeverity {
    Critical,
    High,
    Medium,
    Low,
    Info,
}

/// How a single attack changed between the baseline and the compared run.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[cfg_attr(feature = "schema", derive(JsonSchema))]
#[cfg_attr(feature = "openapi", derive(ToSchema))]
#[cfg_attr(feature = "ts-export", derive(TS))]
#[cfg_attr(feature = "ts-export", ts(export))]
pub enum ComparedAttackStatus {
    /// Landed on the baseline, blocked on the compared run.
    Fixed,
    /// Landed on both runs.
    StillVulnerable,
    /// Blocked on the baseline, landed on the compared run.
    Regressed,
    /// Same outcome on both runs (neither a fix nor a regression).
    Unchanged,
}

/// One classified vulnerability in a single-run report.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(JsonSchema))]
#[cfg_attr(feature = "openapi", derive(ToSchema))]
#[cfg_attr(feature = "ts-export", derive(TS))]
#[cfg_attr(feature = "ts-export", ts(export))]
pub struct RedteamReportFinding {
    pub seq: i32,
    pub attack: String,
    pub goal: String,
    /// Human-facing attack category (e.g. `credential_disclosure`).
    pub category: String,
    pub severity: ReportSeverity,
    /// `landed` | `blocked` | `clean` | `error`.
    pub outcome: String,
    pub landed: bool,
    /// Short excerpt of the agent reply showing what the attack achieved.
    /// Truncated; redaction-aware (see the server-side builder).
    #[serde(default)]
    pub evidence: Option<String>,
    #[serde(default)]
    pub prompt: Option<String>,
    #[serde(default)]
    pub trace_id: Option<String>,
}

/// Rolled-up counts and derived risk for one run.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(JsonSchema))]
#[cfg_attr(feature = "openapi", derive(ToSchema))]
#[cfg_attr(feature = "ts-export", derive(TS))]
#[cfg_attr(feature = "ts-export", ts(export))]
pub struct RedteamReportAggregates {
    /// Every recorded result, including clean control cases.
    #[cfg_attr(feature = "ts-export", ts(type = "number"))]
    pub total: i64,
    /// Non-control attacks (the success-rate denominator).
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
    /// `landed / attacks` in `[0, 1]` (0 when there are no non-control attacks).
    pub success_rate: f64,
    /// Worst severity among landed findings; `Info` when nothing landed.
    pub risk_level: ReportSeverity,
}

/// One attack lined up across the baseline and compared runs.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(JsonSchema))]
#[cfg_attr(feature = "openapi", derive(ToSchema))]
#[cfg_attr(feature = "ts-export", derive(TS))]
#[cfg_attr(feature = "ts-export", ts(export))]
pub struct RedteamComparedAttack {
    pub attack: String,
    pub goal: String,
    /// `landed` | `blocked` | `clean` | `error` on the baseline run.
    pub baseline_outcome: String,
    /// Same on the compared run; `null` when the attack is absent there.
    #[serde(default)]
    pub compare_outcome: Option<String>,
    pub status: ComparedAttackStatus,
}

/// Before/after view comparing two runs of the same agent.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(JsonSchema))]
#[cfg_attr(feature = "openapi", derive(ToSchema))]
#[cfg_attr(feature = "ts-export", derive(TS))]
#[cfg_attr(feature = "ts-export", ts(export))]
pub struct RedteamReportComparison {
    pub baseline: RedteamJobSummary,
    pub compare: RedteamJobSummary,
    pub baseline_aggregates: RedteamReportAggregates,
    pub compare_aggregates: RedteamReportAggregates,
    /// Baseline minus compared success rate, in percentage points.
    pub delta_points: f64,
    pub attacks: Vec<RedteamComparedAttack>,
}

/// Response from the report endpoints: the presentation-ready vulnerability
/// report for one job (optionally a same-agent comparison).
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(JsonSchema))]
#[cfg_attr(feature = "openapi", derive(ToSchema))]
#[cfg_attr(feature = "ts-export", derive(TS))]
#[cfg_attr(feature = "ts-export", ts(export))]
pub struct RedteamReportPayload {
    pub job: RedteamJobSummary,
    pub aggregates: RedteamReportAggregates,
    pub findings: Vec<RedteamReportFinding>,
    #[serde(default)]
    pub comparison: Option<RedteamReportComparison>,
    /// RFC 3339 timestamp of when this report view was generated.
    pub generated_at: String,
}

/// Body of `POST /v1/redteam/reports` — mint a shareable report link.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(JsonSchema))]
#[cfg_attr(feature = "openapi", derive(ToSchema))]
#[cfg_attr(feature = "ts-export", derive(TS))]
#[cfg_attr(feature = "ts-export", ts(export))]
pub struct CreateReportRequest {
    /// The completed job to report on.
    pub job_id: String,
    /// Optional second run of the same agent to compare against.
    #[serde(default)]
    #[cfg_attr(feature = "ts-export", ts(optional))]
    pub compare_job_id: Option<String>,
    /// Days until the link expires. Clamped server-side; defaults to 30.
    #[serde(default)]
    #[cfg_attr(feature = "ts-export", ts(optional))]
    pub ttl_days: Option<u32>,
}

/// Response from `POST /v1/redteam/reports` — a minted share token.
///
/// The dashboard composes the absolute, shareable URL from its own origin and
/// `path`; the server intentionally does not assume its public hostname.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(JsonSchema))]
#[cfg_attr(feature = "openapi", derive(ToSchema))]
#[cfg_attr(feature = "ts-export", derive(TS))]
#[cfg_attr(feature = "ts-export", ts(export))]
pub struct RedteamReportShare {
    /// Opaque bearer token; also the last path segment.
    pub token: String,
    /// Relative path that renders the report (e.g. `/r/{token}`).
    pub path: String,
    pub job_id: String,
    #[serde(default)]
    pub compare_job_id: Option<String>,
    /// RFC 3339 timestamp.
    pub created_at: String,
    /// RFC 3339 timestamp; `null` when the link never expires.
    #[serde(default)]
    pub expires_at: Option<String>,
}

/// One attack result flattened with its parent job's context. Powers the
/// workspace-wide records browser (`GET /v1/redteam/attacks`), which lists
/// attacks across all jobs rather than within a single job.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(JsonSchema))]
#[cfg_attr(feature = "openapi", derive(ToSchema))]
#[cfg_attr(feature = "ts-export", derive(TS))]
#[cfg_attr(feature = "ts-export", ts(export))]
pub struct RedteamAttackRecord {
    /// The parent job this attack belongs to.
    pub job_id: String,
    pub target: String,
    pub profile: String,
    /// RFC 3339 timestamp of the parent job's creation.
    pub created_at: String,
    pub seq: i32,
    pub attack: String,
    pub goal: String,
    /// `landed` | `blocked` | `clean` | `error`.
    pub outcome: String,
    pub landed: bool,
    #[serde(default)]
    pub prompt: Option<String>,
    pub reply: String,
    #[serde(default)]
    pub trace_id: Option<String>,
}

/// Response from `GET /v1/redteam/attacks`.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(JsonSchema))]
#[cfg_attr(feature = "openapi", derive(ToSchema))]
#[cfg_attr(feature = "ts-export", derive(TS))]
#[cfg_attr(feature = "ts-export", ts(export))]
pub struct RedteamAttackRecordListResponse {
    pub records: Vec<RedteamAttackRecord>,
}

// ---------------------------------------------------------------------------
// Harden (`POST /v1/redteam/jobs/{id}/harden`).
//
// Hardening synthesizes guardrail policies from the attacks that landed in a
// completed job, *verifies* each candidate against the landed cases, generated
// obfuscated variants, and benign controls, and recommends only the survivors.
// Recommendations persist `enabled = false` — an operator opts in via
// `PATCH /v1/policies/{id}/enabled`, exactly like `guardrails:generate`.
// ---------------------------------------------------------------------------

/// Body of `POST /v1/redteam/jobs/{id}/harden`.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(JsonSchema))]
#[cfg_attr(feature = "openapi", derive(ToSchema))]
#[cfg_attr(feature = "ts-export", derive(TS))]
#[cfg_attr(feature = "ts-export", ts(export))]
pub struct HardenRequest {
    /// `false` (default) previews candidates without persisting; `true` upserts
    /// the survivors `enabled = false`.
    #[serde(default)]
    pub persist: bool,
}

/// Outcome of re-running a candidate policy through the engine before
/// recommending it. A candidate `passed` only when it blocks every landed
/// case, blocks enough obfuscated variants, and false-blocks no controls.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(JsonSchema))]
#[cfg_attr(feature = "openapi", derive(ToSchema))]
#[cfg_attr(feature = "ts-export", derive(TS))]
#[cfg_attr(feature = "ts-export", ts(export))]
pub struct VerifyResult {
    /// Landed cases the candidate now blocks, over the landed cases tested.
    #[cfg_attr(feature = "ts-export", ts(type = "number"))]
    pub blocked_landed: u32,
    #[cfg_attr(feature = "ts-export", ts(type = "number"))]
    pub landed_total: u32,
    /// Obfuscated/paraphrased variants the candidate blocks, over those tested.
    #[cfg_attr(feature = "ts-export", ts(type = "number"))]
    pub blocked_variants: u32,
    #[cfg_attr(feature = "ts-export", ts(type = "number"))]
    pub variant_total: u32,
    /// Benign control cases the candidate wrongly blocks, over those tested.
    #[cfg_attr(feature = "ts-export", ts(type = "number"))]
    pub false_blocks: u32,
    #[cfg_attr(feature = "ts-export", ts(type = "number"))]
    pub control_total: u32,
    pub passed: bool,
}

/// One recommended guardrail synthesized + verified from a job's landed attacks.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(JsonSchema))]
#[cfg_attr(feature = "openapi", derive(ToSchema))]
#[cfg_attr(feature = "ts-export", derive(TS))]
#[cfg_attr(feature = "ts-export", ts(export))]
pub struct HardenCandidate {
    /// The synthesized policy (persisted `enabled = false` when `persist`).
    pub policy: PolicyDocument,
    /// Enforcement substrate, e.g. `semantic_output` | `regex_output` |
    /// `approval` | `param_source`.
    pub substrate: String,
    /// `seq` of the landed cases this candidate was derived from.
    pub evidence_seqs: Vec<i32>,
    /// Where the match logic came from: `llm` | `deterministic`.
    pub source: String,
    pub verify: VerifyResult,
}

/// Response from `POST /v1/redteam/jobs/{id}/harden`.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(JsonSchema))]
#[cfg_attr(feature = "openapi", derive(ToSchema))]
#[cfg_attr(feature = "ts-export", derive(TS))]
#[cfg_attr(feature = "ts-export", ts(export))]
pub struct HardenResponse {
    pub candidates: Vec<HardenCandidate>,
    /// Substrates a landed attack needed but that this job's traces could not
    /// reach (e.g. an action attack with only output-level traces). Surfaced
    /// so coverage gaps are explicit rather than silently approximated.
    pub unreachable: Vec<String>,
    /// RFC 3339 timestamp of when these candidates were generated.
    pub generated_at: String,
}
