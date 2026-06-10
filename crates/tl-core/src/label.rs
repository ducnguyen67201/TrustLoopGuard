use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

#[cfg(feature = "schema")]
use schemars::JsonSchema;
#[cfg(feature = "ts-export")]
use ts_rs::TS;
#[cfg(feature = "openapi")]
use utoipa::ToSchema;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(JsonSchema))]
#[cfg_attr(feature = "openapi", derive(ToSchema))]
#[cfg_attr(feature = "ts-export", derive(TS))]
#[cfg_attr(feature = "ts-export", ts(export))]
pub struct Source {
    pub id: String,
    pub origin: Origin,
    #[serde(default)]
    pub labels: Labels,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts-export", ts(optional))]
    pub kind: Option<String>,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[cfg_attr(feature = "schema", derive(JsonSchema))]
#[cfg_attr(feature = "openapi", derive(ToSchema))]
#[cfg_attr(feature = "ts-export", derive(TS))]
#[cfg_attr(feature = "ts-export", ts(export))]
pub enum Origin {
    User,
    System,
    Tool,
    Memory,
    File,
    Web,
    Email,
    Api,
    #[default]
    Unknown,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[cfg_attr(feature = "schema", derive(JsonSchema))]
#[cfg_attr(feature = "openapi", derive(ToSchema))]
#[cfg_attr(feature = "ts-export", derive(TS))]
#[cfg_attr(feature = "ts-export", ts(export))]
pub enum Trust {
    Trusted,
    Untrusted,
    #[default]
    Unknown,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[cfg_attr(feature = "schema", derive(JsonSchema))]
#[cfg_attr(feature = "openapi", derive(ToSchema))]
#[cfg_attr(feature = "ts-export", derive(TS))]
#[cfg_attr(feature = "ts-export", ts(export))]
pub enum Confidentiality {
    Public,
    Private,
    Secret,
    Identity,
    #[default]
    Unknown,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[cfg_attr(feature = "schema", derive(JsonSchema))]
#[cfg_attr(feature = "openapi", derive(ToSchema))]
#[cfg_attr(feature = "ts-export", derive(TS))]
#[cfg_attr(feature = "ts-export", ts(export))]
pub enum Integrity {
    Low,
    Medium,
    High,
    #[default]
    Unknown,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(JsonSchema))]
#[cfg_attr(feature = "openapi", derive(ToSchema))]
#[cfg_attr(feature = "ts-export", derive(TS))]
#[cfg_attr(feature = "ts-export", ts(export))]
pub struct Labels {
    #[serde(default)]
    pub trust: Trust,
    #[serde(default)]
    pub confidentiality: Confidentiality,
    #[serde(default)]
    pub integrity: Integrity,
}

/// Why a resolved label family value was chosen.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[cfg_attr(feature = "schema", derive(JsonSchema))]
#[cfg_attr(feature = "openapi", derive(ToSchema))]
#[cfg_attr(feature = "ts-export", derive(TS))]
#[cfg_attr(feature = "ts-export", ts(export))]
pub enum LabelBasis {
    /// Built-in default derived from the source origin.
    OriginDefault,
    /// An enabled workspace `SourceLabelPolicy` override applied.
    WorkspaceOverride,
    /// The producer declared the value and it was accepted.
    Declared,
}

/// Per-family basis for one source's resolved labels.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(JsonSchema))]
#[cfg_attr(feature = "openapi", derive(ToSchema))]
#[cfg_attr(feature = "ts-export", derive(TS))]
#[cfg_attr(feature = "ts-export", ts(export))]
pub struct LabelBasisSet {
    pub trust: LabelBasis,
    pub confidentiality: LabelBasis,
    pub integrity: LabelBasis,
}

/// How the workspace label policy read went during resolution.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[cfg_attr(feature = "schema", derive(JsonSchema))]
#[cfg_attr(feature = "openapi", derive(ToSchema))]
#[cfg_attr(feature = "ts-export", derive(TS))]
#[cfg_attr(feature = "ts-export", ts(export))]
pub enum LabelPolicyStatus {
    /// No enabled policy rows configured for the workspace.
    NotConfigured,
    /// Enabled policy rows were loaded and consulted.
    Applied,
    /// The policy store could not be consulted; built-in defaults used.
    Unavailable,
}

/// Evidence of how one source's labels were resolved.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(JsonSchema))]
#[cfg_attr(feature = "openapi", derive(ToSchema))]
#[cfg_attr(feature = "ts-export", derive(TS))]
#[cfg_attr(feature = "ts-export", ts(export))]
pub struct SourceLabelEvidence {
    pub source_id: String,
    pub labels: Labels,
    pub basis: LabelBasisSet,
}

/// Label resolution + propagation evidence attached by the event
/// pipeline. `None` until the pipeline has run. Observe-only: no
/// checker changes a verdict because of this value.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(JsonSchema))]
#[cfg_attr(feature = "openapi", derive(ToSchema))]
#[cfg_attr(feature = "ts-export", derive(TS))]
#[cfg_attr(feature = "ts-export", ts(export))]
pub struct LabelResolution {
    pub policy_status: LabelPolicyStatus,
    // Empty collections are skipped on the wire, so the exported TS
    // type must mark them optional or consumers type-check against
    // fields that common traces (no sources / no provenance) omit.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    #[cfg_attr(
        feature = "ts-export",
        ts(as = "Option<Vec<SourceLabelEvidence>>", optional)
    )]
    pub sources: Vec<SourceLabelEvidence>,
    /// Parameter path -> labels derived over provenance. A path absent
    /// from this map has unknown derivation — absence is never "clean".
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    #[cfg_attr(
        feature = "ts-export",
        ts(as = "Option<BTreeMap<String, Labels>>", optional)
    )]
    pub derived: BTreeMap<String, Labels>,
}
