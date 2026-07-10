use async_trait::async_trait;
use tl_core::{GatewayCredentialStatus, GatewayProviderConnection, GatewayRoute};

use super::{lock_error, MemoryGatewayRoute, MemoryGatewayStore, MemoryProviderConnection};
use crate::gateway::store::{
    GatewayRoutePatch, GatewayStore, GatewayStoreError, NewGatewayProviderConnection,
    NewGatewayRoute, ProviderConnectionPatch, ProviderConnectionSecret, ResolvedGatewayRoute,
};

#[async_trait]
impl GatewayStore for MemoryGatewayStore {
    async fn list_provider_connections(
        &self,
        workspace_id: &str,
    ) -> Result<Vec<GatewayProviderConnection>, GatewayStoreError> {
        let rows = self.provider_connections.read().map_err(lock_error)?;
        Ok(rows
            .iter()
            .filter(|row| row.workspace_id == workspace_id)
            .map(|row| row.connection.clone())
            .collect())
    }

    async fn create_provider_connection(
        &self,
        input: NewGatewayProviderConnection,
    ) -> Result<GatewayProviderConnection, GatewayStoreError> {
        let now = chrono::Utc::now().to_rfc3339();
        let connection = GatewayProviderConnection {
            id: input.id,
            display_name: input.display_name,
            kind: input.kind,
            base_url: input.base_url,
            default_model: input.default_model,
            credential_status: GatewayCredentialStatus::Configured,
            created_at: now.clone(),
            updated_at: now,
        };
        let mut rows = self.provider_connections.write().map_err(lock_error)?;
        rows.push(MemoryProviderConnection {
            workspace_id: input.workspace_id,
            connection: connection.clone(),
            encrypted_api_key: input.encrypted_api_key,
        });
        Ok(connection)
    }

    async fn update_provider_connection(
        &self,
        workspace_id: &str,
        id: &str,
        patch: ProviderConnectionPatch,
    ) -> Result<GatewayProviderConnection, GatewayStoreError> {
        let mut rows = self.provider_connections.write().map_err(lock_error)?;
        let row = rows
            .iter_mut()
            .find(|row| row.workspace_id == workspace_id && row.connection.id == id)
            .ok_or(GatewayStoreError::NotFound)?;

        if let Some(value) = patch.display_name {
            row.connection.display_name = value;
        }
        if let Some(value) = patch.base_url {
            row.connection.base_url = value;
        }
        if let Some(value) = patch.default_model {
            row.connection.default_model = value;
        }
        if let Some(value) = patch.encrypted_api_key {
            row.encrypted_api_key = value;
            row.connection.credential_status = GatewayCredentialStatus::Configured;
        }

        row.connection.updated_at = chrono::Utc::now().to_rfc3339();
        Ok(row.connection.clone())
    }

    async fn get_provider_connection_secret(
        &self,
        workspace_id: &str,
        id: &str,
    ) -> Result<ProviderConnectionSecret, GatewayStoreError> {
        let rows = self.provider_connections.read().map_err(lock_error)?;
        let row = rows
            .iter()
            .find(|row| row.workspace_id == workspace_id && row.connection.id == id)
            .ok_or(GatewayStoreError::NotFound)?;

        Ok(ProviderConnectionSecret {
            connection: row.connection.clone(),
            encrypted_api_key: row.encrypted_api_key.clone(),
        })
    }

    async fn list_gateway_routes(
        &self,
        workspace_id: &str,
    ) -> Result<Vec<GatewayRoute>, GatewayStoreError> {
        let rows = self.gateway_routes.read().map_err(lock_error)?;
        Ok(rows
            .iter()
            .filter(|row| row.workspace_id == workspace_id)
            .map(|row| row.route.clone())
            .collect())
    }

    async fn create_gateway_route(
        &self,
        input: NewGatewayRoute,
    ) -> Result<GatewayRoute, GatewayStoreError> {
        let now = chrono::Utc::now().to_rfc3339();
        let route = GatewayRoute {
            id: input.id,
            display_name: input.display_name,
            provider_connection_id: input.provider_connection_id,
            agent_id: input.agent_id,
            created_at: now.clone(),
            updated_at: now,
        };
        let mut rows = self.gateway_routes.write().map_err(lock_error)?;
        rows.push(MemoryGatewayRoute {
            workspace_id: input.workspace_id,
            route: route.clone(),
        });
        Ok(route)
    }

    async fn update_gateway_route(
        &self,
        workspace_id: &str,
        id: &str,
        patch: GatewayRoutePatch,
    ) -> Result<GatewayRoute, GatewayStoreError> {
        let mut rows = self.gateway_routes.write().map_err(lock_error)?;
        let row = rows
            .iter_mut()
            .find(|row| row.workspace_id == workspace_id && row.route.id == id)
            .ok_or(GatewayStoreError::NotFound)?;

        if let Some(value) = patch.display_name {
            row.route.display_name = value;
        }
        if let Some(value) = patch.provider_connection_id {
            row.route.provider_connection_id = value;
        }
        if let Some(value) = patch.agent_id {
            row.route.agent_id = value;
        }
        row.route.updated_at = chrono::Utc::now().to_rfc3339();
        Ok(row.route.clone())
    }

    async fn resolve_gateway_route(
        &self,
        workspace_id: &str,
        route_id: &str,
    ) -> Result<ResolvedGatewayRoute, GatewayStoreError> {
        let route = {
            let rows = self.gateway_routes.read().map_err(lock_error)?;
            rows.iter()
                .find(|row| row.workspace_id == workspace_id && row.route.id == route_id)
                .map(|row| row.route.clone())
                .ok_or(GatewayStoreError::NotFound)?
        };
        let provider = self
            .get_provider_connection_secret(workspace_id, &route.provider_connection_id)
            .await?;
        Ok(ResolvedGatewayRoute {
            route,
            provider_connection: provider.connection,
            encrypted_api_key: provider.encrypted_api_key,
        })
    }
}
