use std::sync::Arc;

use anyhow::Result;
use async_trait::async_trait;
use tl_storage::GatewayRepo;

use crate::gateway::GatewayStore;

pub struct PostgresGatewayAdapter(pub Arc<GatewayRepo>);

impl PostgresGatewayAdapter {
    pub fn new(repo: Arc<GatewayRepo>) -> Arc<Self> {
        Arc::new(Self(repo))
    }
}

#[async_trait]
impl GatewayStore for PostgresGatewayAdapter {
    async fn list_provider_connections(
        &self,
        workspace_id: &str,
    ) -> Result<Vec<tl_core::GatewayProviderConnection>, crate::gateway::GatewayStoreError> {
        self.0
            .list_provider_connections(workspace_id)
            .await
            .map_err(gateway_store_error)
    }

    async fn create_provider_connection(
        &self,
        input: crate::gateway::NewGatewayProviderConnection,
    ) -> Result<tl_core::GatewayProviderConnection, crate::gateway::GatewayStoreError> {
        self.0
            .create_provider_connection(tl_storage::models::NewGatewayProviderConnection {
                workspace_id: input.workspace_id,
                id: input.id,
                display_name: input.display_name,
                kind: crate::gateway::provider_kind_storage_text(input.kind).to_string(),
                base_url: input.base_url,
                default_model: input.default_model,
                encrypted_api_key: input.encrypted_api_key,
            })
            .await
            .map_err(gateway_store_error)
    }

    async fn update_provider_connection(
        &self,
        workspace_id: &str,
        id: &str,
        patch: crate::gateway::ProviderConnectionPatch,
    ) -> Result<tl_core::GatewayProviderConnection, crate::gateway::GatewayStoreError> {
        self.0
            .update_provider_connection(
                workspace_id,
                id,
                patch.display_name.as_deref(),
                patch.base_url.as_ref().map(|value| value.as_deref()),
                patch.default_model.as_deref(),
                patch.encrypted_api_key.as_deref(),
            )
            .await
            .map_err(gateway_store_error)
    }

    async fn get_provider_connection_secret(
        &self,
        workspace_id: &str,
        id: &str,
    ) -> Result<crate::gateway::ProviderConnectionSecret, crate::gateway::GatewayStoreError> {
        self.0
            .get_provider_connection_secret(workspace_id, id)
            .await
            .map(|secret| crate::gateway::ProviderConnectionSecret {
                connection: secret.connection,
                encrypted_api_key: secret.encrypted_api_key,
            })
            .map_err(gateway_store_error)
    }

    async fn delete_provider_connection(
        &self,
        workspace_id: &str,
        id: &str,
    ) -> Result<(), crate::gateway::GatewayStoreError> {
        self.0
            .delete_provider_connection(workspace_id, id)
            .await
            .map_err(gateway_store_error)
    }

    async fn list_gateway_routes(
        &self,
        workspace_id: &str,
    ) -> Result<Vec<tl_core::GatewayRoute>, crate::gateway::GatewayStoreError> {
        self.0
            .list_gateway_routes(workspace_id)
            .await
            .map_err(gateway_store_error)
    }

    async fn create_gateway_route(
        &self,
        input: crate::gateway::NewGatewayRoute,
    ) -> Result<tl_core::GatewayRoute, crate::gateway::GatewayStoreError> {
        let reliability_mode = match input.reliability_mode {
            tl_core::GatewayReliabilityMode::None => "none",
            tl_core::GatewayReliabilityMode::Standard => "standard",
        }
        .to_string();
        self.0
            .create_gateway_route(tl_storage::models::NewGatewayRoute {
                workspace_id: input.workspace_id,
                id: input.id,
                display_name: input.display_name,
                provider_connection_id: input.provider_connection_id,
                agent_id: input.agent_id,
                reliability_mode,
                fallback_provider_connection_id: input.fallback_provider_connection_id,
            })
            .await
            .map_err(gateway_store_error)
    }

    async fn update_gateway_route(
        &self,
        workspace_id: &str,
        id: &str,
        patch: crate::gateway::GatewayRoutePatch,
    ) -> Result<tl_core::GatewayRoute, crate::gateway::GatewayStoreError> {
        self.0
            .update_gateway_route(
                workspace_id,
                id,
                tl_storage::GatewayRoutePatch {
                    display_name: patch.display_name,
                    provider_connection_id: patch.provider_connection_id,
                    agent_id: patch.agent_id,
                    reliability_mode: patch.reliability_mode,
                    fallback_provider_connection_id: patch.fallback_provider_connection_id,
                },
            )
            .await
            .map_err(gateway_store_error)
    }

    async fn resolve_gateway_route(
        &self,
        workspace_id: &str,
        route_id: &str,
    ) -> Result<crate::gateway::ResolvedGatewayRoute, crate::gateway::GatewayStoreError> {
        self.0
            .resolve_gateway_route(workspace_id, route_id)
            .await
            .map(|resolved| crate::gateway::ResolvedGatewayRoute {
                route: resolved.route,
                provider_connection: resolved.provider_connection,
                encrypted_api_key: resolved.encrypted_api_key,
                fallback_provider_connection: resolved.fallback_provider_connection.map(
                    |fallback| crate::gateway::ProviderConnectionSecret {
                        connection: fallback.connection,
                        encrypted_api_key: fallback.encrypted_api_key,
                    },
                ),
            })
            .map_err(gateway_store_error)
    }
}

fn gateway_store_error(error: tl_storage::StorageError) -> crate::gateway::GatewayStoreError {
    match error {
        tl_storage::StorageError::NotFound => crate::gateway::GatewayStoreError::NotFound,
        tl_storage::StorageError::Conflict => crate::gateway::GatewayStoreError::Conflict(
            "provider connection is used by a gateway route".into(),
        ),
        other => crate::gateway::GatewayStoreError::Internal(other.to_string()),
    }
}
