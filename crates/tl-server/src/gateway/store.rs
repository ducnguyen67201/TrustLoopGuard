use async_trait::async_trait;
use tl_core::{
    EnforcementProfile, FailMode, GatewayInputAction, GatewayOutputAction,
    GatewayProviderConnection, GatewayProviderKind, GatewayRoute, ResponseMode, RetentionMode,
};

mod memory;

pub use memory::MemoryGatewayStore;

#[derive(Debug, thiserror::Error)]
pub enum GatewayStoreError {
    #[error("not found")]
    NotFound,
    #[error("internal: {0}")]
    Internal(String),
}

#[derive(Debug, Clone)]
pub struct NewGatewayProviderConnection {
    pub id: String,
    pub workspace_id: String,
    pub display_name: String,
    pub kind: GatewayProviderKind,
    pub base_url: Option<String>,
    pub default_model: String,
    pub encrypted_api_key: String,
}

#[derive(Debug, Clone, Default)]
pub struct ProviderConnectionPatch {
    pub display_name: Option<String>,
    pub base_url: Option<Option<String>>,
    pub default_model: Option<String>,
    pub encrypted_api_key: Option<String>,
}

#[derive(Debug, Clone)]
pub struct ProviderConnectionSecret {
    pub connection: GatewayProviderConnection,
    pub encrypted_api_key: String,
}

#[derive(Debug, Clone)]
pub struct NewEnforcementProfile {
    pub id: String,
    pub workspace_id: String,
    pub display_name: String,
    pub input_action: GatewayInputAction,
    pub output_action: GatewayOutputAction,
    pub fail_mode: FailMode,
    pub retention_mode: RetentionMode,
    pub response_mode: ResponseMode,
    pub fallback_message: String,
    pub max_regenerations: u32,
}

#[derive(Debug, Clone, Default)]
pub struct EnforcementProfilePatch {
    pub display_name: Option<String>,
    pub input_action: Option<GatewayInputAction>,
    pub output_action: Option<GatewayOutputAction>,
    pub fail_mode: Option<FailMode>,
    pub retention_mode: Option<RetentionMode>,
    pub response_mode: Option<ResponseMode>,
    pub fallback_message: Option<String>,
    pub max_regenerations: Option<u32>,
}

#[derive(Debug, Clone)]
pub struct NewGatewayRoute {
    pub id: String,
    pub workspace_id: String,
    pub display_name: String,
    pub provider_connection_id: String,
    pub agent_id: String,
    pub enforcement_profile_id: String,
}

#[derive(Debug, Clone, Default)]
pub struct GatewayRoutePatch {
    pub display_name: Option<String>,
    pub provider_connection_id: Option<String>,
    pub agent_id: Option<String>,
    pub enforcement_profile_id: Option<String>,
}

#[derive(Debug, Clone)]
pub struct ResolvedGatewayRoute {
    pub route: GatewayRoute,
    pub provider_connection: GatewayProviderConnection,
    pub enforcement_profile: EnforcementProfile,
    pub encrypted_api_key: String,
}

#[async_trait]
pub trait GatewayStore: Send + Sync {
    async fn list_provider_connections(
        &self,
        workspace_id: &str,
    ) -> Result<Vec<GatewayProviderConnection>, GatewayStoreError>;
    async fn create_provider_connection(
        &self,
        input: NewGatewayProviderConnection,
    ) -> Result<GatewayProviderConnection, GatewayStoreError>;
    async fn update_provider_connection(
        &self,
        workspace_id: &str,
        id: &str,
        patch: ProviderConnectionPatch,
    ) -> Result<GatewayProviderConnection, GatewayStoreError>;
    async fn get_provider_connection_secret(
        &self,
        workspace_id: &str,
        id: &str,
    ) -> Result<ProviderConnectionSecret, GatewayStoreError>;

    async fn list_enforcement_profiles(
        &self,
        workspace_id: &str,
    ) -> Result<Vec<EnforcementProfile>, GatewayStoreError>;
    async fn create_enforcement_profile(
        &self,
        input: NewEnforcementProfile,
    ) -> Result<EnforcementProfile, GatewayStoreError>;
    async fn update_enforcement_profile(
        &self,
        workspace_id: &str,
        id: &str,
        patch: EnforcementProfilePatch,
    ) -> Result<EnforcementProfile, GatewayStoreError>;

    async fn list_gateway_routes(
        &self,
        workspace_id: &str,
    ) -> Result<Vec<GatewayRoute>, GatewayStoreError>;
    async fn create_gateway_route(
        &self,
        input: NewGatewayRoute,
    ) -> Result<GatewayRoute, GatewayStoreError>;
    async fn update_gateway_route(
        &self,
        workspace_id: &str,
        id: &str,
        patch: GatewayRoutePatch,
    ) -> Result<GatewayRoute, GatewayStoreError>;
    async fn resolve_gateway_route(
        &self,
        workspace_id: &str,
        route_id: &str,
    ) -> Result<ResolvedGatewayRoute, GatewayStoreError>;
}
