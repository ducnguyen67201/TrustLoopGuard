//! Policy authoring API contract types.
//!
//! Policy syntax and validation live in `tl-policy`; these DTOs describe the
//! stable HTTP response shape used by the server, docs, and SDK generators.

use serde::{Deserialize, Serialize};

use crate::Severity;

#[cfg(feature = "schema")]
use schemars::JsonSchema;
#[cfg(feature = "ts-export")]
use ts_rs::TS;
#[cfg(feature = "openapi")]
use utoipa::ToSchema;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(JsonSchema))]
#[cfg_attr(feature = "openapi", derive(ToSchema))]
#[cfg_attr(feature = "ts-export", derive(TS))]
#[cfg_attr(feature = "ts-export", ts(export))]
pub struct PolicyValidateResponse {
    pub valid: bool,
    #[serde(default)]
    #[serde(skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts-export", ts(optional))]
    pub policy_id: Option<String>,
    pub errors: Vec<PolicyValidationIssue>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(JsonSchema))]
#[cfg_attr(feature = "openapi", derive(ToSchema))]
#[cfg_attr(feature = "ts-export", derive(TS))]
#[cfg_attr(feature = "ts-export", ts(export))]
pub struct PolicyValidationIssue {
    pub path: String,
    pub message: String,
}

/// Stable family discriminator for a policy document.
///
/// `Content` is the legacy/default family used when a policy document does not
/// contain a top-level `family:` tag. The other variants are typed policy
/// families with their own evaluators and product forms, but they share the
/// same policy registry, versioning, and environment deployment lifecycle.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(JsonSchema))]
#[cfg_attr(feature = "openapi", derive(ToSchema))]
#[cfg_attr(feature = "ts-export", derive(TS))]
#[cfg_attr(feature = "ts-export", ts(export))]
#[serde(rename_all = "snake_case")]
pub enum PolicyFamily {
    Content,
    Flow,
    ParameterSource,
    Approval,
    Memory,
    Financial,
    SourceLabel,
}

impl PolicyFamily {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Content => "content",
            Self::Flow => "flow",
            Self::ParameterSource => "parameter_source",
            Self::Approval => "approval",
            Self::Memory => "memory",
            Self::Financial => "financial",
            Self::SourceLabel => "source_label",
        }
    }
}

impl std::fmt::Display for PolicyFamily {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(JsonSchema))]
#[cfg_attr(feature = "openapi", derive(ToSchema))]
#[cfg_attr(feature = "ts-export", derive(TS))]
#[cfg_attr(feature = "ts-export", ts(export))]
pub struct PolicySummary {
    pub id: String,
    #[serde(default = "default_policy_family")]
    pub family: PolicyFamily,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts-export", ts(optional))]
    pub description: Option<String>,
    pub severity: Severity,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts-export", ts(optional))]
    pub action: Option<String>,
    pub enabled: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts-export", ts(optional))]
    pub owner_agent_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(JsonSchema))]
#[cfg_attr(feature = "openapi", derive(ToSchema))]
#[cfg_attr(feature = "ts-export", derive(TS))]
#[cfg_attr(feature = "ts-export", ts(export))]
pub struct PolicyDocument {
    pub id: String,
    #[serde(default = "default_policy_family")]
    pub family: PolicyFamily,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts-export", ts(optional))]
    pub description: Option<String>,
    pub severity: Severity,
    pub enabled: bool,
    pub source_yaml: String,
}

const fn default_policy_family() -> PolicyFamily {
    PolicyFamily::Content
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(JsonSchema))]
#[cfg_attr(feature = "openapi", derive(ToSchema))]
#[cfg_attr(feature = "ts-export", derive(TS))]
#[cfg_attr(feature = "ts-export", ts(export))]
pub struct PolicyListResponse {
    pub policies: Vec<PolicySummary>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(JsonSchema))]
#[cfg_attr(feature = "openapi", derive(ToSchema))]
#[cfg_attr(feature = "ts-export", derive(TS))]
#[cfg_attr(feature = "ts-export", ts(export))]
pub struct PolicySetEnabledRequest {
    pub enabled: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(JsonSchema))]
#[cfg_attr(feature = "openapi", derive(ToSchema))]
#[cfg_attr(feature = "ts-export", derive(TS))]
#[cfg_attr(feature = "ts-export", ts(export))]
pub struct PolicyBatchSetEnabledRequest {
    pub ids: Vec<String>,
    pub enabled: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(JsonSchema))]
#[cfg_attr(feature = "openapi", derive(ToSchema))]
#[cfg_attr(feature = "ts-export", derive(TS))]
#[cfg_attr(feature = "ts-export", ts(export))]
pub struct PolicyBatchSetEnabledResponse {
    pub policies: Vec<PolicySummary>,
}

/// Match-type discriminator for a `PolicyDraft`. Mirrors the YAML shape:
/// `match: { literal: "..." }`, `match: { regex: "..." }`, or
/// `match: { semantic: "..." }`. `semantic` is evaluated at runtime by the
/// LLM policy judge, so it survives paraphrase/encoding that literal and
/// regex matchers miss.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(JsonSchema))]
#[cfg_attr(feature = "openapi", derive(ToSchema))]
#[cfg_attr(feature = "ts-export", derive(TS))]
#[cfg_attr(feature = "ts-export", ts(export))]
#[serde(rename_all = "lowercase")]
pub enum PolicyMatchType {
    Literal,
    Regex,
    Semantic,
}

/// Action a policy takes when matched.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(JsonSchema))]
#[cfg_attr(feature = "openapi", derive(ToSchema))]
#[cfg_attr(feature = "ts-export", derive(TS))]
#[cfg_attr(feature = "ts-export", ts(export))]
#[serde(rename_all = "lowercase")]
pub enum PolicyAction {
    Block,
    Rewrite,
    Escalate,
}

/// LLM-drafted policy skeleton. Returned by `POST /v1/policies/draft`,
/// rendered as YAML by the caller, then submitted to `/v1/policies` as
/// usual. The server does not persist drafts.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(JsonSchema))]
#[cfg_attr(feature = "openapi", derive(ToSchema))]
#[cfg_attr(feature = "ts-export", derive(TS))]
#[cfg_attr(feature = "ts-export", ts(export))]
pub struct PolicyDraft {
    pub id: String,
    pub description: String,
    pub match_type: PolicyMatchType,
    pub match_value: String,
    pub action: PolicyAction,
    pub severity: Severity,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts-export", ts(optional))]
    pub rewrite: Option<String>,
}

/// Natural-language description posted to `POST /v1/policies/draft`.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(JsonSchema))]
#[cfg_attr(feature = "openapi", derive(ToSchema))]
#[cfg_attr(feature = "ts-export", derive(TS))]
#[cfg_attr(feature = "ts-export", ts(export))]
pub struct PolicyDraftRequest {
    pub prompt: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(JsonSchema))]
#[cfg_attr(feature = "openapi", derive(ToSchema))]
#[cfg_attr(feature = "ts-export", derive(TS))]
#[cfg_attr(feature = "ts-export", ts(export))]
pub struct PolicyDraftResponse {
    pub draft: PolicyDraft,
}

/// Successful response from `POST /v1/agents/{id}/guardrails:generate`.
///
/// Each item is a freshly persisted policy with `enabled=false` —
/// callers review the set and flip individual policies on via
/// `PATCH /v1/policies/{id}/enabled`.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(JsonSchema))]
#[cfg_attr(feature = "openapi", derive(ToSchema))]
#[cfg_attr(feature = "ts-export", derive(TS))]
#[cfg_attr(feature = "ts-export", ts(export))]
pub struct GuardrailGenerateResponse {
    pub generated: Vec<PolicyDocument>,
}

/// Response from `GET /v1/agents/{id}/guardrails`.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(JsonSchema))]
#[cfg_attr(feature = "openapi", derive(ToSchema))]
#[cfg_attr(feature = "ts-export", derive(TS))]
#[cfg_attr(feature = "ts-export", ts(export))]
pub struct GuardrailListResponse {
    pub policies: Vec<PolicySummary>,
}

/// One entry in a version history list — entity-agnostic.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(JsonSchema))]
#[cfg_attr(feature = "openapi", derive(ToSchema))]
#[cfg_attr(feature = "ts-export", derive(TS))]
#[cfg_attr(feature = "ts-export", ts(export))]
pub struct EntityVersionSummary {
    pub version: i32,
    pub created_at: String, // RFC-3339
}

/// Response from `GET /v1/{entity-type}/{id}/versions`.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(JsonSchema))]
#[cfg_attr(feature = "openapi", derive(ToSchema))]
#[cfg_attr(feature = "ts-export", derive(TS))]
#[cfg_attr(feature = "ts-export", ts(export))]
pub struct EntityVersionListResponse {
    pub versions: Vec<EntityVersionSummary>,
}

/// Response from `GET /v1/{entity-type}/{id}/versions/{version}`.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(JsonSchema))]
#[cfg_attr(feature = "openapi", derive(ToSchema))]
#[cfg_attr(feature = "ts-export", derive(TS))]
#[cfg_attr(feature = "ts-export", ts(export))]
pub struct EntityVersionDetail {
    pub version: i32,
    pub content: String,    // the entity YAML
    pub created_at: String, // RFC-3339
}

/// Response from `POST /v1/policies/ai-edit`.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(JsonSchema))]
#[cfg_attr(feature = "openapi", derive(ToSchema))]
#[cfg_attr(feature = "ts-export", derive(TS))]
#[cfg_attr(feature = "ts-export", ts(export))]
pub struct AiEditRequest {
    pub yaml: String,
    pub instruction: String,
}

/// Response from `POST /v1/policies/ai-edit`.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(JsonSchema))]
#[cfg_attr(feature = "openapi", derive(ToSchema))]
#[cfg_attr(feature = "ts-export", derive(TS))]
#[cfg_attr(feature = "ts-export", ts(export))]
pub struct AiEditResponse {
    pub yaml: String,
}
