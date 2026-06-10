use serde::{Deserialize, Serialize};

use crate::{Confidentiality, Integrity, Origin, Trust};

#[cfg(feature = "schema")]
use schemars::JsonSchema;
#[cfg(feature = "ts-export")]
use ts_rs::TS;
#[cfg(feature = "openapi")]
use utoipa::ToSchema;

/// One workspace override row: per-origin label defaults. A family left
/// `None` inherits the built-in origin default.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(JsonSchema))]
#[cfg_attr(feature = "openapi", derive(ToSchema))]
#[cfg_attr(feature = "ts-export", derive(TS))]
#[cfg_attr(feature = "ts-export", ts(export))]
pub struct SourceLabelPolicy {
    pub origin: Origin,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts-export", ts(optional))]
    pub trust: Option<Trust>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts-export", ts(optional))]
    pub confidentiality: Option<Confidentiality>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts-export", ts(optional))]
    pub integrity: Option<Integrity>,
}

/// A registry row as seen by the control plane: the wire policy plus
/// its `enabled` flag. Disabled policies stay manageable but are
/// skipped during runtime label resolution.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(JsonSchema))]
#[cfg_attr(feature = "openapi", derive(ToSchema))]
#[cfg_attr(feature = "ts-export", derive(TS))]
#[cfg_attr(feature = "ts-export", ts(export))]
pub struct SourceLabelPolicyEntry {
    pub policy: SourceLabelPolicy,
    pub enabled: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(JsonSchema))]
#[cfg_attr(feature = "openapi", derive(ToSchema))]
#[cfg_attr(feature = "ts-export", derive(TS))]
#[cfg_attr(feature = "ts-export", ts(export))]
pub struct SourceLabelPolicyListResponse {
    pub policies: Vec<SourceLabelPolicyEntry>,
}

/// Upsert body for `POST /v1/label-policies`: the wire policy plus an
/// optional `enabled` flag (defaults to `true`).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(JsonSchema))]
#[cfg_attr(feature = "openapi", derive(ToSchema))]
#[cfg_attr(feature = "ts-export", derive(TS))]
#[cfg_attr(feature = "ts-export", ts(export))]
pub struct UpsertSourceLabelPolicyRequest {
    #[serde(flatten)]
    pub policy: SourceLabelPolicy,
    #[serde(default = "default_enabled")]
    #[cfg_attr(feature = "ts-export", ts(as = "Option<bool>", optional))]
    pub enabled: bool,
}

fn default_enabled() -> bool {
    true
}
