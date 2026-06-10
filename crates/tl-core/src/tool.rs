use serde::{Deserialize, Serialize};

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
/// workspace tool-metadata registry. Evidence only — observe-only phases
/// never change a decision because of this value.
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
    /// stay accurate forensic evidence; the collector-claimed side
    /// effect is left untouched and the decision is unaffected.
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
