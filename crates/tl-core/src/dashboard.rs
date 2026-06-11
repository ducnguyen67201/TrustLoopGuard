use serde::{Deserialize, Serialize};

use crate::EnforcementMode;

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
pub struct DashboardApiKey {
    pub id: String,
    pub name: String,
    pub prefix: String,
    pub environment_id: String,
    pub environment: String,
    pub status: String,
    /// RFC 3339 timestamp.
    pub created_at: String,
    /// RFC 3339 timestamp.
    pub last_used_at: Option<String>,
    pub created_by: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(JsonSchema))]
#[cfg_attr(feature = "openapi", derive(ToSchema))]
#[cfg_attr(feature = "ts-export", derive(TS))]
#[cfg_attr(feature = "ts-export", ts(export))]
pub struct ApiKeyListResponse {
    pub api_keys: Vec<DashboardApiKey>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(JsonSchema))]
#[cfg_attr(feature = "openapi", derive(ToSchema))]
#[cfg_attr(feature = "ts-export", derive(TS))]
#[cfg_attr(feature = "ts-export", ts(export))]
pub struct ApiKeyBatchRevokeRequest {
    pub ids: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(JsonSchema))]
#[cfg_attr(feature = "openapi", derive(ToSchema))]
#[cfg_attr(feature = "ts-export", derive(TS))]
#[cfg_attr(feature = "ts-export", ts(export))]
pub struct ApiKeyBatchRevokeResponse {
    pub api_keys: Vec<DashboardApiKey>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(JsonSchema))]
#[cfg_attr(feature = "openapi", derive(ToSchema))]
#[cfg_attr(feature = "ts-export", derive(TS))]
#[cfg_attr(feature = "ts-export", ts(export))]
pub struct CreateApiKeyRequest {
    pub name: String,
    #[serde(default)]
    #[cfg_attr(feature = "ts-export", ts(optional))]
    pub environment_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(JsonSchema))]
#[cfg_attr(feature = "openapi", derive(ToSchema))]
#[cfg_attr(feature = "ts-export", derive(TS))]
#[cfg_attr(feature = "ts-export", ts(export))]
pub struct WorkspaceEnvironment {
    pub id: String,
    pub slug: String,
    pub name: String,
    #[serde(default)]
    #[cfg_attr(feature = "ts-export", ts(optional))]
    pub description: Option<String>,
    pub is_default: bool,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(JsonSchema))]
#[cfg_attr(feature = "openapi", derive(ToSchema))]
#[cfg_attr(feature = "ts-export", derive(TS))]
#[cfg_attr(feature = "ts-export", ts(export))]
pub struct WorkspaceEnvironmentListResponse {
    pub environments: Vec<WorkspaceEnvironment>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(JsonSchema))]
#[cfg_attr(feature = "openapi", derive(ToSchema))]
#[cfg_attr(feature = "ts-export", derive(TS))]
#[cfg_attr(feature = "ts-export", ts(export))]
pub struct CreateWorkspaceEnvironmentRequest {
    pub slug: String,
    pub name: String,
    #[serde(default)]
    #[cfg_attr(feature = "ts-export", ts(optional))]
    pub description: Option<String>,
    #[serde(default)]
    pub is_default: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(JsonSchema))]
#[cfg_attr(feature = "openapi", derive(ToSchema))]
#[cfg_attr(feature = "ts-export", derive(TS))]
#[cfg_attr(feature = "ts-export", ts(export))]
pub struct UpdateWorkspaceEnvironmentRequest {
    #[serde(default)]
    #[cfg_attr(feature = "ts-export", ts(optional))]
    pub slug: Option<String>,
    #[serde(default)]
    #[cfg_attr(feature = "ts-export", ts(optional))]
    pub name: Option<String>,
    #[serde(default)]
    #[cfg_attr(feature = "ts-export", ts(optional))]
    pub description: Option<String>,
    #[serde(default)]
    #[cfg_attr(feature = "ts-export", ts(optional))]
    pub is_default: Option<bool>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(JsonSchema))]
#[cfg_attr(feature = "openapi", derive(ToSchema))]
#[cfg_attr(feature = "ts-export", derive(TS))]
#[cfg_attr(feature = "ts-export", ts(export))]
pub struct CreateApiKeyResponse {
    pub api_key: DashboardApiKey,
    /// Full bearer key. Returned only once at creation time.
    pub plaintext_key: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(JsonSchema))]
#[cfg_attr(feature = "openapi", derive(ToSchema))]
#[cfg_attr(feature = "ts-export", derive(TS))]
#[cfg_attr(feature = "ts-export", ts(export))]
pub struct WorkspaceSettings {
    pub default_action: String,
    pub escalation_webhook_url: Option<String>,
    pub telemetry_enabled: bool,
    pub retention_days: String,
    #[serde(default)]
    pub data_handling_mode: DataHandlingMode,
    /// Rollout mode for the information-flow checker. Default `off`.
    #[serde(default)]
    pub flow_checker_mode: EnforcementMode,
    /// Rollout mode for the memory write-time checker. Default `off`.
    #[serde(default)]
    pub memory_checker_mode: EnforcementMode,
    /// Rollout mode for the parameter-source authorization checker.
    /// Default `off`.
    #[serde(default)]
    pub param_checker_mode: EnforcementMode,
    /// Rollout mode for the approval checker. Default `off`.
    #[serde(default)]
    pub approval_checker_mode: EnforcementMode,
    #[cfg_attr(feature = "ts-export", ts(type = "Record<string, unknown>"))]
    pub config: serde_json::Value,
    /// RFC 3339 timestamp.
    pub updated_at: Option<String>,
}

/// Partial update for workspace runtime settings. Absent fields are
/// left unchanged.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(JsonSchema))]
#[cfg_attr(feature = "openapi", derive(ToSchema))]
#[cfg_attr(feature = "ts-export", derive(TS))]
#[cfg_attr(feature = "ts-export", ts(export))]
pub struct UpdateWorkspaceSettingsRequest {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts-export", ts(optional))]
    pub default_action: Option<String>,
    /// Replacement escalation webhook URL. Absent and `null` both mean
    /// "leave unchanged" — this endpoint cannot clear the URL once set.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts-export", ts(optional))]
    pub escalation_webhook_url: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts-export", ts(optional))]
    pub telemetry_enabled: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts-export", ts(optional))]
    pub retention_days: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts-export", ts(optional))]
    pub data_handling_mode: Option<DataHandlingMode>,
    /// Rollout mode for the information-flow checker.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts-export", ts(optional))]
    pub flow_checker_mode: Option<EnforcementMode>,
    /// Rollout mode for the memory write-time checker.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts-export", ts(optional))]
    pub memory_checker_mode: Option<EnforcementMode>,
    /// Rollout mode for the parameter-source authorization checker.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts-export", ts(optional))]
    pub param_checker_mode: Option<EnforcementMode>,
    /// Rollout mode for the approval checker.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts-export", ts(optional))]
    pub approval_checker_mode: Option<EnforcementMode>,
}

impl UpdateWorkspaceSettingsRequest {
    /// Merge this partial update onto current settings. Absent fields are
    /// left unchanged; the result is what stores persist. Stores stamp
    /// `updated_at` when persisting — the merged value carries the
    /// current timestamp only as a placeholder.
    ///
    /// NOTE: `escalation_webhook_url` cannot be cleared through this
    /// merge — serde collapses an explicit JSON `null` and an absent
    /// field into the same `None`, so both mean "leave unchanged".
    /// Clearing the URL needs a tri-state (`Option<Option<String>>`) on
    /// the wire type.
    pub fn apply_to(&self, current: &WorkspaceSettings) -> WorkspaceSettings {
        WorkspaceSettings {
            default_action: self
                .default_action
                .clone()
                .unwrap_or_else(|| current.default_action.clone()),
            escalation_webhook_url: self
                .escalation_webhook_url
                .clone()
                .or_else(|| current.escalation_webhook_url.clone()),
            telemetry_enabled: self.telemetry_enabled.unwrap_or(current.telemetry_enabled),
            retention_days: self
                .retention_days
                .clone()
                .unwrap_or_else(|| current.retention_days.clone()),
            data_handling_mode: self
                .data_handling_mode
                .unwrap_or(current.data_handling_mode),
            flow_checker_mode: self.flow_checker_mode.unwrap_or(current.flow_checker_mode),
            memory_checker_mode: self
                .memory_checker_mode
                .unwrap_or(current.memory_checker_mode),
            param_checker_mode: self
                .param_checker_mode
                .unwrap_or(current.param_checker_mode),
            approval_checker_mode: self
                .approval_checker_mode
                .unwrap_or(current.approval_checker_mode),
            config: current.config.clone(),
            updated_at: current.updated_at.clone(),
        }
    }
}

/// Replacement per-environment checker-mode overrides for
/// `PUT /v1/environments/{environment_id}/checker-modes`. Omitted fields
/// inherit the workspace-level modes. Server-stamped fields
/// (`updated_at`) are deliberately absent: the response shape is
/// [`EnvironmentCheckerModes`].
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(JsonSchema))]
#[cfg_attr(feature = "openapi", derive(ToSchema))]
#[cfg_attr(feature = "ts-export", derive(TS))]
#[cfg_attr(feature = "ts-export", ts(export))]
pub struct UpdateEnvironmentCheckerModesRequest {
    /// Override for the information-flow checker. Omitted inherits.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts-export", ts(optional))]
    pub flow_checker_mode: Option<EnforcementMode>,
    /// Override for the memory write-time checker. Omitted inherits.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts-export", ts(optional))]
    pub memory_checker_mode: Option<EnforcementMode>,
    /// Override for the parameter-source authorization checker.
    /// Omitted inherits.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts-export", ts(optional))]
    pub param_checker_mode: Option<EnforcementMode>,
    /// Override for the approval checker. Omitted inherits.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts-export", ts(optional))]
    pub approval_checker_mode: Option<EnforcementMode>,
}

impl From<UpdateEnvironmentCheckerModesRequest> for EnvironmentCheckerModes {
    fn from(request: UpdateEnvironmentCheckerModesRequest) -> Self {
        EnvironmentCheckerModes {
            flow_checker_mode: request.flow_checker_mode,
            memory_checker_mode: request.memory_checker_mode,
            param_checker_mode: request.param_checker_mode,
            approval_checker_mode: request.approval_checker_mode,
            updated_at: None,
        }
    }
}

/// Per-environment checker-mode overrides. A `None` field inherits the
/// workspace-level mode from [`WorkspaceSettings`].
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(JsonSchema))]
#[cfg_attr(feature = "openapi", derive(ToSchema))]
#[cfg_attr(feature = "ts-export", derive(TS))]
#[cfg_attr(feature = "ts-export", ts(export))]
pub struct EnvironmentCheckerModes {
    /// Override for the information-flow checker. `None` inherits.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts-export", ts(optional))]
    pub flow_checker_mode: Option<EnforcementMode>,
    /// Override for the memory write-time checker. `None` inherits.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts-export", ts(optional))]
    pub memory_checker_mode: Option<EnforcementMode>,
    /// Override for the parameter-source authorization checker.
    /// `None` inherits.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts-export", ts(optional))]
    pub param_checker_mode: Option<EnforcementMode>,
    /// Override for the approval checker. `None` inherits.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts-export", ts(optional))]
    pub approval_checker_mode: Option<EnforcementMode>,
    /// RFC 3339 timestamp.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts-export", ts(optional))]
    pub updated_at: Option<String>,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[cfg_attr(feature = "schema", derive(JsonSchema))]
#[cfg_attr(feature = "openapi", derive(ToSchema))]
#[cfg_attr(feature = "ts-export", derive(TS))]
#[cfg_attr(feature = "ts-export", ts(export))]
pub enum DataHandlingMode {
    #[default]
    RawAllowed,
    RedactedOnly,
    NoBodyRetention,
    PrivateDeployment,
}
