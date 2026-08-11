use serde::{Deserialize, Serialize};

use crate::{AgentEvaluationProfile, DataHandlingMode, NotificationRule};

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
    /// Generic HTTP payment endpoint. Not a valid LLM route target: the
    /// connection vaults a payment credential the pay gate injects on
    /// forward. Requires `base_url`; `default_model` is unused.
    PaymentHttp,
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

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[cfg_attr(feature = "schema", derive(JsonSchema))]
#[cfg_attr(feature = "openapi", derive(ToSchema))]
#[cfg_attr(feature = "ts-export", derive(TS))]
#[cfg_attr(feature = "ts-export", ts(export))]
pub enum GatewayReliabilityMode {
    #[default]
    None,
    Standard,
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
    /// Required for LLM kinds; unused (may be omitted) for `payment_http`.
    #[serde(default)]
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
pub struct GatewayRoute {
    pub id: String,
    pub display_name: String,
    pub provider_connection_id: String,
    pub agent_id: String,
    #[serde(default)]
    pub reliability_mode: GatewayReliabilityMode,
    #[serde(default)]
    pub fallback_provider_connection_ids: Vec<String>,
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
    #[serde(default)]
    pub reliability_mode: GatewayReliabilityMode,
    #[serde(default)]
    pub fallback_provider_connection_ids: Vec<String>,
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
    pub reliability_mode: Option<GatewayReliabilityMode>,
    #[serde(default)]
    #[cfg_attr(feature = "ts-export", ts(optional))]
    pub fallback_provider_connection_ids: Option<Vec<String>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(JsonSchema))]
#[cfg_attr(feature = "openapi", derive(ToSchema))]
#[cfg_attr(feature = "ts-export", derive(TS))]
#[cfg_attr(feature = "ts-export", ts(export))]
pub struct GatewayRouteListResponse {
    pub gateway_routes: Vec<GatewayRoute>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "mode", rename_all = "snake_case")]
#[cfg_attr(feature = "schema", derive(JsonSchema))]
#[cfg_attr(feature = "openapi", derive(ToSchema))]
#[cfg_attr(feature = "ts-export", derive(TS))]
#[cfg_attr(feature = "ts-export", ts(export))]
pub enum GatewayActivationAgentInput {
    Existing { agent_id: String },
    New { name: String, purpose: String },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(JsonSchema))]
#[cfg_attr(feature = "openapi", derive(ToSchema))]
#[cfg_attr(feature = "ts-export", derive(TS))]
#[cfg_attr(feature = "ts-export", ts(export))]
pub struct CreateGatewayActivationRequest {
    pub provider: CreateGatewayProviderConnectionRequest,
    pub agent: GatewayActivationAgentInput,
    pub route_display_name: String,
    pub alert_email: String,
    /// Explicit acknowledgment that email alerts are intentionally deferred.
    /// When true, `alert_email` may be empty and readiness remains
    /// `needs_attention` until an enabled rule and transport are configured.
    #[serde(default)]
    #[cfg_attr(feature = "ts-export", ts(optional))]
    pub alerts_deferred: Option<bool>,
    /// Exact customer correlation id used by the generated verification
    /// request. The server generates one when omitted.
    #[serde(default)]
    #[cfg_attr(feature = "ts-export", ts(optional))]
    pub verification_session_id: Option<String>,
    pub data_handling_mode: DataHandlingMode,
    #[serde(default)]
    pub confirm_workspace_privacy_change: bool,
    #[serde(default)]
    pub reliability_mode: GatewayReliabilityMode,
    #[serde(default)]
    pub fallback_provider_connection_ids: Vec<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[cfg_attr(feature = "schema", derive(JsonSchema))]
#[cfg_attr(feature = "openapi", derive(ToSchema))]
#[cfg_attr(feature = "ts-export", derive(TS))]
#[cfg_attr(feature = "ts-export", ts(export))]
pub enum ProductionReadinessStatus {
    Ready,
    NeedsAttention,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(JsonSchema))]
#[cfg_attr(feature = "openapi", derive(ToSchema))]
#[cfg_attr(feature = "ts-export", derive(TS))]
#[cfg_attr(feature = "ts-export", ts(export))]
pub struct ProductionReadinessCheck {
    pub id: String,
    pub label: String,
    pub ready: bool,
    pub detail: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(JsonSchema))]
#[cfg_attr(feature = "openapi", derive(ToSchema))]
#[cfg_attr(feature = "ts-export", derive(TS))]
#[cfg_attr(feature = "ts-export", ts(export))]
pub struct GatewayProductionReadiness {
    pub status: ProductionReadinessStatus,
    pub checks: Vec<ProductionReadinessCheck>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(JsonSchema))]
#[cfg_attr(feature = "openapi", derive(ToSchema))]
#[cfg_attr(feature = "ts-export", derive(TS))]
#[cfg_attr(feature = "ts-export", ts(export))]
pub struct CreateGatewayActivationResponse {
    pub route: GatewayRoute,
    pub agent_id: String,
    pub evaluation_profile: AgentEvaluationProfile,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts-export", ts(optional))]
    pub notification_rule: Option<NotificationRule>,
    pub alerts_deferred: bool,
    pub verification_session_id: String,
    pub data_handling_mode: DataHandlingMode,
    pub readiness: GatewayProductionReadiness,
}

#[cfg(test)]
mod tests {
    use super::{CreateGatewayRouteRequest, GatewayReliabilityMode};

    #[test]
    fn legacy_route_request_defaults_reliability_fields() {
        let route: CreateGatewayRouteRequest = serde_json::from_value(serde_json::json!({
            "display_name": "Primary",
            "provider_connection_id": "provider-1",
            "agent_id": "agent-1"
        }))
        .expect("legacy route JSON should remain valid");

        assert_eq!(route.reliability_mode, GatewayReliabilityMode::None);
        assert!(route.fallback_provider_connection_ids.is_empty());
    }

    #[test]
    fn reliability_mode_uses_snake_case_wire_values() {
        assert_eq!(
            serde_json::to_string(&GatewayReliabilityMode::Standard).unwrap(),
            "\"standard\""
        );
        assert_eq!(
            serde_json::from_str::<GatewayReliabilityMode>("\"none\"").unwrap(),
            GatewayReliabilityMode::None
        );
    }
}
