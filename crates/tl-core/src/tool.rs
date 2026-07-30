use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
#[cfg_attr(feature = "ts-export", derive(ts_rs::TS))]
#[cfg_attr(feature = "ts-export", ts(export))]
pub struct ToolIdentity {
    pub server_id: String,
    pub tool_name: String,
    pub schema_hash: String,
}

use crate::{Origin, SideEffectClass};

#[cfg(feature = "schema")]
use schemars::JsonSchema;
#[cfg(feature = "ts-export")]
use ts_rs::TS;
#[cfg(feature = "openapi")]
use utoipa::ToSchema;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(JsonSchema))]
#[cfg_attr(feature = "openapi", derive(ToSchema))]
#[cfg_attr(feature = "ts-export", derive(TS))]
#[cfg_attr(feature = "ts-export", ts(export))]
pub struct ToolMetadata {
    pub tool: String,
    pub side_effect: SideEffectClass,
    pub reversible: bool,
    #[serde(default)]
    pub params: Vec<ParamSpec>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts-export", ts(optional))]
    pub approval: Option<ApprovalRule>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts-export", ts(optional))]
    #[cfg_attr(feature = "ts-export", ts(type = "Record<string, unknown> | null"))]
    pub sandbox_hint: Option<serde_json::Value>,
}

/// Outcome of resolving an event's `action.operation` against the
/// workspace tool-metadata registry. Resolved and unregistered outcomes are
/// evidence for checkers and policies; resolution failure is a fail-closed
/// pipeline condition that defers execution.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "status", rename_all = "snake_case")]
#[cfg_attr(feature = "schema", derive(JsonSchema))]
#[cfg_attr(feature = "openapi", derive(ToSchema))]
#[cfg_attr(feature = "ts-export", derive(TS))]
#[cfg_attr(feature = "ts-export", ts(export))]
pub enum ToolResolution {
    /// The tool is registered and enabled; the registry's side-effect
    /// class is authoritative for the event.
    Resolved { metadata: ToolMetadata },
    /// No enabled registry entry exists for the tool — the conservative
    /// default for unknown or disabled tools.
    Unregistered,
    /// The registry could not be consulted (e.g. storage failure).
    /// Distinguishes degraded resolution from genuine absence so traces
    /// stay accurate forensic evidence. Collector-claimed action semantics
    /// are discarded and execution is deferred until metadata is available.
    ResolutionFailed,
}

/// A registry row as seen by the control plane: the wire `ToolMetadata`
/// plus its `enabled` flag. Disabled tools stay manageable but resolve
/// as unregistered at runtime.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(JsonSchema))]
#[cfg_attr(feature = "openapi", derive(ToSchema))]
#[cfg_attr(feature = "ts-export", derive(TS))]
#[cfg_attr(feature = "ts-export", ts(export))]
pub struct ToolMetadataEntry {
    pub metadata: ToolMetadata,
    pub enabled: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(JsonSchema))]
#[cfg_attr(feature = "openapi", derive(ToSchema))]
#[cfg_attr(feature = "ts-export", derive(TS))]
#[cfg_attr(feature = "ts-export", ts(export))]
pub struct ToolMetadataListResponse {
    pub tools: Vec<ToolMetadataEntry>,
}

/// Upsert body for `POST /v1/tool-metadata`: the wire `ToolMetadata`
/// plus an optional `enabled` flag (defaults to `true`).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(JsonSchema))]
#[cfg_attr(feature = "openapi", derive(ToSchema))]
#[cfg_attr(feature = "ts-export", derive(TS))]
#[cfg_attr(feature = "ts-export", ts(export))]
pub struct UpsertToolMetadataRequest {
    #[serde(flatten)]
    pub metadata: ToolMetadata,
    #[serde(default = "default_enabled")]
    #[cfg_attr(feature = "ts-export", ts(as = "Option<bool>", optional))]
    pub enabled: bool,
}

fn default_enabled() -> bool {
    true
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(JsonSchema))]
#[cfg_attr(feature = "openapi", derive(ToSchema))]
#[cfg_attr(feature = "ts-export", derive(TS))]
#[cfg_attr(feature = "ts-export", ts(export))]
pub struct ParamSpec {
    pub path: String,
    pub role: ParamRole,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub allowed_sources: Vec<AllowedSource>,
    /// Optional inclusive numeric bounds on this parameter's value, checked
    /// by the value-limit checker. `None` means the value is unconstrained.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts-export", ts(optional))]
    pub limit: Option<ParamLimit>,
}

/// Inclusive numeric bounds for a parameter, expressed in the tool's own
/// minor units (e.g. cents for a money amount). The value-limit checker
/// reads the parameter at [`ParamSpec::path`], coerces it to an integer,
/// and emits [`on_breach`](ParamLimit::on_breach) when it falls outside
/// `[min, max]`. Integer units keep the type `Eq` and sidestep float
/// comparison ambiguity; the checker is currency-agnostic, so the registry
/// author is responsible for declaring `max`/`min` in the same unit the
/// agent passes.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(JsonSchema))]
#[cfg_attr(feature = "openapi", derive(ToSchema))]
#[cfg_attr(feature = "ts-export", derive(TS))]
#[cfg_attr(feature = "ts-export", ts(export))]
pub struct ParamLimit {
    /// Inclusive maximum. A value greater than this breaches the limit.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts-export", ts(optional))]
    pub max: Option<i64>,
    /// Inclusive minimum. A value less than this breaches the limit.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts-export", ts(optional))]
    pub min: Option<i64>,
    /// Authorization effect to recommend when a bound is breached. Defaults to `Deny`.
    #[serde(default)]
    pub on_breach: LimitAction,
}

/// The authorization effect a [`ParamLimit`] breach maps to. `Deny` is the safe
/// default for money movement; `RequireApproval` routes a breach to review.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[cfg_attr(feature = "schema", derive(JsonSchema))]
#[cfg_attr(feature = "openapi", derive(ToSchema))]
#[cfg_attr(feature = "ts-export", derive(TS))]
#[cfg_attr(feature = "ts-export", ts(export))]
pub enum LimitAction {
    #[default]
    Deny,
    RequireApproval,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[cfg_attr(feature = "schema", derive(JsonSchema))]
#[cfg_attr(feature = "openapi", derive(ToSchema))]
#[cfg_attr(feature = "ts-export", derive(TS))]
#[cfg_attr(feature = "ts-export", ts(export))]
pub enum ParamRole {
    AuthorityBearing,
    ContentBearing,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(JsonSchema))]
#[cfg_attr(feature = "openapi", derive(ToSchema))]
#[cfg_attr(feature = "ts-export", derive(TS))]
#[cfg_attr(feature = "ts-export", ts(export))]
pub struct AllowedSource {
    pub origin: Origin,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts-export", ts(optional))]
    pub source_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts-export", ts(optional))]
    pub kind: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(JsonSchema))]
#[cfg_attr(feature = "openapi", derive(ToSchema))]
#[cfg_attr(feature = "ts-export", derive(TS))]
#[cfg_attr(feature = "ts-export", ts(export))]
pub struct ApprovalRule {
    pub required: bool,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub approver_roles: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts-export", ts(optional))]
    pub reason: Option<String>,
}
