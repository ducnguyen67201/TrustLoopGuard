use serde::{Deserialize, Serialize};

#[cfg(feature = "schema")]
use schemars::JsonSchema;
#[cfg(feature = "ts-export")]
use ts_rs::TS;
#[cfg(feature = "openapi")]
use utoipa::ToSchema;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[cfg_attr(feature = "schema", derive(JsonSchema))]
#[cfg_attr(feature = "openapi", derive(ToSchema))]
#[cfg_attr(feature = "ts-export", derive(TS))]
#[cfg_attr(feature = "ts-export", ts(export))]
pub enum GatewayProviderKind {
    OpenaiCompatible,
    Anthropic,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[cfg_attr(feature = "schema", derive(JsonSchema))]
#[cfg_attr(feature = "openapi", derive(ToSchema))]
#[cfg_attr(feature = "ts-export", derive(TS))]
#[cfg_attr(feature = "ts-export", ts(export))]
pub enum GatewayCredentialStatus {
    Configured,
    Missing,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[cfg_attr(feature = "schema", derive(JsonSchema))]
#[cfg_attr(feature = "openapi", derive(ToSchema))]
#[cfg_attr(feature = "ts-export", derive(TS))]
#[cfg_attr(feature = "ts-export", ts(export))]
pub enum GatewayInputAction {
    Allow,
    Block,
    Redact,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[cfg_attr(feature = "schema", derive(JsonSchema))]
#[cfg_attr(feature = "openapi", derive(ToSchema))]
#[cfg_attr(feature = "ts-export", derive(TS))]
#[cfg_attr(feature = "ts-export", ts(export))]
pub enum GatewayOutputAction {
    Allow,
    Block,
    Rewrite,
    Escalate,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[cfg_attr(feature = "schema", derive(JsonSchema))]
#[cfg_attr(feature = "openapi", derive(ToSchema))]
#[cfg_attr(feature = "ts-export", derive(TS))]
#[cfg_attr(feature = "ts-export", ts(export))]
pub enum FailMode {
    Open,
    Closed,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[cfg_attr(feature = "schema", derive(JsonSchema))]
#[cfg_attr(feature = "openapi", derive(ToSchema))]
#[cfg_attr(feature = "ts-export", derive(TS))]
#[cfg_attr(feature = "ts-export", ts(export))]
pub enum RetentionMode {
    MetadataOnly,
    RedactedBody,
    FullBody,
}

/// How the gateway returns guarded responses for a route.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[cfg_attr(feature = "schema", derive(JsonSchema))]
#[cfg_attr(feature = "openapi", derive(ToSchema))]
#[cfg_attr(feature = "ts-export", derive(TS))]
#[cfg_attr(feature = "ts-export", ts(export))]
pub enum ResponseMode {
    /// Buffered JSON only; a streaming (`stream:true`) request is rejected.
    #[default]
    Regular,
    /// Streaming clients are allowed; the guarded result is emitted as SSE.
    Streaming,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(JsonSchema))]
#[cfg_attr(feature = "openapi", derive(ToSchema))]
#[cfg_attr(feature = "ts-export", derive(TS))]
#[cfg_attr(feature = "ts-export", ts(export))]
pub struct GatewayProviderConnection {
    pub id: String,
    pub display_name: String,
    pub kind: GatewayProviderKind,
    pub base_url: Option<String>,
    pub default_model: String,
    pub credential_status: GatewayCredentialStatus,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(JsonSchema))]
#[cfg_attr(feature = "openapi", derive(ToSchema))]
#[cfg_attr(feature = "ts-export", derive(TS))]
#[cfg_attr(feature = "ts-export", ts(export))]
pub struct CreateGatewayProviderConnectionRequest {
    #[serde(default)]
    #[cfg_attr(feature = "ts-export", ts(optional))]
    pub id: Option<String>,
    pub display_name: String,
    pub kind: GatewayProviderKind,
    #[serde(default)]
    #[cfg_attr(feature = "ts-export", ts(optional))]
    pub base_url: Option<String>,
    pub default_model: String,
    pub provider_api_key: String,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(JsonSchema))]
#[cfg_attr(feature = "openapi", derive(ToSchema))]
#[cfg_attr(feature = "ts-export", derive(TS))]
#[cfg_attr(feature = "ts-export", ts(export))]
pub struct UpdateGatewayProviderConnectionRequest {
    #[serde(default)]
    #[cfg_attr(feature = "ts-export", ts(optional))]
    pub display_name: Option<String>,
    #[serde(default)]
    #[cfg_attr(feature = "ts-export", ts(optional))]
    pub base_url: Option<String>,
    #[serde(default)]
    #[cfg_attr(feature = "ts-export", ts(optional))]
    pub default_model: Option<String>,
    #[serde(default)]
    #[cfg_attr(feature = "ts-export", ts(optional))]
    pub provider_api_key: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(JsonSchema))]
#[cfg_attr(feature = "openapi", derive(ToSchema))]
#[cfg_attr(feature = "ts-export", derive(TS))]
#[cfg_attr(feature = "ts-export", ts(export))]
pub struct GatewayProviderConnectionListResponse {
    pub provider_connections: Vec<GatewayProviderConnection>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(JsonSchema))]
#[cfg_attr(feature = "openapi", derive(ToSchema))]
#[cfg_attr(feature = "ts-export", derive(TS))]
#[cfg_attr(feature = "ts-export", ts(export))]
pub struct EnforcementProfile {
    pub id: String,
    pub display_name: String,
    pub input_action: GatewayInputAction,
    pub output_action: GatewayOutputAction,
    pub fail_mode: FailMode,
    pub retention_mode: RetentionMode,
    #[serde(default)]
    pub response_mode: ResponseMode,
    pub fallback_message: String,
    pub max_regenerations: u32,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(JsonSchema))]
#[cfg_attr(feature = "openapi", derive(ToSchema))]
#[cfg_attr(feature = "ts-export", derive(TS))]
#[cfg_attr(feature = "ts-export", ts(export))]
pub struct CreateEnforcementProfileRequest {
    #[serde(default)]
    #[cfg_attr(feature = "ts-export", ts(optional))]
    pub id: Option<String>,
    pub display_name: String,
    pub input_action: GatewayInputAction,
    pub output_action: GatewayOutputAction,
    pub fail_mode: FailMode,
    pub retention_mode: RetentionMode,
    #[serde(default)]
    pub response_mode: ResponseMode,
    pub fallback_message: String,
    #[serde(default)]
    pub max_regenerations: u32,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(JsonSchema))]
#[cfg_attr(feature = "openapi", derive(ToSchema))]
#[cfg_attr(feature = "ts-export", derive(TS))]
#[cfg_attr(feature = "ts-export", ts(export))]
pub struct UpdateEnforcementProfileRequest {
    #[serde(default)]
    #[cfg_attr(feature = "ts-export", ts(optional))]
    pub display_name: Option<String>,
    #[serde(default)]
    #[cfg_attr(feature = "ts-export", ts(optional))]
    pub input_action: Option<GatewayInputAction>,
    #[serde(default)]
    #[cfg_attr(feature = "ts-export", ts(optional))]
    pub output_action: Option<GatewayOutputAction>,
    #[serde(default)]
    #[cfg_attr(feature = "ts-export", ts(optional))]
    pub fail_mode: Option<FailMode>,
    #[serde(default)]
    #[cfg_attr(feature = "ts-export", ts(optional))]
    pub retention_mode: Option<RetentionMode>,
    #[serde(default)]
    #[cfg_attr(feature = "ts-export", ts(optional))]
    pub response_mode: Option<ResponseMode>,
    #[serde(default)]
    #[cfg_attr(feature = "ts-export", ts(optional))]
    pub fallback_message: Option<String>,
    #[serde(default)]
    #[cfg_attr(feature = "ts-export", ts(optional))]
    pub max_regenerations: Option<u32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(JsonSchema))]
#[cfg_attr(feature = "openapi", derive(ToSchema))]
#[cfg_attr(feature = "ts-export", derive(TS))]
#[cfg_attr(feature = "ts-export", ts(export))]
pub struct EnforcementProfileListResponse {
    pub enforcement_profiles: Vec<EnforcementProfile>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(JsonSchema))]
#[cfg_attr(feature = "openapi", derive(ToSchema))]
#[cfg_attr(feature = "ts-export", derive(TS))]
#[cfg_attr(feature = "ts-export", ts(export))]
pub struct GatewayRoute {
    pub id: String,
    pub display_name: String,
    pub provider_connection_id: String,
    pub agent_id: String,
    pub enforcement_profile_id: String,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(JsonSchema))]
#[cfg_attr(feature = "openapi", derive(ToSchema))]
#[cfg_attr(feature = "ts-export", derive(TS))]
#[cfg_attr(feature = "ts-export", ts(export))]
pub struct CreateGatewayRouteRequest {
    #[serde(default)]
    #[cfg_attr(feature = "ts-export", ts(optional))]
    pub id: Option<String>,
    pub display_name: String,
    pub provider_connection_id: String,
    pub agent_id: String,
    pub enforcement_profile_id: String,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(JsonSchema))]
#[cfg_attr(feature = "openapi", derive(ToSchema))]
#[cfg_attr(feature = "ts-export", derive(TS))]
#[cfg_attr(feature = "ts-export", ts(export))]
pub struct UpdateGatewayRouteRequest {
    #[serde(default)]
    #[cfg_attr(feature = "ts-export", ts(optional))]
    pub display_name: Option<String>,
    #[serde(default)]
    #[cfg_attr(feature = "ts-export", ts(optional))]
    pub provider_connection_id: Option<String>,
    #[serde(default)]
    #[cfg_attr(feature = "ts-export", ts(optional))]
    pub agent_id: Option<String>,
    #[serde(default)]
    #[cfg_attr(feature = "ts-export", ts(optional))]
    pub enforcement_profile_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(JsonSchema))]
#[cfg_attr(feature = "openapi", derive(ToSchema))]
#[cfg_attr(feature = "ts-export", derive(TS))]
#[cfg_attr(feature = "ts-export", ts(export))]
pub struct GatewayRouteListResponse {
    pub gateway_routes: Vec<GatewayRoute>,
}
